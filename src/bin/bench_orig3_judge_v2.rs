// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Drake Stapleton (AIEN / aien-dev)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! ORIG-3 v2 judge: applies the frozen `orig3_selector_v2.json` decision rule
//! (`docs/orig3/PROTOCOL-v2.md`) to selmain mode E v2 output and the ORIG-4 overhead runs.
//!
//! Inputs: three per-record files written by aien-train `selmain` (mode E, config flags 14)
//! for the candidate, the shuffled-label control and the untrained control; the output
//! directory of omega-mv `tests/orig4/run.sh` with `ORIG4_OVERHEAD=1`; the candidate's
//! training wall time and the collection wall time in milliseconds.
//! Per-record columns (21): `seq fam core M N model rule ref static near oracle c_model
//! c_oracle c_rule c_ref c_static c_near press calls split cell`, costs in ps per call.
//! The three files must agree on every column except `model` and `c_model`.
//! Exit 0 after printing the verdict; exit 2 on unreadable, malformed or inconsistent input.
//! Reads files only: no network, no GPU, no live server.

use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "bench_orig3_judge_v2")]
#[command(
    about = "Apply the ORIG-3 v2 frozen decision rule to selmain E v2 files and overhead runs"
)]
struct Args {
    /// Candidate (mlp_scratch_v2) per-record file
    #[arg(long)]
    mlp: PathBuf,
    /// shuffled-label control per-record file
    #[arg(long)]
    shuffled: PathBuf,
    /// untrained control per-record file
    #[arg(long)]
    untrained: PathBuf,
    /// OUT directory of tests/orig4/run.sh ORIG4_OVERHEAD=1 (summary.txt, ovh_*_1.stdout)
    #[arg(long)]
    overhead_dir: PathBuf,
    /// Candidate training wall time, ms (run-select.sh v2_train_mlp wall_ms)
    #[arg(long)]
    train_wall_ms: u64,
    /// Collection wall time, ms (harness start to end)
    #[arg(long)]
    collect_wall_ms: u64,
}

const COLS: usize = 21;
const CORES: usize = 2;
/// Superiority margin: total <= 0.99 x min(retrieval, heuristic), in percent.
const SUP_PCT: u128 = 99;
/// Non-inferiority bound: total > 1.01 x min(retrieval, heuristic) is NO-GO, in percent.
const NONINF_PCT: u128 = 101;
/// One-sided exact sign test level per contrast (0.05 / 4: 2 core classes x 2 comparators).
const ALPHA: f64 = 0.0125;

/// One decision of one per-record line.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Rec {
    core: usize,
    press: u64,
    split: u64,
    calls: u128,
    model: u64,
    oracle: u64,
    rule: u64,
    reference: u64,
    stat: u64,
    near: u64,
    /// Every column except model and c_model, for the cross-file consistency check.
    key: [u64; COLS - 2],
}

fn load(path: &Path) -> Result<Vec<Rec>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let w: Vec<u64> = line
            .split_whitespace()
            .map(|f| f.parse::<u64>())
            .collect::<Result<_, _>>()
            .map_err(|e| format!("{}:{}: {e}", path.display(), ln + 1))?;
        if w.len() != COLS {
            return Err(format!(
                "{}:{}: {} columns, want {COLS}",
                path.display(),
                ln + 1,
                w.len()
            ));
        }
        let (core, press, split) = (w[2] as usize, w[17], w[19]);
        if core > 2 || press > 1 || !(split == 1 || split == 2) {
            return Err(format!(
                "{}:{}: core {core} press {press} split {split} out of range",
                path.display(),
                ln + 1
            ));
        }
        let mut key = [0u64; COLS - 2];
        let mut k = 0;
        for (i, v) in w.iter().enumerate() {
            if i != 5 && i != 11 {
                key[k] = *v;
                k += 1;
            }
        }
        out.push(Rec {
            core,
            press,
            split,
            calls: u128::from(w[18]),
            model: w[11],
            oracle: w[12],
            rule: w[13],
            reference: w[14],
            stat: w[15],
            near: w[16],
            key,
        });
    }
    Ok(out)
}

