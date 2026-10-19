//! Regression fences over a metric's own history.
//!
//! Idea credit: rustc-perf docs/comparison-analysis.md (a change is significant only when its
//! relative delta is an outlier against that benchmark's own history, Q3 + 3 * IQR, lower fence
//! ignored; then bucketed by magnitude) and radar.lean-lang.org/about (small, medium and large
//! changes accumulate; a commit is significant when enough of them pile up). No code copied.
//!
//! Thresholds (and why) are written down in docs/NOISE.md; the constants below are the only
//! copy the code reads.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Fewer earlier deltas than this and no fence is drawn: with 7 values each quartile rests on
/// one or two observations, so one noisy run would move the fence by itself.
pub const MIN_HISTORY: usize = 8;
/// rustc-perf's outer fence: Q3 + 3 * IQR of the absolute relative deltas.
pub const IQR_MULTIPLIER: f64 = 3.0;
/// Over-the-fence ratio above which a change is medium (2x) and large (4x).
pub const MEDIUM_RATIO: f64 = 2.0;
pub const LARGE_RATIO: f64 = 4.0;
/// A change under 1 % is at most small; under 5 % at most medium. 5 % is the top of the
/// 4 to 6 % spread TensorFold measured between two GB10s on identical code: one box cannot
/// call a change of that size large.
pub const SMALL_CAP_PCT: f64 = 1.0;
pub const MEDIUM_CAP_PCT: f64 = 5.0;
/// Commit score: small 1, medium 2, large 4; significant at 4 or more, which is one large,
/// two medium or four small changes (Lean radar's "a small number of large, a medium number
/// of medium, a large number of small"), and any mix that adds up.
pub const SIGNIFICANT_SCORE: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Magnitude {
    Small,
    Medium,
    Large,
}

impl Magnitude {
    pub fn as_str(self) -> &'static str {
        match self {
            Magnitude::Small => "small",
            Magnitude::Medium => "medium",
            Magnitude::Large => "large",
        }
    }

    pub fn weight(self) -> u32 {
        match self {
            Magnitude::Small => 1,
            Magnitude::Medium => 2,
            Magnitude::Large => 4,
        }
    }
}

/// One observation of one metric.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// Sort key (RFC 3339 UTC time).
    pub at: String,
    /// What the point belongs to, for example `omega@<40-hex commit>`.
    pub commit: String,
    pub value: f64,
}

/// Quartile by linear interpolation between closest ranks (the common "type 7").
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let h = (n - 1) as f64 * q;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

/// Upper fence Q3 + 3 * IQR of `history` (absolute relative deltas), or None when the
/// history is shorter than [`MIN_HISTORY`].
pub fn fence(history: &[f64]) -> Option<f64> {
    if history.len() < MIN_HISTORY {
        return None;
    }
    let mut s = history.to_vec();
    s.sort_by(f64::total_cmp);
    let q1 = quantile(&s, 0.25);
    let q3 = quantile(&s, 0.75);
    Some(q3 + IQR_MULTIPLIER * (q3 - q1))
}

/// Magnitude of a change `delta` (relative, signed) that is over `fence`; None when it is not.
pub fn classify(delta: f64, fence: f64) -> Option<Magnitude> {
    let d = delta.abs();
    if d <= fence {
        return None;
    }
    let ratio = if fence > 0.0 {
        d / fence
    } else {
        f64::INFINITY
    };
    let by_ratio = if ratio > LARGE_RATIO {
        Magnitude::Large
    } else if ratio > MEDIUM_RATIO {
        Magnitude::Medium
    } else {
        Magnitude::Small
    };
    let pct = d * 100.0;
    let cap = if pct < SMALL_CAP_PCT {
        Magnitude::Small
    } else if pct < MEDIUM_CAP_PCT {
        Magnitude::Medium
    } else {
        Magnitude::Large
    };
    Some(by_ratio.min(cap))
}

