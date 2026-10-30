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

//! ORIG-3 judge: applies the preregistered `orig3_selector.json` decision rule to selmain E output.
//!
//! Reads the per-record lines that the aien-train `selmain` evaluator (mode E) writes on fd 4.
//! Native twin of `docs/orig3/judge.sh`: same input, byte-identical output, so either can be
//! rerun over `evidence/ORIG3-SELECTOR/`.
//!
//! Usage: `bench_orig3_judge MLP AFFINE SHUFFLED UNTRAINED` (four `.records` files).
//! Columns per line: `seq fam core M N model rule ref static near oracle c_model c_oracle
//! c_rule c_ref c_static c_near`. Gated families: 2 (H1) and 3 (H2); core classes 0 and 1.
//! Exit 0 after printing the verdict; exit 2 on unreadable or malformed input. Reads files
//! only: no network, no GPU, no live server.

use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bench_orig3_judge")]
#[command(about = "Apply the ORIG-3 preregistered decision rule to selmain E per-record files")]
struct Args {
    /// Candidate (mlp_scratch) per-record file
    mlp: PathBuf,
    /// affine baseline per-record file
    affine: PathBuf,
    /// shuffled_control per-record file
    shuffled: PathBuf,
    /// untrained_control per-record file
    untrained: PathBuf,
}

const CORES: usize = 2;
const CRIT_60: u64 = 37;
const CRIT_120: u64 = 70;

/// Totals of one arm's file over the gated families, per core class.
#[derive(Default, Clone, Copy)]
struct Totals {
    n: u64,
    model: u64,
    oracle: u64,
    rule: u64,
    reference: u64,
    stat: u64,
    near: u64,
    win_rule: u64,
    loss_rule: u64,
    win_near: u64,
    loss_near: u64,
    win_stat: u64,
    loss_stat: u64,
}

fn load(path: &PathBuf) -> Result<[Totals; CORES], String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut t = [Totals::default(); CORES];
    for (ln, line) in text.lines().enumerate() {
        let w: Vec<u64> = line
            .split_whitespace()
            .map(|f| f.parse::<u64>())
            .collect::<Result<_, _>>()
            .map_err(|e| format!("{}:{}: {e}", path.display(), ln + 1))?;
        if w.len() != 17 {
            return Err(format!(
                "{}:{}: {} columns, want 17",
                path.display(),
                ln + 1,
                w.len()
            ));
        }
        if w[1] != 2 && w[1] != 3 {
            continue;
        }
        let core = w[2] as usize;
        if core >= CORES {
            continue;
        }
        let c = &mut t[core];
        let (model, rule, reference, stat, near) = (w[11], w[13], w[14], w[15], w[16]);
        c.n += 1;
        c.model += model;
        c.oracle += w[12];
        c.rule += rule;
        c.reference += reference;
        c.stat += stat;
        c.near += near;
        c.win_rule += u64::from(model < rule);
        c.loss_rule += u64::from(model > rule);
        c.win_near += u64::from(model < near);
        c.loss_near += u64::from(model > near);
        c.win_stat += u64::from(model < stat);
        c.loss_stat += u64::from(model > stat);
    }
    Ok(t)
}

fn ratio(a: u64, b: u64) -> f64 {
    a as f64 / b as f64
}

fn main() {
    let args = Args::parse();
    let files = [args.mlp, args.affine, args.shuffled, args.untrained];
    let mut arms = Vec::with_capacity(files.len());
    for f in &files {
        match load(f) {
            Ok(t) => arms.push(t),
            Err(e) => {
                eprintln!("bench_orig3_judge: {e}");
                std::process::exit(2);
            }
        }
    }
    let (mlp, aff, shuf, untr) = (arms[0], arms[1], arms[2], arms[3]);
    let mut go = true;
    let mut nogo = false;
    for c in 0..CORES {
        let m = mlp[c];
        let crit = match m.n {
            60 => Some(CRIT_60),
            120 => Some(CRIT_120),
            _ => None,
        };
        let best = m.stat.min(m.near);
        let a = m.model as f64 <= 0.95 * m.rule as f64;
        let b = m.model <= m.reference;
        let cc = m.model <= aff[c].model;
        let d = crit.is_some_and(|k| m.win_rule >= k);
        let e = m.model <= best;
        let shuf_a = shuf[c].model as f64 <= 0.95 * shuf[c].rule as f64;
        let untr_a = untr[c].model as f64 <= 0.95 * untr[c].rule as f64;
        let f = !(shuf_a || untr_a);
        println!(
            "core {c} n {} SEALED H1+H2: mlp/oracle {:.4} aff/oracle {:.4} rule/oracle {:.4} ref/oracle {:.4} static/oracle {:.4} retrieval/oracle {:.4} shuffled/oracle {:.4} untrained/oracle {:.4}",
            m.n,
            ratio(m.model, m.oracle),
            ratio(aff[c].model, aff[c].oracle),
            ratio(m.rule, m.oracle),
            ratio(m.reference, m.oracle),
            ratio(m.stat, m.oracle),
            ratio(m.near, m.oracle),
            ratio(shuf[c].model, shuf[c].oracle),
            ratio(untr[c].model, untr[c].oracle),
        );
        println!(
            "core {c} A(<=0.95 rule)={} [{:.3}] B(<=ref)={} [{:.3}] C(<=affine)={} [{:.4}] D(sign vs rule {}/{} >= {})={} E(<=min(static,retrieval))={} [{:.4}] F(controls fail A: shuffled {:.3} untrained {:.3})={}",
            u8::from(a),
            ratio(m.model, m.rule),
            u8::from(b),
            ratio(m.model, m.reference),
            u8::from(cc),
            ratio(m.model, aff[c].model),
            m.win_rule,
            m.loss_rule,
            crit.map_or(-1, |k| k as i64),
            u8::from(d),
            u8::from(e),
            ratio(m.model, best),
            ratio(shuf[c].model, shuf[c].rule),
            ratio(untr[c].model, untr[c].rule),
            u8::from(f),
        );
        println!(
            "core {c} reported: sign vs retrieval +{}/-{}, vs static +{}/-{}",
            m.win_near, m.loss_near, m.win_stat, m.loss_stat
        );
        if !(a && b && cc && d && e && f) {
            go = false;
        }
        if m.model as f64 > 1.01 * best as f64 {
            nogo = true;
        }
    }
    let verdict = if go {
        "GO"
    } else if nogo {
        "NO-GO"
    } else {
        "INCONCLUSIVE"
    };
    println!("ORIG3_VERDICT={verdict}");
}