/// Lower median of a non-empty list.
fn median(mut v: Vec<u64>) -> u64 {
    v.sort_unstable();
    v[(v.len() - 1) / 2]
}

/// `scored=` of the COST line of one overhead run.
fn scored(dir: &Path, run: &str) -> Result<u64, String> {
    let p = dir.join(format!("{run}.stdout"));
    let text = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("COST ") {
            for f in rest.split_whitespace() {
                if let Some(v) = f.strip_prefix("scored=") {
                    return v.parse().map_err(|e| format!("{}: {e}", p.display()));
                }
            }
        }
    }
    Err(format!("{}: no COST scored= line", p.display()))
}

struct Overhead {
    reps: usize,
    med: [u64; 3],
    scored_mlp: u64,
    scored_near: u64,
    mlp_ps: u128,
    near_ps: u128,
}

/// Per-decision overhead in ps: (median wall of the advisor run - median wall of the disabled
/// run) / scored decisions, from the interleaved OVERHEAD lines of summary.txt.
fn overhead(dir: &Path) -> Result<Overhead, String> {
    let p = dir.join("summary.txt");
    let text = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut walls: [Vec<u64>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("OVERHEAD ") else {
            continue;
        };
        let mut run = None;
        let mut us = None;
        for f in rest.split_whitespace() {
            if let Some(v) = f.strip_prefix("run=") {
                run = Some(v);
            } else if let Some(v) = f.strip_prefix("wall_us=") {
                us = Some(
                    v.parse::<u64>()
                        .map_err(|e| format!("{}: {e}", p.display()))?,
                );
            }
        }
        let i = match run {
            Some("disabled") => 0,
            Some("mlp") => 1,
            Some("near") => 2,
            _ => return Err(format!("{}: bad OVERHEAD line {line}", p.display())),
        };
        walls[i].push(us.ok_or_else(|| format!("{}: no wall_us in {line}", p.display()))?);
    }
    let reps = walls[0].len();
    if reps == 0 || walls.iter().any(|w| w.len() != reps) {
        return Err(format!("{}: unequal or missing OVERHEAD runs", p.display()));
    }
    let med = [
        median(walls[0].clone()),
        median(walls[1].clone()),
        median(walls[2].clone()),
    ];
    let scored_mlp = scored(dir, "ovh_mlp_1")?;
    let scored_near = scored(dir, "ovh_near_1")?;
    if scored_mlp == 0 || scored_near == 0 {
        return Err("overhead runs scored no decision".into());
    }
    let per = |m: u64, s: u64| u128::from(m.saturating_sub(med[0])) * 1_000_000 / u128::from(s);
    Ok(Overhead {
        reps,
        med,
        scored_mlp,
        scored_near,
        mlp_ps: per(med[1], scored_mlp),
        near_ps: per(med[2], scored_near),
    })
}

/// Exact one-sided sign test: P(X >= k | X ~ Binomial(n, 1/2)).
fn sign_p(k: u64, n: u64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let ln2 = std::f64::consts::LN_2;
    let mut lnc = 0.0f64;
    let mut p = 0.0f64;
    for i in 0..=n {
        if i > 0 {
            lnc += ((n - i + 1) as f64).ln() - (i as f64).ln();
        }
        if i >= k {
            p += (lnc - n as f64 * ln2).exp();
        }
    }
    p.min(1.0)
}

const POLICIES: [&str; 10] = [
    "mlp_scratch_v2",
    "retrieval",
    "heuristic",
    "best_static",
    "fixed_rule",
    "always_reference",
    "untrained_control",
    "shuffled_label_control",
    "constant_control",
    "oracle",
];
const HEUR: [&str; 3] = ["fixed_rule", "always_reference", "best_static"];

/// ps per call chosen by policy p for decision i (index into the three aligned files).
fn ps(p: usize, i: usize, m: &[Rec], s: &[Rec], u: &[Rec], heur: usize) -> u64 {
    let r = &m[i];
    let h = [r.rule, r.reference, r.stat];
    match p {
        0 => r.model,
        1 => r.near,
        2 => h[heur],
        3 => r.stat,
        4 => r.rule,
        5 => r.reference,
        6 => u[i].model,
        7 => s[i].model,
        // A constant score ranks every arm equal; the frozen tie rule picks arm 0.
        8 => r.reference,
        _ => r.oracle,
    }
}