/// Higher is better for throughput metrics; lower is better for everything else (times).
pub fn higher_is_better(metric: &str) -> bool {
    metric.contains("tok_s") || metric.contains("per_sec")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flag {
    pub metric: String,
    pub commit: String,
    pub at: String,
    pub previous: f64,
    pub value: f64,
    pub delta: f64,
    pub fence: f64,
    pub magnitude: Magnitude,
    pub regression: bool,
}

/// Walk one metric's points in time order: each delta is judged against the deltas before it.
pub fn scan_series(metric: &str, points: &[Point]) -> Vec<Flag> {
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| a.at.cmp(&b.at));
    let mut history = Vec::new();
    let mut flags = Vec::new();
    for w in pts.windows(2) {
        let (prev, cur) = (&w[0], &w[1]);
        if prev.value <= 0.0 || !prev.value.is_finite() || !cur.value.is_finite() {
            continue;
        }
        let delta = (cur.value - prev.value) / prev.value;
        if let Some(f) = fence(&history) {
            if let Some(magnitude) = classify(delta, f) {
                flags.push(Flag {
                    metric: metric.to_string(),
                    commit: cur.commit.clone(),
                    at: cur.at.clone(),
                    previous: prev.value,
                    value: cur.value,
                    delta,
                    fence: f,
                    magnitude,
                    regression: (delta > 0.0) != higher_is_better(metric),
                });
            }
        }
        history.push(delta.abs());
    }
    flags
}

/// Commits whose flags add up to [`SIGNIFICANT_SCORE`], with their score.
pub fn significant_commits(flags: &[Flag]) -> BTreeMap<String, u32> {
    let mut score: BTreeMap<String, u32> = BTreeMap::new();
    for f in flags {
        *score.entry(f.commit.clone()).or_default() += f.magnitude.weight();
    }
    score.retain(|_, s| *s >= SIGNIFICANT_SCORE);
    score
}

fn rfc3339_secs(s: &str) -> Option<f64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.timestamp() as f64)
}

/// Points a JSON document contributes: (metric, point).
/// - `aien-verification-bundle/1` (aien-receipts), only with `pipeline_wall`:
///   `<repo>/pipeline_wall_s` = finished_utc - started_utc, keyed by repository and commit.
///   This is how long the judge took, which swings 100x with gate-result and build caches;
///   it is a pipeline health signal, not a benchmark metric, so it is off by default.
/// - `aien-abba-receipt/1`: `abba/<model>/<arm label>/decode_tok_s_geomean`, only for arms
///   whose tokens were verified.
pub fn points_from(v: &Value, pipeline_wall: bool) -> Vec<(String, Point)> {
    let s = |p: &str| v.pointer(p).and_then(Value::as_str);
    match s("/schema") {
        Some("aien-verification-bundle/1") if pipeline_wall => {
            let (Some(repo), Some(commit), Some(start), Some(end)) = (
                s("/repository"),
                s("/commit"),
                s("/started_utc"),
                s("/finished_utc"),
            ) else {
                return Vec::new();
            };
            let (Some(a), Some(b)) = (rfc3339_secs(start), rfc3339_secs(end)) else {
                return Vec::new();
            };
            let short = repo.rsplit('/').next().unwrap_or(repo);
            vec![(
                format!("{}/pipeline_wall_s", short),
                Point {
                    at: end.to_string(),
                    commit: format!("{}@{}", short, commit),
                    value: b - a,
                },
            )]
        }
        Some("aien-abba-receipt/1") => {
            let at = s("/generated_utc").unwrap_or_default().to_string();
            let mut out = Vec::new();
            for arm in ["baseline", "candidate"] {
                let a = |k: &str| v.pointer(&format!("/{}/{}", arm, k));
                let (Some(true), Some(g)) = (
                    a("tokens_verified").and_then(Value::as_bool),
                    a("decode_tok_s_geomean").and_then(Value::as_f64),
                ) else {
                    continue;
                };
                let label = a("label").and_then(Value::as_str).unwrap_or(arm);
                let model = a("model").and_then(Value::as_str).unwrap_or("model");
                let commit = a("engine_commit")
                    .and_then(Value::as_str)
                    .unwrap_or("not_recorded");
                out.push((
                    format!("abba/{}/{}/decode_tok_s_geomean", model, label),
                    Point {
                        at: at.clone(),
                        commit: format!("{}@{}", label, commit),
                        value: g,
                    },
                ));
            }
            out
        }
        _ => Vec::new(),
    }
}

