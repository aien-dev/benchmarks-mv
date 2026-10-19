//! ABBA alternation for a baseline and a candidate streaming endpoint.
//!
//! Idea credit: TensorFold CONTRIBUTING.md "Performance receipts" (baseline, candidate,
//! candidate, baseline order; geometric mean of per-process medians; keep the slower cells;
//! never quote a speed before checking that run's tokens). No code was copied.
//!
//! Rules:
//! - Every request is greedy (`temperature` 0), serial (one in flight) and streamed.
//! - Before any cell, each prompt runs once on the baseline: that serial greedy reply is the
//!   reference. Every measured reply of either arm must hash equal to the reference for its
//!   prompt, or that arm's speed is printed as `UNVERIFIED tokens` and no delta is printed.
//! - One cell is one pass over the prompt set on one arm. Cells run in the order
//!   B C C B per round. A cell's figure is the median of its per-request decode rates; an
//!   arm's figure is the geometric mean of its cell medians. Every cell is kept and the
//!   slowest cell of each arm is printed beside the mean.
//! - Decode rate of one request: `(usage.completion_tokens - 1)` divided by the time from the
//!   first to the last content event. It excludes queueing, the server's GPU lock wait and
//!   prefill, which are kept separately as TTFT.
//! - HTTP 503 (the server is busy with someone else's request) is retried with backoff and is
//!   never a measurement.

use super::llm_stream::{
    self, distribution, measure_one, sha256_hex, Outcome, RequestSample, StreamConfig,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub const RECEIPT_SCHEMA: &str = "aien-abba-receipt/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arm {
    Baseline,
    Candidate,
}

impl Arm {
    pub fn as_str(self) -> &'static str {
        match self {
            Arm::Baseline => "baseline",
            Arm::Candidate => "candidate",
        }
    }
}

/// Cell order: baseline, candidate, candidate, baseline, once per round. A linear drift
/// (warming, throttling, load arriving) adds the same to both arms over one round.
pub fn abba_order(rounds: u32) -> Vec<Arm> {
    (0..rounds)
        .flat_map(|_| [Arm::Baseline, Arm::Candidate, Arm::Candidate, Arm::Baseline])
        .collect()
}

/// Median of the finite values (mean of the two middle values for an even count).
pub fn median(values: &[f64]) -> Option<f64> {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

/// Geometric mean of strictly positive finite values; None if any value is not.
pub fn geomean(values: &[f64]) -> Option<f64> {
    if values.is_empty() || values.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return None;
    }
    Some((values.iter().map(|x| x.ln()).sum::<f64>() / values.len() as f64).exp())
}

/// Decode tokens per second of one complete request (see module docs).
pub fn decode_rate(s: &RequestSample) -> Option<f64> {
    if !s.is_complete() {
        return None;
    }
    let tokens = s.usage_completion_tokens?;
    let window_ms = s.last_content_ms? - s.first_content_ms?;
    (tokens >= 2 && window_ms > 0.0).then(|| (tokens - 1) as f64 / (window_ms / 1000.0))
}

/// Settings of one ABBA comparison.
#[derive(Debug, Clone)]
pub struct AbbaConfig {
    /// Endpoint, model and labels of each arm; prompt, temperature and request counts are
    /// overridden per request.
    pub baseline: StreamConfig,
    pub candidate: StreamConfig,
    pub prompts: Vec<String>,
    pub rounds: u32,
    /// Discarded serial passes over the prompt set on the candidate before the first cell
    /// (the baseline's reference pass already warms the baseline).
    pub candidate_warmup_passes: u32,
    pub busy_retries: u32,
    pub busy_backoff_max: Duration,
    /// Stop before starting a request once this much time has passed.
    pub deadline: Duration,
}

impl AbbaConfig {
    /// The exact request one arm sends for one prompt.
    pub fn request(&self, arm: Arm, prompt: &str) -> StreamConfig {
        let base = match arm {
            Arm::Baseline => &self.baseline,
            Arm::Candidate => &self.candidate,
        };
        StreamConfig {
            prompt: prompt.to_string(),
            temperature: Some(0.0),
            requests: 1,
            concurrency: 1,
            warmup: 0,
            request_usage: true,
            ..base.clone()
        }
    }
}

/// One request as measured, with its busy retries.
#[derive(Debug, Clone)]
pub struct Measured {
    pub prompt_index: usize,
    pub sample: RequestSample,
    pub busy_retries: u32,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub index: usize,
    pub arm: Arm,
    pub requests: Vec<Measured>,
}

