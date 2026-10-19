//! Exactness contract client: runs the byte-equalities against a live aien-serve-http.
//!
//! Idea credit: TensorFold (CONTRIBUTING.md "Exactness and precision"). Greedy output at one
//! checkpoint and settings must be token-for-token equal:
//!   (a) drafted == plain: needs a per-request draft switch. aien-serve-http exposes none (its
//!       request type has no such field and unknown fields are ignored), so (a) is reported as
//!       not testable rather than faked.
//!   (b) resumed == fresh: a long prompt prefilled cold, then the same prompt resumed from the
//!       server's prefix state, and a follow-up turn resumed versus the same turn cold.
//!   (c) concurrent == solo: N=2 and N=4 requests sent at the same instant versus each one run
//!       alone. The server runs one generation at a time and answers 503 to the rest, so the
//!       streams are served one after another; what is tested is that admission, retry and the
//!       state left by another stream do not change a stream's tokens.
//!
//! Token ids come from `usage.aien_token_ids` (request field `aien_return_token_ids`). The cache
//! state of each request is read from the server log (`--server-log`), never inferred from the
//! request order; without the log, (b) pairs that match stay UNVERIFIED.
//!
//! The server takes the GPU lock itself; this client must not hold it.

use aien_benchmarks::exactness::{
    compare, reused_from_server_log, CacheState, Observation, PairResult, Verdict,
};
use clap::Parser;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

#[derive(Parser, Debug, Clone)]
#[command(name = "bench_exactness")]
#[command(about = "Exactness contract (TensorFold): resumed == fresh, concurrent == solo")]
struct Args {
    /// Server base URL
    #[arg(long, default_value = "http://127.0.0.1:18091")]
    base: String,
    /// Model id; default: the first id from /v1/models
    #[arg(long)]
    model: Option<String>,
    /// aien-serve-http log, read to observe each request's reused prompt tokens
    #[arg(long)]
    server_log: Option<PathBuf>,
    /// Which equalities to run: all, resume, concurrent
    #[arg(long, default_value = "all")]
    only: String,
    /// Greedy tokens per request
    #[arg(long, default_value_t = 32)]
    max_tokens: u32,
    /// Lines of the long document in (b); each line is roughly 25 tokens
    #[arg(long, default_value_t = 60)]
    long_lines: usize,
    /// Give up on one request after this many seconds (503 retries included)
    #[arg(long, default_value_t = 600)]
    request_timeout_s: u64,
    /// Wait between tries after a 503 (a 503 from the busy gate costs the server nothing)
    #[arg(long, default_value_t = 500)]
    retry_ms: u64,
    /// Write the JSON report here
    #[arg(long)]
    out: Option<PathBuf>,
}

const SYSTEM: &str = "You are a precise assistant. Answer briefly.";

fn user(text: &str) -> Value {
    json!({"role": "user", "content": text})
}

fn long_document(lines: usize) -> String {
    let mut s =
        String::from("Station log. Each line gives a station, its pressure and its valve state.\n");
    for k in 1..=lines {
        let pressure = (k * 37) % 101;
        let valve = if (k * 13) % 3 == 0 { "closed" } else { "open" };
        s.push_str(&format!(
            "Line {k}: station S{k:03} reads pressure {pressure} kPa, temperature {} C, valve {valve}.\n",
            20 + (k * 7) % 45
        ));
    }
    s
}

struct Ctx {
    args: Args,
    client: Client,
    model: String,
}

impl Ctx {
    /// One greedy request; retries 503 with backoff until the request timeout.
    async fn run(
        &self,
        label: &str,
        messages: Vec<Value>,
        max_tokens: u32,
        stream: bool,
    ) -> Result<Observation, String> {
        let mut msgs = vec![json!({"role": "system", "content": SYSTEM})];
        msgs.extend(messages);
        self.run_raw(label, msgs, max_tokens, stream).await
    }