/// Every `*.json` under `root` (recursively, skipping `.git`), grouped into series.
pub fn collect(
    root: &Path,
    pipeline_wall: bool,
    series: &mut BTreeMap<String, Vec<Point>>,
) -> std::io::Result<usize> {
    let mut files = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != ".git") {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "json") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(v) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                for (metric, p) in points_from(&v, pipeline_wall) {
                    series.entry(metric).or_default().push(p);
                    files += 1;
                }
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pts(values: &[f64]) -> Vec<Point> {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| Point {
                at: format!("2026-10-{:02}T00:00:00Z", i + 1),
                commit: format!("c{}", i),
                value: *v,
            })
            .collect()
    }

    #[test]
    fn quartiles_and_fence() {
        let s = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(quantile(&s, 0.25), 2.0);
        assert_eq!(quantile(&s, 0.75), 4.0);
        assert_eq!(fence(&[0.01; 7]), None);
        // History 1..8 percent: Q1 2.75, Q3 6.25, IQR 3.5, fence 6.25 + 10.5 = 16.75 percent.
        let h: Vec<f64> = (1..=8).map(|i| i as f64 / 100.0).collect();
        assert!((fence(&h).unwrap() - 0.1675).abs() < 1e-12);
    }

    #[test]
    fn magnitude_buckets_and_caps() {
        assert_eq!(classify(0.05, 0.10), None);
        assert_eq!(classify(0.15, 0.10), Some(Magnitude::Small));
        assert_eq!(classify(-0.30, 0.10), Some(Magnitude::Medium));
        assert_eq!(classify(0.50, 0.10), Some(Magnitude::Large));
        // Far over a tiny fence but only 3 percent: capped at medium; 0.5 percent at small.
        assert_eq!(classify(0.03, 0.001), Some(Magnitude::Medium));
        assert_eq!(classify(0.005, 0.0001), Some(Magnitude::Small));
    }

    #[test]
    fn a_noisy_history_absorbs_noise_and_flags_a_real_jump() {
        // Wall times wobbling about 100 s by up to +-3 percent, then a jump to 150 s.
        let mut v = vec![
            100.0, 102.0, 99.0, 101.0, 98.0, 100.0, 103.0, 100.0, 99.0, 101.0,
        ];
        assert!(scan_series("x/pipeline_wall_s", &pts(&v)).is_empty());
        v.push(150.0);
        let flags = scan_series("x/pipeline_wall_s", &pts(&v));
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].magnitude, Magnitude::Large);
        assert!(flags[0].regression, "slower wall time is a regression");
        // The same jump in a throughput metric is an improvement.
        assert!(!scan_series("abba/m/a/decode_tok_s_geomean", &pts(&v))[0].regression);
    }

    #[test]
    fn short_history_never_flags() {
        assert!(scan_series("x", &pts(&[100.0, 100.0, 100.0, 500.0])).is_empty());
    }

    #[test]
    fn commit_significance_accumulates() {
        let f = |c: &str, m| Flag {
            metric: "m".into(),
            commit: c.into(),
            at: String::new(),
            previous: 1.0,
            value: 2.0,
            delta: 1.0,
            fence: 0.1,
            magnitude: m,
            regression: true,
        };
        use Magnitude::*;
        let flags = vec![
            f("one-large", Large),
            f("two-medium", Medium),
            f("two-medium", Medium),
            f("three-small", Small),
            f("three-small", Small),
            f("three-small", Small),
            f("mixed", Medium),
            f("mixed", Small),
            f("mixed", Small),
        ];
        let sig = significant_commits(&flags);
        assert_eq!(
            sig.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["mixed", "one-large", "two-medium"]
        );
    }

    #[test]
    fn bundle_and_receipt_points() {
        let b = json!({"schema": "aien-verification-bundle/1", "repository": "github.com/aien-dev/omega",
            "commit": "a".repeat(40), "started_utc": "2026-10-19T13:00:00Z", "finished_utc": "2026-10-19T13:02:05Z"});
        assert!(
            points_from(&b, false).is_empty(),
            "pipeline wall time is opt-in"
        );
        let p = points_from(&b, true);
        assert_eq!(p[0].0, "omega/pipeline_wall_s");
        assert_eq!(p[0].1.value, 125.0);
        let r = json!({"schema": "aien-abba-receipt/1", "generated_utc": "2026-10-19T18:00:00Z",
            "baseline": {"label": "a", "model": "m", "tokens_verified": true, "decode_tok_s_geomean": 7.5},
            "candidate": {"label": "b", "model": "m", "tokens_verified": false, "decode_tok_s_geomean": "UNVERIFIED tokens"}});
        let p = points_from(&r, false);
        assert_eq!(
            p.len(),
            1,
            "an arm with unverified tokens contributes nothing"
        );
        assert_eq!(p[0].0, "abba/m/a/decode_tok_s_geomean");
    }
}
