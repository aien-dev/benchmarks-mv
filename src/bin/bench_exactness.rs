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
    /// Stop the whole run after this many seconds; the report keeps what finished
    #[arg(long, default_value_t = 840)]
    deadline_s: u64,
    /// Concurrent widths for (c), comma-separated
    #[arg(long, default_value = "2,4")]
    widths: String,
    /// Write the JSON report here (rewritten after every request)
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
    deadline: Instant,
}

/// Everything the report holds; written after every request so a stopped run keeps its evidence.
#[derive(Default)]
struct Run {
    header: Value,
    obs: Vec<Observation>,
    pairs: Vec<PairResult>,
    errors: Vec<String>,
}

impl Run {
    fn save(&self, out: Option<&PathBuf>) {
        let Some(path) = out else { return };
        let mut report = self.header.clone();
        report["observations"] = json!(self.obs);
        report["pairs"] = json!(self.pairs);
        report["errors"] = json!(self.errors);
        let text = serde_json::to_string_pretty(&report).expect("report");
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("bench_exactness: write {}: {e}", path.display());
        }
    }
}

impl Ctx {
    /// One greedy request under `system`; retries 503 until the request timeout or the deadline.
    async fn run(
        &self,
        label: &str,
        system: &str,
        messages: Vec<Value>,
        max_tokens: u32,
        stream: bool,
    ) -> Result<Observation, String> {
        let mut msgs = vec![json!({"role": "system", "content": system})];
        msgs.extend(messages);
        self.run_raw(label, msgs, max_tokens, stream).await
    }

    fn record(&self, run: &mut Run, o: Observation) -> Observation {
        run.obs.push(o.clone());
        run.save(self.args.out.as_ref());
        o
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
        let deadline = (t0 + Duration::from_secs(self.args.request_timeout_s)).min(self.deadline);
        let mut busy = 0u32;
        let retry = Duration::from_millis(self.args.retry_ms);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!("{label}: run deadline reached after {busy} tries"));
            }
            let resp = self
                .client
                .post(&url)
                .timeout(left)
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
    /// next request starts cold. Not counted as evidence.
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

/// (b) resumed == fresh. The system prompt carries a per-run nonce, so no state left on the
/// server by an earlier request can be a prefix of the first prompt: it starts cold without a
/// flush request (each request can wait minutes for the shared chip).
async fn resume(ctx: &Ctx, run: &mut Run, nonce: u128) -> Result<(), String> {
    println!(
        "(b) resumed == fresh, long prompt ({} lines)",
        ctx.args.long_lines
    );
    let n = ctx.args.max_tokens;
    let system = format!("{SYSTEM} Session {nonce}.");
    let q = user(&format!(
        "{}\nList every station whose pressure is above 90 kPa, as a comma-separated list.",
        long_document(ctx.args.long_lines)
    ));
    let fresh = ctx
        .run("long/fresh", &system, vec![q.clone()], n, false)
        .await?;
    let fresh = ctx.record(run, fresh);
    let again = ctx
        .run("long/resumed", &system, vec![q.clone()], n, false)
        .await?;
    let again = ctx.record(run, again);
    run.pairs.push(compare(
        "resumed == fresh (same prompt)",
        &fresh,
        &again,
        fresh.cache.is_cold() && again.cache.is_reused(),
    ));
    run.save(ctx.args.out.as_ref());
    let reply = json!({"role": "assistant", "content": fresh.reply_text});
    let follow = user("Now list every station whose valve is closed and pressure is below 10 kPa.");
    let turn2 = vec![q, reply, follow];
    let t2_resumed = ctx
        .run("turn2/resumed", &system, turn2.clone(), n, false)
        .await?;
    let t2_resumed = ctx.record(run, t2_resumed);
    let flushed = ctx.flush().await?;
    ctx.record(run, flushed);
    let t2_fresh = ctx.run("turn2/fresh", &system, turn2, n, false).await?;
    let t2_fresh = ctx.record(run, t2_fresh);
    run.pairs.push(compare(
        "resumed == fresh (follow-up turn)",
        &t2_fresh,
        &t2_resumed,
        t2_fresh.cache.is_cold() && t2_resumed.cache.is_reused(),
    ));
    run.save(ctx.args.out.as_ref());
    Ok(())
}

