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

//! The ORIG-3 v2 judge on synthetic inputs: every verdict of the frozen rule (GO, NO-GO,
//! INCONCLUSIVE, INVALID), the overhead arithmetic, and refusal of malformed or inconsistent
//! input. Synthetic numbers only; no record of any collection is used here.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Choices of one synthetic file: ps per call of the model's arm on every line.
struct Case {
    dir: PathBuf,
}

/// seq fam core M N model rule ref static near oracle c_model c_oracle c_rule c_ref c_static
/// c_near press calls split cell. Arms: rule 2 (900), ref 0 (1000), static 4 (800), near 4
/// (800), oracle 1 (700).
fn line(seq: usize, core: usize, split: u64, model_ps: u64) -> String {
    let arm = match model_ps {
        700 => 1,
        800 => 4,
        900 => 2,
        _ => 0,
    };
    format!(
        "{seq} 5 {core} 64 64 {arm} 2 0 4 4 1 {model_ps} 700 900 1000 800 800 0 100 {split} 102\n"
    )
}

fn records(model_ps: u64) -> String {
    let mut s = String::new();
    let mut seq = 0;
    for core in 0..2 {
        for k in 0..170 {
            let split = if k < 20 { 1 } else { 2 };
            s.push_str(&line(seq, core, split, model_ps));
            seq += 1;
        }
    }
    s
}

impl Case {
    fn new(tag: &str) -> Case {
        let dir = std::env::temp_dir().join(format!("orig3-judge-v2-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Case { dir }
    }
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let p = self.dir.join(name);
        fs::write(&p, text).unwrap();
        p
    }
    /// Overhead runs: wall_us medians disabled, mlp, near over 3 reps; 1000 scored each.
    fn overhead(&self, dis: u64, mlp: u64, near: u64) {
        let mut s = String::new();
        for rep in 1..=3 {
            let j = rep as u64;
            s.push_str(&format!(
                "OVERHEAD run=disabled rep={rep} wall_us={}\n",
                dis + j
            ));
            s.push_str(&format!("OVERHEAD run=mlp rep={rep} wall_us={}\n", mlp + j));
            s.push_str(&format!(
                "OVERHEAD run=near rep={rep} wall_us={}\n",
                near + j
            ));
        }
        self.write("summary.txt", &s);
        let cost =
            "COST records=1000 scored=1000 fallback=0 adv_cost=1 oracle_cost=1 incumbent_cost=1\n";
        self.write("ovh_mlp_1.stdout", cost);
        self.write("ovh_near_1.stdout", cost);
    }
    fn run(&self, mlp: &Path, shuf: &Path, untr: &Path) -> (i32, String, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_bench_orig3_judge_v2"))
            .arg("--mlp")
            .arg(mlp)
            .arg("--shuffled")
            .arg(shuf)
            .arg("--untrained")
            .arg(untr)
            .arg("--overhead-dir")
            .arg(&self.dir)
            .args(["--train-wall-ms", "3000", "--collect-wall-ms", "120000"])
            .output()
            .expect("run bench_orig3_judge_v2");
        (
            out.status.code().expect("exit code"),
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).ok();
    }
}

#[test]
fn better_choices_without_overhead_is_go() {
    let c = Case::new("go");
    c.overhead(1000, 1000, 1000);
    let m = c.write("m", &records(700));
    let bad = c.write("b", &records(1000));
    let (code, out, _) = c.run(&m, &bad, &bad);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("core=0 heldout_n=150 validation_n=20"),
        "{out}"
    );
    assert!(out.contains("heuristic=best_static"), "{out}");
    assert!(out.contains("per_decision_ps mlp=0 retrieval=0"), "{out}");
    // (3000 + 120000) ms = 1.23e14 ps over a saving of 10000 ps per decision.
    assert!(out.contains("break_even_decisions=12300000000 "), "{out}");
    assert!(out.ends_with("ORIG3_V2_VERDICT=GO\n"), "{out}");
}

#[test]
fn overhead_larger_than_the_saving_is_nogo() {
    let c = Case::new("nogo");
    // 100 ms over 1000 decisions = 100 us = 1e8 ps per decision for the scorer path.
    c.overhead(1000, 101_000, 1000);
    let m = c.write("m", &records(700));
    let bad = c.write("b", &records(1000));
    let (code, out, _) = c.run(&m, &bad, &bad);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("per_decision_ps mlp=100000000 retrieval=0"),
        "{out}"
    );
    assert!(
        out.contains("break_even_calls_per_decision=1000000"),
        "{out}"
    );
    assert!(out.ends_with("ORIG3_V2_VERDICT=NO-GO\n"), "{out}");
}

#[test]
fn parity_with_retrieval_is_inconclusive() {
    let c = Case::new("inc");
    c.overhead(1000, 1000, 1000);
    let m = c.write("m", &records(800));
    let bad = c.write("b", &records(1000));
    let (code, out, _) = c.run(&m, &bad, &bad);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("break_even_decisions=none"), "{out}");
    assert!(out.ends_with("ORIG3_V2_VERDICT=INCONCLUSIVE\n"), "{out}");
}

#[test]
fn a_control_meeting_the_primary_criterion_makes_the_run_invalid() {
    let c = Case::new("invalid");
    c.overhead(1000, 1000, 1000);
    let m = c.write("m", &records(700));
    let bad = c.write("b", &records(1000));
    let (code, out, _) = c.run(&m, &m, &bad);
    assert_eq!(code, 0, "{out}");
    assert!(out.ends_with("ORIG3_V2_VERDICT=INVALID\n"), "{out}");
}

#[test]
fn refuses_malformed_and_inconsistent_input() {
    let c = Case::new("bad");
    c.overhead(1000, 1000, 1000);
    let m = c.write("m", &records(700));
    let short = c.write(
        "s",
        "1 5 0 64 64 1 2 0 4 4 1 700 700 900 1000 800 800 0 100 2\n",
    );
    let (code, out, err) = c.run(&short, &m, &m);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.contains("20 columns, want 21"), "{err}");
    let other = c.write(
        "o",
        &records(700).replacen(" 900 1000 800 800 ", " 901 1000 800 800 ", 1),
    );
    let (code, _, err) = c.run(&m, &other, &m);
    assert_eq!(code, 2);
    assert!(err.contains("differs between files"), "{err}");
}

#[test]
fn committed_v2_evidence_reproduces_judge_out() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("evidence/ORIG3-SELECTOR-v2");
    let out = Command::new(env!("CARGO_BIN_EXE_bench_orig3_judge_v2"))
        .arg("--mlp")
        .arg(dir.join("v2-mlp.records"))
        .arg("--shuffled")
        .arg(dir.join("v2-shuffled.records"))
        .arg("--untrained")
        .arg(dir.join("v2-untrained.records"))
        .arg("--overhead-dir")
        .arg(dir.join("overhead"))
        .args(["--train-wall-ms", "10017", "--collect-wall-ms", "70575"])
        .output()
        .expect("run bench_orig3_judge_v2");
    let want = fs::read_to_string(dir.join("judge.out")).expect("judge.out");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), want);
    assert_eq!(out.status.code(), Some(0));
    assert!(want.ends_with("ORIG3_V2_VERDICT=NO-GO\n"));
}