impl Cell {
    pub fn rates(&self) -> Vec<f64> {
        self.requests
            .iter()
            .filter_map(|m| decode_rate(&m.sample))
            .collect()
    }

    pub fn median_rate(&self) -> Option<f64> {
        median(&self.rates())
    }

    /// Prompts whose reply is missing or differs from the reference.
    pub fn token_mismatches(&self, reference: &[Option<String>]) -> Vec<usize> {
        self.requests
            .iter()
            .filter(|m| {
                let want = reference.get(m.prompt_index).cloned().flatten();
                want.is_none() || m.sample.output_sha256 != want
            })
            .map(|m| m.prompt_index)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct AbbaRun {
    pub reference: Vec<Measured>,
    pub warmup: Vec<Measured>,
    pub cells: Vec<Cell>,
    /// Set when the run stopped early (deadline or a request that kept failing).
    pub stopped: Option<String>,
    pub wall: Duration,
}

impl AbbaRun {
    pub fn reference_hashes(&self) -> Vec<Option<String>> {
        self.reference
            .iter()
            .map(|m| m.sample.output_sha256.clone())
            .collect()
    }
}

/// One request, retried with doubling backoff while the server answers 503 busy.
pub fn measure_retrying(
    cfg: &StreamConfig,
    index: u32,
    retries: u32,
    backoff_max: Duration,
    measure: &mut dyn FnMut(&StreamConfig, u32) -> RequestSample,
) -> (RequestSample, u32) {
    let mut wait = Duration::from_millis(500);
    let mut used = 0;
    loop {
        let s = measure(cfg, index);
        let busy = s.outcome == Outcome::HttpStatus && s.http_status == Some(503);
        if !busy || used >= retries {
            return (s, used);
        }
        used += 1;
        std::thread::sleep(wait);
        wait = (wait * 2).min(backoff_max);
    }
}

/// Run the reference pass, the candidate warmup and the ABBA cells with `measure` (the
/// network in production, a stub in tests).
pub fn run_abba_with(
    cfg: &AbbaConfig,
    measure: &mut dyn FnMut(&StreamConfig, u32) -> RequestSample,
) -> AbbaRun {
    let t0 = Instant::now();
    let mut next_index = 0u32;
    let mut stopped = None;
    let mut pass = |arm: Arm, stopped: &mut Option<String>| -> Vec<Measured> {
        let mut out = Vec::new();
        for (i, p) in cfg.prompts.iter().enumerate() {
            if stopped.is_some() {
                break;
            }
            if t0.elapsed() >= cfg.deadline {
                *stopped = Some(format!("deadline {:?} reached", cfg.deadline));
                break;
            }
            let (sample, busy_retries) = measure_retrying(
                &cfg.request(arm, p),
                next_index,
                cfg.busy_retries,
                cfg.busy_backoff_max,
                measure,
            );
            next_index += 1;
            if !sample.is_complete() {
                *stopped = Some(format!(
                    "{} request for prompt {} failed: {} {}",
                    arm.as_str(),
                    i,
                    sample.outcome.as_str(),
                    sample.detail.clone().unwrap_or_default()
                ));
            }
            out.push(Measured {
                prompt_index: i,
                sample,
                busy_retries,
            });
        }
        out
    };
    let reference = pass(Arm::Baseline, &mut stopped);
    let mut warmup = Vec::new();
    for _ in 0..cfg.candidate_warmup_passes {
        warmup.extend(pass(Arm::Candidate, &mut stopped));
    }
    let mut cells = Vec::new();
    for (index, arm) in abba_order(cfg.rounds).into_iter().enumerate() {
        if stopped.is_some() {
            break;
        }
        let requests = pass(arm, &mut stopped);
        cells.push(Cell {
            index,
            arm,
            requests,
        });
    }
    AbbaRun {
        reference,
        warmup,
        cells,
        stopped,
        wall: t0.elapsed(),
    }
}

pub fn run_abba(cfg: &AbbaConfig) -> AbbaRun {
    run_abba_with(cfg, &mut |c, i| measure_one(c, i))
}

/// The verdict for one arm.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmSummary {
    pub arm: Arm,
    pub cell_medians: Vec<Option<f64>>,
    /// Geometric mean of the cell medians; None when any cell has no rate.
    pub geomean: Option<f64>,
    /// Slowest cell median (kept, never dropped).
    pub slowest: Option<f64>,
    pub fastest: Option<f64>,
    /// Empty when every reply of every cell matched the reference.
    pub token_mismatches: Vec<(usize, usize)>,
    pub cells_expected: usize,
}

impl ArmSummary {
    pub fn tokens_verified(&self) -> bool {
        self.token_mismatches.is_empty()
            && self.cell_medians.len() == self.cells_expected
            && self.cells_expected > 0
    }