fn fail(msg: String) -> ! {
    eprintln!("bench_orig3_judge_v2: {msg}");
    std::process::exit(2);
}

fn ratio(a: u128, b: u128) -> f64 {
    a as f64 / b as f64
}

fn main() {
    let args = Args::parse();
    let m = load(&args.mlp).unwrap_or_else(|e| fail(e));
    let s = load(&args.shuffled).unwrap_or_else(|e| fail(e));
    let u = load(&args.untrained).unwrap_or_else(|e| fail(e));
    if m.len() != s.len() || m.len() != u.len() {
        fail("the three per-record files differ in length".into());
    }
    for i in 0..m.len() {
        if m[i].key != s[i].key || m[i].key != u[i].key {
            fail(format!(
                "line {} differs between files outside model columns",
                i + 1
            ));
        }
    }
    let o = overhead(&args.overhead_dir).unwrap_or_else(|e| fail(e));
    // Overhead per policy, ps per decision: the scorer path for the candidate and every
    // control (they run the same advisor), retrieval its own, heuristics and oracle 0.
    let ovh = |p: usize| -> u128 {
        match p {
            0 | 6 | 7 | 8 => o.mlp_ps,
            1 => o.near_ps,
            _ => 0,
        }
    };
    println!(
        "orig3_v2 rule=manifests/prereg/orig3_selector_v2.json protocol=docs/orig3/PROTOCOL-v2.md"
    );
    println!(
        "overhead reps={} median_wall_us disabled={} mlp={} near={} scored mlp={} near={} per_decision_ps mlp={} retrieval={} heuristics=0",
        o.reps, o.med[0], o.med[1], o.med[2], o.scored_mlp, o.scored_near, o.mlp_ps, o.near_ps
    );
    let mut primary = [[false; 10]; CORES];
    let mut noninf_fail = [false; CORES];
    for c in 0..CORES {
        let val: Vec<usize> = (0..m.len())
            .filter(|&i| m[i].core == c && m[i].split == 1 && m[i].press == 0)
            .collect();
        let held: Vec<usize> = (0..m.len())
            .filter(|&i| m[i].core == c && m[i].split == 2 && m[i].press == 0)
            .collect();
        if val.is_empty() || held.is_empty() {
            fail(format!(
                "core {c}: no VALIDATION or HELD-OUT pressure-0 decisions"
            ));
        }
        let vt: Vec<u128> = (0..3)
            .map(|h| {
                val.iter()
                    .map(|&i| m[i].calls * u128::from([m[i].rule, m[i].reference, m[i].stat][h]))
                    .sum()
            })
            .collect();
        let mut heur = 0;
        for h in 1..3 {
            if vt[h] < vt[heur] {
                heur = h;
            }
        }
        println!(
            "core={c} heldout_n={} validation_n={} validation_total_ps fixed_rule={} always_reference={} best_static={} heuristic={}",
            held.len(),
            val.len(),
            vt[0],
            vt[1],
            vt[2],
            HEUR[heur]
        );
        let cost =
            |p: usize, i: usize| m[i].calls * u128::from(ps(p, i, &m, &s, &u, heur)) + ovh(p);
        let choice = |p: usize| -> u128 {
            held.iter()
                .map(|&i| m[i].calls * u128::from(ps(p, i, &m, &s, &u, heur)))
                .sum()
        };
        let total: Vec<u128> = (0..10)
            .map(|p| held.iter().map(|&i| cost(p, i)).sum())
            .collect();
        let oracle = choice(9);
        let cmp = total[1].min(total[2]);
        for p in 0..10 {
            println!(
                "core={c} policy={} choice_ps={} overhead_ps={} total_ps={} choice/oracle={:.4} total/oracle_choice={:.4} total/min(retrieval,heuristic)={:.4}",
                POLICIES[p],
                choice(p),
                ovh(p) * held.len() as u128,
                total[p],
                ratio(choice(p), oracle),
                ratio(total[p], oracle),
                ratio(total[p], cmp)
            );
        }
        for p in [0usize, 6, 7, 8] {
            let p1 = total[p] * 100 <= SUP_PCT * cmp;
            let mut p2 = true;
            let mut signs = String::new();
            for q in [1usize, 2] {
                let (mut k, mut n) = (0u64, 0u64);
                for &i in &held {
                    let (a, b) = (cost(p, i), cost(q, i));
                    if a != b {
                        n += 1;
                        k += u64::from(a < b);
                    }
                }
                let pv = sign_p(k, n);
                p2 &= pv <= ALPHA;
                signs.push_str(&format!(
                    " vs_{} wins={k} discordant={n} p={pv:.3e}",
                    POLICIES[q]
                ));
            }
            primary[c][p] = p1 && p2;
            println!(
                "core={c} primary policy={} P1(total<=0.99*min)={} P2(sign<=0.0125 vs both)={}{} PRIMARY={}",
                POLICIES[p],
                u8::from(p1),
                u8::from(p2),
                signs,
                u8::from(primary[c][p])
            );
        }
        noninf_fail[c] = total[0] * 100 > NONINF_PCT * cmp;
        println!(
            "core={c} noninferiority_fail(total>1.01*min)={} candidate/min={:.4}",
            u8::from(noninf_fail[c]),
            ratio(total[0], cmp)
        );
        // Amortization against the cheaper comparator (overhead included).
        let q = if total[1] <= total[2] { 1 } else { 2 };
        let n = held.len() as u128;
        let saving = total[q] as i128 - total[0] as i128;
        let per_dec = saving as f64 / n as f64;
        let fixed_ps = u128::from(args.train_wall_ms + args.collect_wall_ms) * 1_000_000_000;
        let calls: u128 = held.iter().map(|&i| m[i].calls).sum();
        let per_call = (choice(q) as f64 - choice(0) as f64) / calls as f64;
        let be_dec = if per_dec > 0.0 {
            format!("{:.0}", (fixed_ps as f64 / per_dec).ceil())
        } else {
            "none".into()
        };
        let be_calls = if per_call > 0.0 {
            format!("{:.0}", (o.mlp_ps as f64 / per_call).ceil())
        } else {
            "none".into()
        };
        println!(
            "core={c} amortization comparator={} saving_ps_per_decision={per_dec:.1} train_ms={} collect_ms={} break_even_decisions={be_dec} per_call_saving_ps={per_call:.3} mean_calls_per_decision={:.1} break_even_calls_per_decision={be_calls}",
            POLICIES[q],
            args.train_wall_ms,
            args.collect_wall_ms,
            calls as f64 / n as f64
        );
        // Reported only: held-out decisions at pressure 1 (no overhead, choice cost only).
        let p1: Vec<usize> = (0..m.len())
            .filter(|&i| m[i].core == c && m[i].split == 2 && m[i].press == 1)
            .collect();
        if !p1.is_empty() {
            let ch = |p: usize| -> u128 {
                p1.iter()
                    .map(|&i| m[i].calls * u128::from(ps(p, i, &m, &s, &u, heur)))
                    .sum()
            };
            let or = ch(9);
            println!(
                "core={c} reported_pressure1 n={} choice/oracle mlp={:.4} retrieval={:.4} heuristic={:.4} best_static={:.4} fixed_rule={:.4}",
                p1.len(),
                ratio(ch(0), or),
                ratio(ch(1), or),
                ratio(ch(2), or),
                ratio(ch(3), or),
                ratio(ch(4), or)
            );
        }
    }
    let invalid = (0..CORES).any(|c| primary[c][6] || primary[c][7] || primary[c][8]);
    let go = (0..CORES).all(|c| primary[c][0]);
    let nogo = noninf_fail.iter().any(|&f| f);
    let verdict = if invalid {
        "INVALID"
    } else if go {
        "GO"
    } else if nogo {
        "NO-GO"
    } else {
        "INCONCLUSIVE"
    };
    println!("ORIG3_V2_VERDICT={verdict}");
}
