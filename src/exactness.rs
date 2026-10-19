//! Exactness contract for an OpenAI-compatible server: pure checks, no sockets.
//!
//! Idea credit: TensorFold (github.com/ashhart/TensorFold, CONTRIBUTING.md "Exactness and
//! precision", docs/cache-benchmarks.md). The engine is its own oracle: at one checkpoint,
//! backend and settings, greedy output must be byte-equal whether it was drafted or plain,
//! resumed from cached state or prefilled fresh, and sent alongside other streams or alone.
//!
//! Evidence rules (from the same source):
//! - Token equality is token-ID equality. Text equality is not token equality, so a pair whose
//!   token ids are missing is `Unverified`, never `Pass`.
//! - Cache state is observed, never inferred from the request order. `Cold` needs an explicit
//!   zero reused-token count, `Reused` a positive count not above the prompt count; anything
//!   missing or out of range is `Unverified`.
//! - A pair whose ids differ is `Fail` whatever the cache state was. A pair whose ids match but
//!   whose cache states were not the ones the equality is about is `Unverified`: the path under
//!   test did not run.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// sha256 over the token ids as consecutive little-endian u32 values, lowercase hex.
/// The server-side `usage.aien_token_sha256` proposed with this harness uses the same bytes.
pub fn token_sha256(ids: &[u32]) -> String {
    let mut h = Sha256::new();
    for id in ids {
        h.update(id.to_le_bytes());
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Observed prompt-cache state of one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "reused_tokens")]
pub enum CacheState {
    /// The server reported zero reused prompt tokens.
    Cold,
    /// The server reported this many reused prompt tokens (0 < n <= prompt tokens).
    Reused(usize),
    /// No valid report.
    Unverified,
}

impl CacheState {
    /// Classifies a reported reused-token count against the reported prompt token count.
    pub fn observe(reused: Option<usize>, prompt_tokens: Option<usize>) -> Self {
        match (reused, prompt_tokens) {
            (Some(0), Some(_)) => Self::Cold,
            (Some(n), Some(p)) if n <= p => Self::Reused(n),
            _ => Self::Unverified,
        }
    }

    pub fn is_cold(self) -> bool {
        self == Self::Cold
    }

    pub fn is_reused(self) -> bool {
        matches!(self, Self::Reused(_))
    }
}

/// Reused prompt tokens for request `id`, read from the aien-serve-http log.
///
/// The server prints the engine's line (`qwen38: ... reuse Live(n) (n tok)` or
/// `omega: ... , n reused from the previous turn`) on the generation thread immediately before
/// the request's own `<id>: prompt P tok, reply R tok` line, and runs one generation at a time,
/// so the nearest engine line above the request's line belongs to it. Returns `None` when either
/// line is missing or does not parse.
pub fn reused_from_server_log(log: &str, id: &str) -> Option<usize> {
    let lines: Vec<&str> = log.lines().collect();
    let tag = format!("{id}: prompt ");
    let at = lines.iter().rposition(|l| l.contains(&tag))?;
    // Only the line directly above may carry the engine report: anything else is ambiguous.
    let engine_line = lines.get(at.checked_sub(1)?)?;
    parse_engine_reuse(engine_line)
}

fn parse_engine_reuse(line: &str) -> Option<usize> {
    if line.contains(": qwen38: lock wait") {
        // `reuse Cold (0 tok)`, `reuse Live(1234) (1234 tok)`, `reuse Snapshot(9) (9 tok)`
        let rest = &line[line.find(", reuse ")? + ", reuse ".len()..];
        let open = rest.find(" (")?;
        let close = rest[open..].find(" tok)")?;
        return rest[open + 2..open + close].parse().ok();
    }
    if line.contains(": omega: lock wait") {
        // `prompt 584 tok, 512 reused from the previous turn`
        let end = line.find(" reused from the previous turn")?;
        let start = line[..end].rfind(", ")? + 2;
        return line[start..end].parse().ok();
    }
    None
}

/// What one request produced, as observed by the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Label of the run in the plan, e.g. `long/fresh`.
    pub label: String,
    /// Response `id` (the server log key).
    pub id: Option<String>,
    pub prompt_tokens: Option<usize>,
    pub completion_tokens: Option<usize>,
    pub finish_reason: Option<String>,
    /// `usage.aien_token_ids`; `None` when the server did not return them. Kept in the report so
    /// a later run can compare against it (`--reference`).
    pub token_ids: Option<Vec<u32>>,
    pub token_count: Option<usize>,
    pub token_sha256: Option<String>,
    /// sha256 of the reply text, for reference only (text equality is not token equality).
    pub text_sha256: String,
    /// The reply text (a follow-up turn sends it back); not in the report.
    #[serde(skip, default)]
    pub reply_text: String,
    pub cache: CacheState,
    /// 503 answers seen before this request was served.
    pub busy_retries: u32,
    pub wall_ms: u128,
}

