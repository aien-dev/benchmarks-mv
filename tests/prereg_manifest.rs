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

//! The committed preregistration passes `bench_verify_prereg`; mutated copies of it are refused with exact defect lines.

use aien_benchmarks::prereg::check_prereg;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const MANIFEST: &str = "manifests/prereg/si_genesis_e2.json";

fn committed() -> Value {
    let text = fs::read_to_string(Path::new(ROOT).join(MANIFEST)).expect("read prereg manifest");
    serde_json::from_str(&text).expect("prereg manifest is JSON")
}

fn defects(v: &Value) -> Vec<String> {
    let mut out = Vec::new();
    check_prereg("m.json", v, &mut out);
    out.sort();
    out.dedup();
    out.iter().map(ToString::to_string).collect()
}

fn expect(v: &Value, want: &[&str]) {
    let want: Vec<String> = want.iter().map(|l| format!("m.json: {l}")).collect();
    assert_eq!(defects(v), want);
}

fn arm_index(v: &Value, id: &str) -> usize {
    v["arms"]
        .as_array()
        .unwrap()
        .iter()
        .position(|a| a["id"] == id)
        .unwrap()
}

fn run_bin(root: &Path, args: &[&str]) -> (i32, Vec<String>) {
    let out = Command::new(env!("CARGO_BIN_EXE_bench_verify_prereg"))
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .expect("run bench_verify_prereg");
    let lines = String::from_utf8(out.stdout)
        .expect("utf-8 output")
        .lines()
        .map(str::to_string)
        .collect();
    (out.status.code().expect("exit code"), lines)
}

#[test]
fn committed_prereg_passes() {
    assert_eq!(defects(&committed()), Vec::<String>::new());
    let (code, lines) = run_bin(Path::new(ROOT), &[]);
    assert_eq!(lines, Vec::<String>::new());
    assert_eq!(code, 0);
}

#[test]
fn refuses_missing_baseline_before_eval() {
    let mut v = committed();
    v.as_object_mut().unwrap().remove("baseline_before_eval");
    expect(
        &v,
        &["baseline_before_eval: missing: baselines must be run and frozen before any evaluation"],
    );
}

#[test]
fn refuses_baseline_not_required_or_incomplete() {
    let mut v = committed();
    v["baseline_before_eval"]["required"] = json!(false);
    v["baseline_before_eval"]["baseline_arms"] = json!(["rules_engine", "si_experience_only"]);
    expect(
        &v,
        &[
            "baseline_before_eval.baseline_arms: baseline arm \"frozen_pretrained_small\" is not listed",
            "baseline_before_eval.baseline_arms: baseline arm \"retrieval_only\" is not listed",
            "baseline_before_eval.baseline_arms: baseline arm \"teacher_27b\" is not listed",
            "baseline_before_eval.baseline_arms: baseline arm \"teacher_35b\" is not listed",
            "baseline_before_eval.baseline_arms[1]: arm \"si_experience_only\" does not have role baseline",
            "baseline_before_eval.required: must be true",
        ],
    );
}

#[test]
fn refuses_eval_or_training_before_baseline() {
    let mut v = committed();
    v["phases"] = json!([
        "preregistration_freeze",
        "candidate_training",
        "sealed_eval",
        "baseline_run"
    ]);
    expect(
        &v,
        &[
            "phases: baseline_run must come before sealed_eval",
            "phases: candidate_training must come after baseline_run",
        ],
    );
    v["phases"] = json!(["preregistration_freeze", "sealed_eval"]);
    expect(&v, &["phases: missing baseline_run"]);
}

#[test]
fn refuses_missing_or_empty_g_background() {
    let mut v = committed();
    v["turing_profile"]["G"]
        .as_object_mut()
        .unwrap()
        .remove("background");
    expect(
        &v,
        &["turing_profile.G.background: missing: G must declare every adviser, model and toolchain"],
    );
    v["turing_profile"].as_object_mut().unwrap().remove("G");
    expect(
        &v,
        &["turing_profile.G.background: missing: G must declare every adviser, model and toolchain"],
    );
    v["turing_profile"]["G"] = json!({"version": "g/1", "background": []});
    expect(
        &v,
        &["turing_profile.G.background: empty: G must declare every adviser, model and toolchain"],
    );
}