/// (c) concurrent == solo. Each question differs after the shared system prompt, so the
/// server's state from one stream is never a prefix of another stream's prompt.
async fn concurrent(ctx: Arc<Ctx>, run: &mut Run, widths: &[usize]) -> Result<(), String> {
    println!("(c) concurrent == solo, N = {widths:?}, streamed");
    let n = ctx.args.max_tokens;
    let questions = [
        "Name three uses of copper in one sentence.",
        "Explain in one sentence why bridges have expansion joints.",
        "Give the boiling point of water at sea level in kelvin, with one sentence of context.",
        "In one sentence, what does a pressure relief valve do?",
    ];
    let max = widths.iter().copied().max().unwrap_or(0);
    if max > questions.len() {
        return Err(format!(
            "widths above {} are not supported",
            questions.len()
        ));
    }
    let mut solo = Vec::new();
    for (i, q) in questions.iter().take(max).enumerate() {
        let o = ctx
            .run(&format!("solo/{i}"), SYSTEM, vec![user(q)], n, true)
            .await?;
        solo.push(ctx.record(run, o));
    }
    for &width in widths {
        let barrier = Arc::new(Barrier::new(width));
        let mut tasks = Vec::new();
        for (i, q) in questions.iter().take(width).enumerate() {
            let (ctx, barrier, q) = (ctx.clone(), barrier.clone(), q.to_string());
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                ctx.run(&format!("n{width}/{i}"), SYSTEM, vec![user(&q)], n, true)
                    .await
            }));
        }
        let mut round = Vec::new();
        for t in tasks {
            let o = t.await.map_err(|e| format!("task: {e}"))??;
            round.push(ctx.record(run, o));
        }
        // Precondition: the requests really were in flight together, which the server proves
        // by answering 503 to the ones it could not take yet.
        let busy: u32 = round.iter().map(|o| o.busy_retries).sum();
        println!("  N={width}: {busy} answers were 503 (the single-request server served the streams one by one)");
        for (i, o) in round.iter().enumerate() {
            run.pairs.push(compare(
                &format!("concurrent == solo (N={width}, stream {i})"),
                &solo[i],
                o,
                busy > 0,
            ));
        }
        run.save(ctx.args.out.as_ref());
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let start = Instant::now();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let widths: Vec<usize> = match args
        .widths
        .split(',')
        .map(|w| w.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(w) if w.iter().all(|&x| x >= 2) => w,
        _ => {
            eprintln!("bench_exactness: --widths wants integers >= 2, e.g. 2,4");
            std::process::exit(2);
        }
    };
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
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
    let mut run = Run {
        header: json!({
            "schema": "aien-exactness/1",
            "idea_source": "TensorFold CONTRIBUTING.md 'Exactness and precision', docs/cache-benchmarks.md",
            "base": args.base,
            "model": model,
            "health": health,
            "nonce": nonce.to_string(),
            "drafted_eq_plain": "not tested: the server exposes no per-request draft switch",
        }),
        ..Run::default()
    };
    let only = args.only.clone();
    let ctx = Arc::new(Ctx {
        deadline: start + Duration::from_secs(args.deadline_s),
        args,
        client,
        model,
    });
    if only == "all" || only == "resume" {
        if let Err(e) = resume(&ctx, &mut run, nonce).await {
            run.errors.push(e);
        }
    }
    if only == "all" || only == "concurrent" {
        if let Err(e) = concurrent(ctx.clone(), &mut run, &widths).await {
            run.errors.push(e);
        }
    }
    run.save(ctx.args.out.as_ref());
    println!("results ({} s):", start.elapsed().as_secs());
    for p in &run.pairs {
        println!(
            "  {:?} {}: {} vs {}: {}",
            p.verdict, p.equality, p.reference, p.candidate, p.reason
        );
    }
    for e in &run.errors {
        println!("  ERROR {e}");
    }
    let failed = run.pairs.iter().any(|p| p.verdict == Verdict::Fail);
    std::process::exit(if failed {
        1
    } else if run.errors.is_empty() {
        0
    } else {
        2
    });
}