impl Observation {
    pub fn new(
        label: &str,
        id: Option<String>,
        prompt_tokens: Option<usize>,
        completion_tokens: Option<usize>,
        finish_reason: Option<String>,
        token_ids: Option<Vec<u32>>,
        text: &str,
    ) -> Self {
        let text_sha256 = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Self {
            label: label.to_string(),
            id,
            prompt_tokens,
            completion_tokens,
            finish_reason,
            token_count: token_ids.as_ref().map(Vec::len),
            token_sha256: token_ids.as_deref().map(token_sha256),
            token_ids,
            text_sha256,
            reply_text: text.to_string(),
            cache: CacheState::Unverified,
            busy_retries: 0,
            wall_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Fail,
    Unverified,
}

/// One byte-equality between a reference run and a candidate run.
#[derive(Debug, Clone, Serialize)]
pub struct PairResult {
    pub equality: String,
    pub reference: String,
    pub candidate: String,
    pub verdict: Verdict,
    pub reason: String,
}

/// Compares token ids. `preconditions_met` says whether the observed cache states are the ones
/// the equality is about (for example the fresh run cold and the resumed run reused).
pub fn compare(
    equality: &str,
    reference: &Observation,
    candidate: &Observation,
    preconditions_met: bool,
) -> PairResult {
    let (verdict, reason) = match (&reference.token_ids, &candidate.token_ids) {
        (Some(a), Some(b)) if a != b => {
            let first = a.iter().zip(b).position(|(x, y)| x != y);
            let at = first.unwrap_or(a.len().min(b.len()));
            (
                Verdict::Fail,
                format!(
                    "token ids differ at index {at} (lengths {} and {})",
                    a.len(),
                    b.len()
                ),
            )
        }
        (Some(a), Some(_)) if a.is_empty() => {
            (Verdict::Unverified, "both runs returned no tokens".into())
        }
        (Some(a), Some(_)) if preconditions_met => {
            (Verdict::Pass, format!("{} token ids identical", a.len()))
        }
        (Some(_), Some(_)) => (
            Verdict::Unverified,
            format!(
                "token ids identical, but observed cache states ({:?}, {:?}) did not exercise the path",
                reference.cache, candidate.cache
            ),
        ),
        _ => (
            Verdict::Unverified,
            "token ids missing from usage (text equality is not token equality)".into(),
        ),
    };
    PairResult {
        equality: equality.to_string(),
        reference: reference.label.clone(),
        candidate: candidate.label.clone(),
        verdict,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(label: &str, ids: Option<Vec<u32>>, cache: CacheState) -> Observation {
        let mut o = Observation::new(label, None, Some(10), None, None, ids, "x");
        o.cache = cache;
        o
    }

    #[test]
    fn token_hash_is_sha256_of_le_u32() {
        // sha256 of the empty string
        assert_eq!(
            token_sha256(&[]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // [1] is the bytes 01 00 00 00, which is not the hash of [256] (00 01 00 00)
        assert_ne!(token_sha256(&[1]), token_sha256(&[256]));
        assert_ne!(token_sha256(&[1, 2]), token_sha256(&[2, 1]));
    }

    #[test]
    fn cache_state_needs_an_explicit_valid_count() {
        assert_eq!(CacheState::observe(Some(0), Some(5)), CacheState::Cold);
        assert_eq!(CacheState::observe(Some(5), Some(5)), CacheState::Reused(5));
        assert_eq!(
            CacheState::observe(Some(6), Some(5)),
            CacheState::Unverified
        );
        assert_eq!(CacheState::observe(None, Some(5)), CacheState::Unverified);
        assert_eq!(CacheState::observe(Some(0), None), CacheState::Unverified);
    }

    #[test]
    fn reads_reuse_from_qwen38_and_omega_log_lines() {
        let log = "\
aien-serve-http: qwen38: lock wait 39983 ms, prompt 584 tok, reuse Cold (0 tok), fed 584 (0 stepped), prefill 5047 ms
aien-serve-http: chatcmpl-a: prompt 584 tok, reply 422 tok, finish stop
aien-serve-http: qwen38: lock wait 1 ms, prompt 700 tok, reuse Snapshot(690) (690 tok), fed 10 (0 stepped)
aien-serve-http: chatcmpl-b: prompt 700 tok, reply 4 tok, finish length
aien-serve-http: omega: lock wait 0 ms, prompt 584 tok, 512 reused from the previous turn, prefill 9 ms
aien-serve-http: chatcmpl-c: prompt 584 tok, reply 4 tok, finish length
aien-serve-http: chatcmpl-d: GPU busy: GPU lock held by other jobs for 300 s
aien-serve-http: chatcmpl-e: prompt 9 tok, reply 1 tok, finish length
";
        assert_eq!(reused_from_server_log(log, "chatcmpl-a"), Some(0));
        assert_eq!(reused_from_server_log(log, "chatcmpl-b"), Some(690));
        assert_eq!(reused_from_server_log(log, "chatcmpl-c"), Some(512));
        // The line above e is not an engine line: no report, never a guessed zero.
        assert_eq!(reused_from_server_log(log, "chatcmpl-e"), None);
        assert_eq!(reused_from_server_log(log, "chatcmpl-zz"), None);
    }

    #[test]
    fn verdicts_follow_the_evidence_rules() {
        let cold = CacheState::Cold;
        let warm = CacheState::Reused(9);
        let a = obs("fresh", Some(vec![1, 2, 3]), cold);
        let b = obs("resumed", Some(vec![1, 2, 3]), warm);
        let c = obs("resumed", Some(vec![1, 2, 4]), warm);
        let none = obs("resumed", None, warm);
        assert_eq!(compare("r", &a, &b, true).verdict, Verdict::Pass);
        assert_eq!(compare("r", &a, &b, false).verdict, Verdict::Unverified);
        let fail = compare("r", &a, &c, false);
        assert_eq!(fail.verdict, Verdict::Fail);
        assert!(fail.reason.contains("index 2"), "{}", fail.reason);
        assert_eq!(compare("r", &a, &none, true).verdict, Verdict::Unverified);
        let short = obs("resumed", Some(vec![1, 2]), warm);
        assert!(compare("r", &a, &short, true).reason.contains("index 2"));
    }
}
