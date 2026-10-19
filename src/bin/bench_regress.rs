//! `bench_regress <dir>...`: scan aien-receipts bundles and ABBA receipts, judge every
//! metric's latest changes against its own history (Q3 + 3 * IQR of earlier relative
//! deltas), bucket what is over the fence as small, medium or large, and report a commit
//! as significant when its changes add up. Read-only. Thresholds: src/regress.rs and
//! docs/NOISE.md. Exit 0 always unless a directory cannot be read (2): a flag is a report,
//! not a failure. `--pipeline-wall` also reads each bundle's judge wall time (a pipeline
//! health signal dominated by cache hits, not a benchmark metric).

use aien_benchmarks::regress::{self, MIN_HISTORY};
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let mut pipeline_wall = false;
    let dirs: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| {
            let flag = a == "--pipeline-wall";
            pipeline_wall |= flag;
            !flag
        })
        .collect();
    if dirs.is_empty() {
        eprintln!("usage: bench_regress [--pipeline-wall] <dir>... (for example ~/workspace/aien-receipts)");
        std::process::exit(2);
    }
    let mut series = BTreeMap::new();
    for d in &dirs {
        match regress::collect(Path::new(d), pipeline_wall, &mut series) {
            Ok(n) => println!("read {} point(s) from {}", n, d),
            Err(e) => {
                eprintln!("bench_regress: {}: {}", d, e);
                std::process::exit(2);
            }
        }
    }
    if series.is_empty() {
        println!("no benchmark metric found: aien-receipts bundles carry gate verdicts, not speeds; ABBA receipts (aien-abba-receipt/1) are read when present");
    }
    let mut all = Vec::new();
    println!(
        "{:<44} {:>6} {:>9} {:>9}  flags",
        "metric", "points", "median", "fence%"
    );
    for (metric, points) in &series {
        let mut vals: Vec<f64> = points.iter().map(|p| p.value).collect();
        vals.sort_by(f64::total_cmp);
        let med = vals[vals.len() / 2];
        let mut sorted = points.clone();
        sorted.sort_by(|a, b| a.at.cmp(&b.at));
        let deltas: Vec<f64> = sorted
            .windows(2)
            .filter(|w| w[0].value > 0.0)
            .map(|w| ((w[1].value - w[0].value) / w[0].value).abs())
            .collect();
        let fence = regress::fence(&deltas).map_or(format!("<{} deltas", MIN_HISTORY), |f| {
            format!("{:.1}", f * 100.0)
        });
        let flags = regress::scan_series(metric, points);
        println!(
            "{:<44} {:>6} {:>9.2} {:>9}  {}",
            metric,
            points.len(),
            med,
            fence,
            flags.len()
        );
        all.extend(flags);
    }
    println!();
    for f in &all {
        println!(
            "FLAG {:<6} {:<11} {} {} {:.2} -> {:.2} ({:+.1}%, fence {:.1}%) at {}",
            f.magnitude.as_str(),
            if f.regression {
                "regression"
            } else {
                "improvement"
            },
            f.metric,
            f.commit,
            f.previous,
            f.value,
            f.delta * 100.0,
            f.fence * 100.0,
            f.at
        );
    }
    let sig = regress::significant_commits(&all);
    for (commit, score) in &sig {
        println!("SIGNIFICANT {} score {}", commit, score);
    }
    println!(
        "{} metric(s), {} flag(s), {} significant commit(s)",
        series.len(),
        all.len(),
        sig.len()
    );
}