#[test]
fn refuses_adviser_missing_from_g() {
    let mut v = committed();
    let bg = v["turing_profile"]["G"]["background"]
        .as_array_mut()
        .unwrap();
    bg.retain(|e| e["id"] != "model_35b");
    let mut want: Vec<String> = [
        "teacher_35b",
        "multi_teacher_verified",
        "fusion_distilled_student",
        "si_plus_teacher",
    ]
    .iter()
    .map(|id| {
        format!(
            "arms[{}].background_refs: \"model_35b\" is not declared in turing_profile.G.background",
            arm_index(&v, id)
        )
    })
    .collect();
    want.sort();
    let want: Vec<&str> = want.iter().map(String::as_str).collect();
    expect(&v, &want);

    let mut v = committed();
    let i = arm_index(&v, "teacher_27b");
    v["arms"][i]["background_refs"] = json!([]);
    let line = format!(
        "arms[{i}].background_refs: arm \"teacher_27b\" is pretrained or uses a teacher but declares no background"
    );
    expect(&v, &[line.as_str()]);
}

#[test]
fn refuses_tj_without_measured_energy() {
    let mut v = committed();
    v["turing_profile"]["tj"]["allowed"] = json!(true);
    expect(
        &v,
        &["turing_profile.tj.allowed: T/J requires turing_profile.energy.status MEASURED"],
    );
    v["turing_profile"]["energy"]["status"] = json!("MEASURED");
    expect(&v, &[]);
}

#[test]
fn frozen_prereg_needs_pinned_background() {
    let mut v = committed();
    v["status"] = json!("FROZEN");
    let pending: Vec<usize> = v["turing_profile"]["G"]["background"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, e)| e["revision"] == "PENDING_PIN")
        .map(|(i, _)| i)
        .collect();
    assert!(
        !pending.is_empty(),
        "the SCOPED manifest still has unpinned entries"
    );
    let lines: Vec<String> = pending
        .iter()
        .map(|i| {
            format!(
                "turing_profile.G.background[{i}].revision: PENDING_PIN is allowed only while status is SCOPED"
            )
        })
        .collect();
    let want: Vec<&str> = lines.iter().map(String::as_str).collect();
    expect(&v, &want);
    for i in &pending {
        v["turing_profile"]["G"]["background"][*i]["revision"] =
            json!(format!("sha256:{}", "0".repeat(64)));
    }
    expect(&v, &[]);
    v["turing_profile"]["G"]["background"][0]["revision"] = json!("main");
    expect(
        &v,
        &["turing_profile.G.background[0].revision: \"main\" is not a commit id or sha256:<64 hex>"],
    );
}

#[test]
fn refuses_underpowered_or_inconsistent_sample_size() {
    let mut v = committed();
    let s = &mut v["sample_size"]["studies"][0];
    s["tasks_planned"] = json!(40);
    s["units_planned"] = json!(480);
    s["effective_n_planned"] = json!(309);
    expect(
        &v,
        &["sample_size.studies[0].effective_n_planned: underpowered: 309 < exact_min_effective_n 381"],
    );
    let s = &mut v["sample_size"]["studies"][0];
    s["design_effect"] = json!(1.0);
    s["alpha_per_contrast"] = json!(0.05);
    expect(
        &v,
        &[
            "sample_size.studies[0].alpha_per_contrast: 0.05 * 4 contrasts exceeds alpha_familywise 0.05",
            "sample_size.studies[0].design_effect: 1 != 1 + (units_per_cluster - 1) * icc_assumed = 1.55",
            "sample_size.studies[0].effective_n_planned: underpowered: 309 < exact_min_effective_n 381",
        ],
    );
}

#[test]
fn refuses_unsealed_heldout_and_placeholders() {
    let mut v = committed();
    v["heldout"]["sealing"]["prediction_commit_before_reveal"] = json!(false);
    v["stopping_rules"] = json!([]);
    v["arms"][0]["description"] = json!("TBD");
    expect(
        &v,
        &[
            "arms[0].description: placeholder value \"TBD\"",
            "heldout.sealing.prediction_commit_before_reveal: must be true",
            "stopping_rules: must be a non-empty array",
        ],
    );
}

#[test]
fn binary_refuses_bad_manifest_file() {
    let dir = std::env::temp_dir().join(format!("bench_verify_prereg_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mut v = committed();
    v.as_object_mut().unwrap().remove("baseline_before_eval");
    v["turing_profile"]["G"]["background"] = json!([]);
    fs::write(dir.join("bad.json"), serde_json::to_string(&v).unwrap()).unwrap();
    let (code, lines) = run_bin(&dir, &["bad.json"]);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        lines,
        vec![
            "bad.json: baseline_before_eval: missing: baselines must be run and frozen before any evaluation",
            "bad.json: turing_profile.G.background: empty: G must declare every adviser, model and toolchain",
        ]
    );
    assert_eq!(code, 1);
}

#[test]
fn binary_refuses_empty_target() {
    let dir =
        std::env::temp_dir().join(format!("bench_verify_prereg_empty_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let (code, lines) = run_bin(&dir, &["."]);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(lines, Vec::<String>::new());
    assert_eq!(code, 1, "no manifest checked is not a pass");
}