    /// Fastest over slowest cell, minus one: the within-arm spread of this run.
    pub fn spread(&self) -> Option<f64> {
        Some(self.fastest? / self.slowest? - 1.0)
    }
}

pub fn summarize_arm(run: &AbbaRun, cfg: &AbbaConfig, arm: Arm) -> ArmSummary {
    let reference = run.reference_hashes();
    let cells: Vec<&Cell> = run.cells.iter().filter(|c| c.arm == arm).collect();
    let cell_medians: Vec<Option<f64>> = cells.iter().map(|c| c.median_rate()).collect();
    let complete: Option<Vec<f64>> = cell_medians.iter().copied().collect();
    let mut token_mismatches = Vec::new();
    for c in &cells {
        for p in c.token_mismatches(&reference) {
            token_mismatches.push((c.index, p));
        }
        if c.requests.len() < cfg.prompts.len() {
            token_mismatches.push((c.index, c.requests.len()));
        }
    }
    let finite: Vec<f64> = cell_medians.iter().flatten().copied().collect();
    ArmSummary {
        arm,
        geomean: complete.as_deref().and_then(geomean),
        slowest: finite.iter().copied().reduce(f64::min),
        fastest: finite.iter().copied().reduce(f64::max),
        cell_medians,
        token_mismatches,
        cells_expected: 2 * cfg.rounds as usize,
    }
}

/// Candidate over baseline, minus one; only when both arms' tokens were verified.
pub fn delta(base: &ArmSummary, cand: &ArmSummary) -> Option<f64> {
    if !base.tokens_verified() || !cand.tokens_verified() {
        return None;
    }
    Some(cand.geomean? / base.geomean? - 1.0)
}

/// What one arm's speed line says: a figure only when its tokens were verified.
pub fn arm_line(s: &ArmSummary) -> String {
    if !s.tokens_verified() {
        return format!(
            "{}: UNVERIFIED tokens ({} cell/prompt mismatch(es) or missing cells; no tok/s is quoted)",
            s.arm.as_str(),
            s.token_mismatches.len().max(1)
        );
    }
    let cells: Vec<String> = s
        .cell_medians
        .iter()
        .map(|m| m.map_or("-".to_string(), |v| format!("{:.3}", v)))
        .collect();
    format!(
        "{}: decode tok/s geomean of cell medians {} | slowest cell {} | cells [{}] | spread {}",
        s.arm.as_str(),
        s.geomean.map_or("-".to_string(), |v| format!("{:.3}", v)),
        s.slowest.map_or("-".to_string(), |v| format!("{:.3}", v)),
        cells.join(", "),
        s.spread()
            .map_or("-".to_string(), |v| format!("{:.2}%", v * 100.0))
    )
}

/// Facts about the box and the server that a receipt must name. Operator-supplied facts
/// that were not given are written as "not_recorded", never guessed.
#[derive(Debug, Clone, Default)]
pub struct ArmIdentity {
    pub engine_commit: Option<String>,
    pub checkpoint_revision: Option<String>,
    /// Running server process to read the binary hash and loaded GPU libraries from.
    pub server_pid: Option<u32>,
}

fn or_not_recorded(v: &Option<String>) -> Value {
    json!(v.clone().unwrap_or_else(|| "not_recorded".to_string()))
}

/// SHA-256 of the server binary and the GPU runtime libraries it has mapped, read from
/// /proc (no tool is run).
pub fn server_identity(pid: u32) -> Value {
    let exe = std::fs::read_link(format!("/proc/{}/exe", pid))
        .map(|p| p.display().to_string())
        .ok();
    let exe_sha256 = std::fs::read(format!("/proc/{}/exe", pid))
        .ok()
        .map(|b| sha256_hex(&b));
    let mut libs: Vec<String> = std::fs::read_to_string(format!("/proc/{}/maps", pid))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_whitespace().nth(5).map(str::to_string))
        .filter(|p| {
            let name = p.rsplit('/').next().unwrap_or("");
            name.contains(".so")
                && [
                    "cuda",
                    "cublas",
                    "nvidia",
                    "nvrtc",
                    "nccl",
                    "cudnn",
                    "nvJitLink",
                ]
                .iter()
                .any(|k| name.contains(k))
        })
        .collect();
    libs.sort();
    libs.dedup();
    let libs: Vec<Value> = libs
        .iter()
        .map(|p| {
            let real = std::fs::canonicalize(p)
                .map(|r| r.display().to_string())
                .unwrap_or_else(|_| p.clone());
            json!({
                "path": p,
                "resolved": real,
                "sha256": std::fs::read(p).ok().map(|b| sha256_hex(&b)),
            })
        })
        .collect();
    json!({
        "pid": pid,
        "exe": exe,
        "exe_sha256": exe_sha256,
        "runtime_libraries": libs,
    })
}