    async fn run_raw(
        &self,
        label: &str,
        messages: Vec<Value>,
        max_tokens: u32,
        stream: bool,
    ) -> Result<Observation, String> {
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": 0.0,
            "seed": 1,
            "stream": stream,
            "aien_return_token_ids": true,
        });
        if stream {
            body["stream_options"] = json!({"include_usage": true});
        }
        let url = format!("{}/v1/chat/completions", self.args.base);
        let t0 = Instant::now();
        let deadline = t0 + Duration::from_secs(self.args.request_timeout_s);
        let mut busy = 0u32;
        let retry = Duration::from_millis(self.args.retry_ms);
        loop {
            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("{label}: send: {e}"))?;
            let status = resp.status();
            let text = resp
                .text()
                .await
                .map_err(|e| format!("{label}: read: {e}"))?;
            if status == StatusCode::SERVICE_UNAVAILABLE {
                busy += 1;
                if Instant::now() + retry > deadline {
                    return Err(format!("{label}: still 503 after {busy} tries"));
                }
                tokio::time::sleep(retry).await;
                continue;
            }
            if !status.is_success() {
                return Err(format!(
                    "{label}: HTTP {status}: {}",
                    text.chars().take(300).collect::<String>()
                ));
            }
            let mut o = if stream {
                parse_stream(label, &text)?
            } else {
                parse_json(label, &text)?
            };
            o.busy_retries = busy;
            o.wall_ms = t0.elapsed().as_millis();
            o.cache = self.observe_cache(&o);
            println!(
                "  {label:<22} prompt {:>5} tok  reply {:>3} tok  cache {:<22} 503s {busy:<3} token_sha256 {}",
                o.prompt_tokens.map_or("?".into(), |n| n.to_string()),
                o.token_count.map_or("?".into(), |n| n.to_string()),
                format!("{:?}", o.cache),
                o.token_sha256.as_deref().unwrap_or("MISSING"),
            );
            return Ok(o);
        }
    }

    fn observe_cache(&self, o: &Observation) -> CacheState {
        let (Some(path), Some(id)) = (&self.args.server_log, &o.id) else {
            return CacheState::Unverified;
        };
        let reused = std::fs::read_to_string(path)
            .ok()
            .and_then(|log| reused_from_server_log(&log, id));
        CacheState::observe(reused, o.prompt_tokens)
    }

    /// A tiny request with a different system prompt: replaces the server's prefix state so the
    /// next request with our system prompt starts cold.
    async fn flush(&self) -> Result<Observation, String> {
        let msgs = vec![
            json!({"role": "system", "content": "Scratch session."}),
            user("Say ok."),
        ];
        self.run_raw("flush", msgs, 1, false).await
    }
}

fn usage_fields(
    label: &str,
    id: Option<String>,
    usage: Option<&Value>,
    finish: Option<String>,
    text: &str,
) -> Observation {
    let get = |k: &str| {
        usage
            .and_then(|u| u.get(k))
            .and_then(Value::as_u64)
            .map(|n| n as usize)
    };
    let ids = usage
        .and_then(|u| u.get("aien_token_ids"))
        .and_then(Value::as_array)
        .and_then(|a| {
            a.iter()
                .map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok()))
                .collect::<Option<Vec<u32>>>()
        });
    Observation::new(
        label,
        id,
        get("prompt_tokens"),
        get("completion_tokens"),
        finish,
        ids,
        text,
    )
}

fn parse_json(label: &str, text: &str) -> Result<Observation, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("{label}: bad JSON: {e}"))?;
    let choice = &v["choices"][0];
    let content = choice["message"]["content"].as_str().unwrap_or("");
    Ok(usage_fields(
        label,
        v["id"].as_str().map(str::to_string),
        v.get("usage"),
        choice["finish_reason"].as_str().map(str::to_string),
        content,
    ))
}

fn parse_stream(label: &str, text: &str) -> Result<Observation, String> {
    let (mut id, mut usage, mut finish, mut content, mut done) =
        (None, None, None, String::new(), false);
    for line in text.lines() {
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data == "[DONE]" {
            done = true;
            continue;
        }
        let v: Value =
            serde_json::from_str(data).map_err(|e| format!("{label}: bad SSE JSON: {e}"))?;
        if let Some(err) = v.get("error") {
            return Err(format!("{label}: server error in stream: {err}"));
        }
        if id.is_none() {
            id = v["id"].as_str().map(str::to_string);
        }
        if let Some(c) = v["choices"][0]["delta"]["content"].as_str() {
            content.push_str(c);
        }
        if let Some(f) = v["choices"][0]["finish_reason"].as_str() {
            finish = Some(f.to_string());
        }
        if v.get("usage").is_some() {
            usage = v.get("usage").cloned();
        }
    }
    if !done {
        return Err(format!("{label}: stream ended without [DONE]"));
    }
    Ok(usage_fields(label, id, usage.as_ref(), finish, &content))
}

/// (b) resumed == fresh.
async fn resume(
    ctx: &Ctx,
    obs: &mut Vec<Observation>,
    pairs: &mut Vec<PairResult>,
) -> Result<(), String> {
    println!(
        "(b) resumed == fresh, long prompt ({} lines)",
        ctx.args.long_lines
    );
    let n = ctx.args.max_tokens;
    let q = user(&format!(
        "{}\nList every station whose pressure is above 90 kPa, as a comma-separated list.",
        long_document(ctx.args.long_lines)
    ));
    obs.push(ctx.flush().await?);
    let fresh = ctx.run("long/fresh", vec![q.clone()], n, false).await?;
    let again = ctx.run("long/resumed", vec![q.clone()], n, false).await?;
    let reply = json!({"role": "assistant", "content": fresh.reply_text});
    let follow = user("Now list every station whose valve is closed and pressure is below 10 kPa.");
    let turn2 = vec![q.clone(), reply.clone(), follow.clone()];
    let t2_resumed = ctx.run("turn2/resumed", turn2.clone(), n, false).await?;
    obs.push(ctx.flush().await?);
    let t2_fresh = ctx.run("turn2/fresh", turn2, n, false).await?;
    pairs.push(compare(
        "resumed == fresh (same prompt)",
        &fresh,
        &again,
        fresh.cache.is_cold() && again.cache.is_reused(),
    ));
    pairs.push(compare(
        "resumed == fresh (follow-up turn)",
        &t2_fresh,
        &t2_resumed,
        t2_fresh.cache.is_cold() && t2_resumed.cache.is_reused(),
    ));
    obs.extend([fresh, again, t2_resumed, t2_fresh]);
    Ok(())
}

