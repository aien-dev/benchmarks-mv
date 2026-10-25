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

//! The native ORIG-3 judge reproduces the committed `judge.out` byte for byte from the
//! committed sealed per-record files, and refuses malformed input.

use std::fs;
use std::path::Path;
use std::process::Command;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const EVIDENCE: &str = "evidence/ORIG3-SELECTOR";

fn run(args: &[&Path]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_bench_orig3_judge"))
        .args(args)
        .output()
        .expect("run bench_orig3_judge");
    (
        out.status.code().expect("exit code"),
        String::from_utf8(out.stdout).expect("utf-8 stdout"),
        String::from_utf8(out.stderr).expect("utf-8 stderr"),
    )
}

#[test]
fn committed_sealed_evidence_reproduces_judge_out() {
    let dir = Path::new(ROOT).join(EVIDENCE);
    let files: Vec<_> = ["mlp", "affine", "shuffled", "untrained"]
        .iter()
        .map(|a| dir.join(format!("sealed-{a}.records")))
        .collect();
    let refs: Vec<&Path> = files.iter().map(Path::new).collect();
    let (code, stdout, _) = run(&refs);
    let want = fs::read_to_string(dir.join("judge.out")).expect("read judge.out");
    assert_eq!(stdout, want);
    assert_eq!(code, 0);
    assert!(stdout.ends_with("ORIG3_VERDICT=INCONCLUSIVE\n"));
}

#[test]
fn refuses_malformed_line() {
    let dir = Path::new(ROOT).join(EVIDENCE);
    let tmp = std::env::temp_dir().join(format!("orig3-judge-bad-{}.records", std::process::id()));
    fs::write(&tmp, "1 2 0 4 4 4 2 0 4 4 4 1 1 1 1 1\n").unwrap();
    let good = dir.join("sealed-mlp.records");
    let (code, stdout, stderr) = run(&[&tmp, &good, &good, &good]);
    fs::remove_file(&tmp).ok();
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert!(stderr.contains("16 columns, want 17"));
}