/// First line of /proc/driver/nvidia/version, or "not_recorded".
pub fn driver_version() -> String {
    std::fs::read_to_string("/proc/driver/nvidia/version")
        .ok()
        .and_then(|t| t.lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_else(|| "not_recorded".to_string())
}

fn measured_json(m: &Measured, reference: &[Option<String>]) -> Value {
    let want = reference.get(m.prompt_index).cloned().flatten();
    json!({
        "prompt_index": m.prompt_index,
        "outcome": m.sample.outcome.as_str(),
        "busy_retries": m.busy_retries,
        "completion_tokens": m.sample.usage_completion_tokens,
        "ttft_ms": m.sample.first_content_ms,
        "e2e_ms": m.sample.completion_ms,
        "decode_tok_s": decode_rate(&m.sample),
        "output_sha256": m.sample.output_sha256,
        "matches_reference": want.is_some() && m.sample.output_sha256 == want,
    })
}

/// The receipt: every fact TensorFold's receipts name (commit, GPU, driver, checkpoint
/// revision, runtime libraries, request parameters) plus every cell.
#[allow(clippy::too_many_arguments)]
pub fn build_receipt(
    cfg: &AbbaConfig,
    run: &AbbaRun,
    source: &llm_stream::SourceIdentity,
    ids: (&ArmIdentity, &ArmIdentity),
    labels: (&str, &str),
) -> Value {
    let hw = llm_stream::hardware_identity();
    let build = llm_stream::build_identity();
    let reference = run.reference_hashes();
    let base = summarize_arm(run, cfg, Arm::Baseline);
    let cand = summarize_arm(run, cfg, Arm::Candidate);
    let arm_json = |arm: Arm, label: &str, id: &ArmIdentity, s: &ArmSummary| {
        let c = match arm {
            Arm::Baseline => &cfg.baseline,
            Arm::Candidate => &cfg.candidate,
        };
        json!({
            "label": label,
            "endpoint": c.endpoint(),
            "model": c.model,
            "engine": c.engine,
            "precision": c.precision.clone().unwrap_or_else(|| "unlabeled".to_string()),
            "engine_commit": or_not_recorded(&id.engine_commit),
            "checkpoint_revision": or_not_recorded(&id.checkpoint_revision),
            "server": id.server_pid.map_or(json!("not_recorded"), server_identity),
            "tokens_verified": s.tokens_verified(),
            "token_mismatches": s.token_mismatches.iter().map(|(c, p)| json!({"cell": c, "prompt_index": p})).collect::<Vec<_>>(),
            "decode_tok_s_geomean": if s.tokens_verified() { json!(s.geomean) } else { json!("UNVERIFIED tokens") },
            "decode_tok_s_slowest_cell": if s.tokens_verified() { json!(s.slowest) } else { json!("UNVERIFIED tokens") },
            "cell_medians": s.cell_medians,
            "within_arm_spread": s.spread(),
        })
    };
    let all_rates: Vec<f64> = run.cells.iter().flat_map(|c| c.rates()).collect();
    json!({
        "schema": RECEIPT_SCHEMA,
        "generated_utc": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "harness": {"repo": llm_stream::REPO, "commit": source.commit, "dirty": source.dirty, "profile": build.profile, "rustc": build.rustc},
        "hardware": {"host": hw.host, "cpu": hw.cpu, "gpu": hw.gpu, "driver": driver_version()},
        "request": {
            "path": cfg.baseline.path,
            "temperature": 0.0,
            "max_tokens": cfg.baseline.max_tokens,
            "stream": true,
            "stream_options.include_usage": true,
            "concurrency": 1,
            "prompts": cfg.prompts.iter().map(|p| json!({"bytes": p.len(), "sha256": sha256_hex(p.as_bytes()), "text": p})).collect::<Vec<_>>(),
        },
        "protocol": {
            "order": abba_order(cfg.rounds).iter().map(|a| a.as_str()).collect::<Vec<_>>(),
            "rounds": cfg.rounds,
            "candidate_warmup_passes": cfg.candidate_warmup_passes,
            "cell": "one serial pass over the prompt set on one arm",
            "cell_figure": "median of per-request decode tok/s: (usage.completion_tokens - 1) / (last content event - first content event)",
            "arm_figure": "geometric mean of the arm's cell medians; every cell kept; slowest cell reported",
            "token_check": "reply text SHA-256 of every measured request equals the baseline's serial greedy reference reply for the same prompt",
            "busy": format!("HTTP 503 retried up to {} times with backoff doubling from 0.5 s to {:?}; retries are not measurements", cfg.busy_retries, cfg.busy_backoff_max),
        },
        "baseline": arm_json(Arm::Baseline, labels.0, ids.0, &base),
        "candidate": arm_json(Arm::Candidate, labels.1, ids.1, &cand),
        "candidate_over_baseline": delta(&base, &cand),
        "pooled_decode_tok_s": {"requests": all_rates.len(), "p50": distribution(&all_rates).p50},
        "stopped_early": run.stopped,
        "wall_s": run.wall.as_secs_f64(),
        "reference": run.reference.iter().map(|m| measured_json(m, &reference)).collect::<Vec<_>>(),
        "candidate_warmup": run.warmup.iter().map(|m| measured_json(m, &reference)).collect::<Vec<_>>(),
        "cells": run.cells.iter().map(|c| json!({
            "index": c.index,
            "arm": c.arm.as_str(),
            "median_decode_tok_s": c.median_rate(),
            "requests": c.requests.iter().map(|m| measured_json(m, &reference)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "does_not_show": [
            "prefill or time-to-first-token speed (TTFT is recorded per request but includes the server's GPU lock wait)",
            "behaviour under concurrency (every request is serial)",
            "a difference between two machines (both arms ran on this box)",
            "token-id equality: the hash covers the streamed reply text, not token ids",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(tokens: u64, first: f64, last: f64, hash: &str) -> RequestSample {
        let mut s = complete_sample();
        s.usage_completion_tokens = Some(tokens);
        s.first_content_ms = Some(first);
        s.last_content_ms = Some(last);
        s.output_sha256 = Some(hash.to_string());
        s
    }

    fn complete_sample() -> RequestSample {
        let start = Instant::now();
        let mut acc = llm_stream::StreamAccumulator::new(0, start);
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\ndata: [DONE]\n\n";
        let raw = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        acc.feed(raw.as_bytes(), start);
        let s = acc.finish();
        assert!(s.is_complete());
        s
    }

    #[test]
    fn order_is_baseline_candidate_candidate_baseline() {
        use Arm::*;
        assert_eq!(
            abba_order(1),
            vec![Baseline, Candidate, Candidate, Baseline]
        );
        assert_eq!(abba_order(2).len(), 8);
        assert_eq!(abba_order(2)[4], Baseline);
    }

    #[test]
    fn median_and_geomean() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&[]), None);
        let g = geomean(&[2.0, 8.0]).unwrap();
        assert!((g - 4.0).abs() < 1e-12);
        assert_eq!(geomean(&[2.0, 0.0]), None);
        assert_eq!(geomean(&[]), None);
    }

    #[test]
    fn decode_rate_excludes_first_token_and_wait() {
        // 11 tokens, first content at 5000 ms (lock wait + prefill), last at 6000 ms:
        // 10 decode steps in 1 s.
        let s = sample(11, 5000.0, 6000.0, "h");
        assert_eq!(decode_rate(&s), Some(10.0));
        assert_eq!(decode_rate(&sample(1, 0.0, 0.0, "h")), None);
    }

    /// Stub endpoint: reply hash per prompt, a fixed decode rate per arm (port), and one
    /// 503 before the first request.
    fn stub(
        rate_by_port: fn(u16) -> f64,
        hash: fn(u16, &str) -> String,
    ) -> impl FnMut(&StreamConfig, u32) -> RequestSample {
        let mut busy_once = true;
        move |c: &StreamConfig, _i: u32| {
            if busy_once {
                busy_once = false;
                let mut s = complete_sample();
                s.outcome = Outcome::HttpStatus;
                s.http_status = Some(503);
                s.output_sha256 = None;
                return s;
            }
            assert_eq!(c.temperature, Some(0.0));
            let rate = rate_by_port(c.port);
            sample(
                11,
                100.0,
                100.0 + 10.0 / rate * 1000.0,
                &hash(c.port, &c.prompt),
            )
        }
    }

    fn cfg() -> AbbaConfig {
        AbbaConfig {
            baseline: StreamConfig {
                port: 1,
                ..StreamConfig::default()
            },
            candidate: StreamConfig {
                port: 2,
                ..StreamConfig::default()
            },
            prompts: vec!["a".into(), "b".into()],
            rounds: 1,
            candidate_warmup_passes: 1,
            busy_retries: 3,
            busy_backoff_max: Duration::from_millis(1),
            deadline: Duration::from_secs(60),
        }
    }

    #[test]
    fn abba_reports_geomean_and_delta_when_tokens_match() {
        let c = cfg();
        let mut m = stub(|p| if p == 1 { 10.0 } else { 11.0 }, |_, pr| pr.to_string());
        let run = run_abba_with(&c, &mut m);
        assert!(run.stopped.is_none());
        assert_eq!(run.reference[0].busy_retries, 1);
        assert_eq!(run.cells.len(), 4);
        let b = summarize_arm(&run, &c, Arm::Baseline);
        let k = summarize_arm(&run, &c, Arm::Candidate);
        assert!(b.tokens_verified() && k.tokens_verified());
        assert!((b.geomean.unwrap() - 10.0).abs() < 1e-9);
        assert!((delta(&b, &k).unwrap() - 0.1).abs() < 1e-9);
        assert!(arm_line(&k).contains("geomean"));
    }

    #[test]
    fn wrong_tokens_refuse_the_speed_figure() {
        let c = cfg();
        // The candidate is "faster" but its replies differ: no figure, no delta.
        let mut m = stub(
            |p| if p == 1 { 10.0 } else { 13.0 },
            |port, pr| format!("{}{}", pr, if port == 2 { "!" } else { "" }),
        );
        let run = run_abba_with(&c, &mut m);
        let b = summarize_arm(&run, &c, Arm::Baseline);
        let k = summarize_arm(&run, &c, Arm::Candidate);
        assert!(b.tokens_verified());
        assert!(!k.tokens_verified());
        assert_eq!(delta(&b, &k), None);
        let line = arm_line(&k);
        assert!(line.contains("UNVERIFIED tokens"));
        assert!(!line.contains("13"));
    }

    #[test]
    fn slower_cells_are_kept() {
        let c = cfg();
        let mut calls = 0u32;
        let mut base = stub(|_| 10.0, |_, pr| pr.to_string());
        // Every request of the last baseline cell runs at half speed.
        let mut m = |cf: &StreamConfig, i: u32| {
            calls += 1;
            let mut s = base(cf, i);
            if cf.port == 1 && calls > 2 + 2 + 2 + 4 + 1 {
                let f = s.first_content_ms.unwrap();
                s.last_content_ms = Some(f + 2000.0);
            }
            s
        };
        let run = run_abba_with(&c, &mut m);
        let b = summarize_arm(&run, &c, Arm::Baseline);
        assert_eq!(b.cell_medians.len(), 2);
        assert_eq!(b.slowest, Some(5.0));
        let g = b.geomean.unwrap();
        assert!((g - (50.0f64).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn a_failed_request_stops_the_run_and_leaves_tokens_unverified() {
        let c = cfg();
        let mut n = 0;
        let mut m = |cf: &StreamConfig, _i: u32| {
            n += 1;
            let mut s = sample(11, 0.0, 1000.0, &cf.prompt);
            if n == 6 {
                s.outcome = Outcome::ReadTimeout;
                s.output_sha256 = None;
            }
            s
        };
        let run = run_abba_with(&c, &mut m);
        assert!(run.stopped.is_some());
        assert!(!summarize_arm(&run, &c, Arm::Candidate).tokens_verified());
    }
}