/// (c) concurrent == solo.
async fn concurrent(
    ctx: Arc<Ctx>,
    obs: &mut Vec<Observation>,
    pairs: &mut Vec<PairResult>,
) -> Result<(), String> {
    println!("(c) concurrent == solo, N = 2 and 4, streamed");
    let n = ctx.args.max_tokens;
    let questions = [
        "Name three uses of copper in one sentence.",
        "Explain in one sentence why bridges have expansion joints.",
        "Give the boiling point of water at sea level in kelvin, with one sentence of context.",
        "In one sentence, what does a pressure relief valve do?",
    ];
    let mut solo = Vec::new();
    for (i, q) in questions.iter().enumerate() {
        obs.push(ctx.flush().await?);
        solo.push(
            ctx.run(&format!("solo/{i}"), vec![user(q)], n, true)
                .await?,
        );
    }
    for width in [2usize, 4] {
        let barrier = Arc::new(Barrier::new(width));
        let mut tasks = Vec::new();
        for (i, q) in questions.iter().take(width).enumerate() {
            let (ctx, barrier, q) = (ctx.clone(), barrier.clone(), q.to_string());
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                ctx.run(&format!("n{width}/{i}"), vec![user(&q)], n, true)
                    .await
            }));
        }
        let mut round = Vec::new();
        for t in tasks {
            round.push(t.await.map_err(|e| format!("task: {e}"))??);
        }
        // Precondition: the requests really were in flight together, which the server proves
        // by answering 503 to the ones it could not take yet.
        let busy: u32 = round.iter().map(|o| o.busy_retries).sum();
        println!("  N={width}: {busy} answers were 503 (the single-request server served the streams one by one)");
        for (i, o) in round.into_iter().enumerate() {
            pairs.push(compare(
                &format!("concurrent == solo (N={width}, stream {i})"),
                &solo[i],
                &o,
                busy > 0,
            ));
            obs.push(o);
        }
    }
    obs.extend(solo);
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = Client::builder()
        .timeout(Duration::from_secs(args.request_timeout_s))
        .build()
        .expect("http client");
    let models: Value = match client.get(format!("{}/v1/models", args.base)).send().await {
        Ok(r) => r.json().await.unwrap_or(Value::Null),
        Err(e) => {
            eprintln!("bench_exactness: /v1/models: {e}");
            std::process::exit(2);
        }
    };
    let model = args
        .model
        .clone()
        .or_else(|| models["data"][0]["id"].as_str().map(str::to_string))
        .unwrap_or_else(|| {
            eprintln!("bench_exactness: no model id from /v1/models");
            std::process::exit(2);
        });
    let health: Value = match client.get(format!("{}/health", args.base)).send().await {
        Ok(r) => r.json().await.unwrap_or(Value::Null),
        Err(_) => Value::Null,
    };
    println!(
        "bench_exactness: model {model}, engine {}",
        health["engine"].as_str().unwrap_or("?")
    );
    println!("(a) drafted == plain: NOT TESTED, the server exposes no per-request draft switch");
    let only = args.only.clone();
    let ctx = Arc::new(Ctx {
        args,
        client,
        model: model.clone(),
    });
    let (mut obs, mut pairs) = (Vec::new(), Vec::new());
    let mut errors = Vec::new();
    if only == "all" || only == "resume" {
        if let Err(e) = resume(&ctx, &mut obs, &mut pairs).await {
            errors.push(e);
        }
    }
    if only == "all" || only == "concurrent" {
        if let Err(e) = concurrent(ctx.clone(), &mut obs, &mut pairs).await {
            errors.push(e);
        }
    }
    println!("results:");
    for p in &pairs {
        println!(
            "  {:?} {}: {} vs {}: {}",
            p.verdict, p.equality, p.reference, p.candidate, p.reason
        );
    }
    for e in &errors {
        println!("  ERROR {e}");
    }
    if let Some(path) = &ctx.args.out {
        let report = json!({
            "schema": "aien-exactness/1",
            "idea_source": "TensorFold CONTRIBUTING.md 'Exactness and precision', docs/cache-benchmarks.md",
            "base": ctx.args.base,
            "model": model,
            "health": health,
            "drafted_eq_plain": "not tested: the server exposes no per-request draft switch",
            "observations": obs,
            "pairs": pairs,
            "errors": errors,
        });
        if let Err(e) = std::fs::write(path, serde_json::to_string_pretty(&report).expect("report"))
        {
            eprintln!("bench_exactness: write {}: {e}", path.display());
        }
    }
    let failed = pairs.iter().any(|p| p.verdict == Verdict::Fail);
    std::process::exit(if failed {
        1
    } else if errors.is_empty() {
        0
    } else {
        2
    });
}
