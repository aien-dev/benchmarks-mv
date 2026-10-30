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

//! Offline checks for benchmark preregistration manifests (`aien-bench-prereg/1`).
//!
//! A preregistration fixes arms, environments, the sealed split, metrics, sample sizes,
//! stopping rules and the Turing profile before any candidate is trained. It is a plan, not
//! evidence: it carries no measurements. Rules, one [`Defect`] per violation:
//! - `schema`, `prereg_id`, `status` ([`PREREG_STATUSES`]) and `results_status`
//!   ([`RUN_STATUSES`]) are present; no string anywhere is a placeholder;
//! - `arms` have unique ids, a role in [`ARM_ROLES`], a boolean `pretrained`, and include at
//!   least one baseline, one candidate and one control;
//! - `baseline_before_eval.required` is `true`, its `baseline_arms` are exactly the arms whose
//!   role is `baseline`, and `phases` runs `preregistration_freeze` before `baseline_run`
//!   before `sealed_eval` (with `candidate_training`, if present, after `baseline_run` and
//!   before `sealed_eval`);
//! - `turing_profile.G.background` is a non-empty list of `{id, kind, owner, revision}`;
//!   every arm that is pretrained or uses a teacher names its background, and every
//!   `background_refs` entry is declared in `G`; [`PENDING_PIN`] revisions are allowed only
//!   while `status` is `SCOPED`, otherwise a revision is a commit id or `sha256:<64 hex>`;
//! - `turing_profile` declares `B` (a baseline arm), a blinded `D`, all five `P` fields,
//!   `T_ideal` and `T_real` separately, and `tj.allowed` is `true` only when
//!   `energy.status == "MEASURED"`;
//! - `heldout.sealing` names a mechanism and sets `prediction_commit_before_reveal`,
//!   `judge_independent` and `single_use` to `true`;
//! - every `sample_size.studies` entry is arithmetically consistent: `units_planned =
//!   tasks_planned * units_per_cluster`, `design_effect = 1 + (units_per_cluster - 1) * icc`,
//!   `effective_n_planned = floor(units_planned / design_effect)` and at least
//!   `exact_min_effective_n`, `exact_power_at_planned >= power_target`, and
//!   `alpha_per_contrast * confirmatory_contrasts <= alpha_familywise`;
//! - `stopping_rules` is a non-empty list of `{id, trigger, action}`; every `claims` status is
//!   in [`RUN_STATUSES`].

use crate::evidence::Defect;
use crate::evidence::{collect_strings, display_path, get, is_commit, is_placeholder, push, walk};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The preregistration contract this module validates.
pub const PREREG_SCHEMA: &str = "aien-bench-prereg/1";

/// Lifecycle of the preregistration itself.
pub const PREREG_STATUSES: [&str; 3] = ["SCOPED", "FROZEN", "SUPERSEDED"];

/// Status of the campaign and of each claim.
pub const RUN_STATUSES: [&str; 5] = ["NOT_RUN", "BLOCKED", "INCONCLUSIVE", "FAIL", "PASS"];

/// Arm roles.
pub const ARM_ROLES: [&str; 3] = ["baseline", "candidate", "control"];

/// A background revision not yet bound; allowed only while `status` is `SCOPED`.
pub const PENDING_PIN: &str = "PENDING_PIN";

const PHASE_FREEZE: &str = "preregistration_freeze";
const PHASE_BASELINE: &str = "baseline_run";
const PHASE_TRAINING: &str = "candidate_training";
const PHASE_EVAL: &str = "sealed_eval";

const P_FIELDS: [&str; 5] = [
    "encoding",
    "baseline",
    "sampling",
    "energy",
    "statistical_test",
];

const STUDY_NUMBERS: [&str; 12] = [
    "alpha_familywise",
    "confirmatory_contrasts",
    "alpha_per_contrast",
    "power_target",
    "exact_min_effective_n",
    "tasks_planned",
    "units_per_cluster",
    "icc_assumed",
    "design_effect",
    "units_planned",
    "effective_n_planned",
    "exact_power_at_planned",
];

const EPS: f64 = 1e-9;

/// Result of a run over one or more paths.
#[derive(Debug, Default)]
pub struct PreregReport {
    pub defects: Vec<Defect>,
    pub manifests: usize,
    /// Informational lines (for example a target that does not exist).
    pub notes: Vec<String>,
}

/// Validate every JSON file under each target (a file or directory relative to `root`).
pub fn verify_prereg_paths(root: &Path, targets: &[String]) -> PreregReport {
    let mut report = PreregReport::default();
    for target in targets {
        let clean = target.trim_end_matches('/');
        let full = root.join(clean);
        if !full.exists() {
            report
                .notes
                .push(format!("{clean}: skipped: not present under the root"));
            continue;
        }
        let mut files = Vec::new();
        if full.is_dir() {
            walk(&full, &mut files);
        } else {
            files.push(full.clone());
        }
        files.sort();
        for file in files
            .iter()
            .filter(|f| f.extension().is_some_and(|e| e == "json"))
        {
            let display = display_path(root, file);
            let value: Value = match fs::read_to_string(file)
                .map_err(|e| format!("cannot read: {e}"))
                .and_then(|t| serde_json::from_str(&t).map_err(|e| format!("invalid JSON: {e}")))
            {
                Ok(v) => v,
                Err(reason) => {
                    push(&mut report.defects, &display, "(file)", reason);
                    continue;
                }
            };
            report.manifests += 1;
            let mut found = Vec::new();
            check_prereg(&display, &value, &mut found);
            found.sort();
            found.dedup();
            report.defects.extend(found);
        }
    }
    report
}

/// Validate one parsed preregistration. `path` is only used for the defect lines.
pub fn check_prereg(path: &str, v: &Value, out: &mut Vec<Defect>) {
    let mut d = |field: &str, reason: String| push(out, path, field, reason);
    let Some(top) = v.as_object() else {
        d("(file)", "top level is not a JSON object".into());
        return;
    };
    match top.get("schema").and_then(Value::as_str) {
        None => d("schema", "missing".into()),
        Some(s) if s != PREREG_SCHEMA => d("schema", format!("\"{s}\" is not {PREREG_SCHEMA}")),
        Some(_) => {}
    }
    if top.get("prereg_id").and_then(Value::as_str).is_none() {
        d("prereg_id", "missing".into());
    }
    let status = enum_field(top, "status", &PREREG_STATUSES, &mut d);
    enum_field(top, "results_status", &RUN_STATUSES, &mut d);

    let mut strings = Vec::new();
    collect_strings(v, String::new(), &mut strings);
    for (field, s) in &strings {
        if is_placeholder(s) {
            d(field, format!("placeholder value \"{s}\""));
        }
    }

    let arms = check_arms(top, &mut d);
    check_baseline_before_eval(top, &arms, &mut d);
    check_turing(top, status, &arms, &mut d);
    check_sealing(top, &mut d);
    check_sample_size(top, &mut d);
    check_stopping_rules(top, &mut d);
    if let Some(claims) = top.get("claims") {
        match claims.as_array() {
            Some(list) => {
                for (i, c) in list.iter().enumerate() {
                    match c.get("status").and_then(Value::as_str) {
                        Some(s) if RUN_STATUSES.contains(&s) => {}
                        Some(s) => d(
                            &format!("claims[{i}].status"),
                            format!("\"{s}\" is not one of {}", RUN_STATUSES.join(", ")),
                        ),
                        None => d(&format!("claims[{i}].status"), "missing".into()),
                    }
                }
            }
            None => d("claims", "must be an array".into()),
        }
    }
}

/// One declared arm.
struct Arm {
    id: String,
    role: Option<String>,
    needs_background: bool,
    refs: Vec<String>,
}

fn check_arms(top: &Map<String, Value>, d: &mut impl FnMut(&str, String)) -> Vec<Arm> {
    let Some(list) = top.get("arms").and_then(Value::as_array) else {
        d("arms", "missing or not an array".into());
        return Vec::new();
    };
    if list.is_empty() {
        d("arms", "must declare at least one arm".into());
    }
    let mut arms = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, a) in list.iter().enumerate() {
        let at = |f: &str| format!("arms[{i}].{f}");
        let Some(id) = a.get("id").and_then(Value::as_str) else {
            d(&at("id"), "missing".into());
            continue;
        };
        if !seen.insert(id.to_string()) {
            d(&at("id"), format!("duplicate arm id \"{id}\""));
        }
        let role = match a.get("role").and_then(Value::as_str) {
            Some(r) if ARM_ROLES.contains(&r) => Some(r.to_string()),
            Some(r) => {
                d(
                    &at("role"),
                    format!("\"{r}\" is not one of {}", ARM_ROLES.join(", ")),
                );
                None
            }
            None => {
                d(&at("role"), "missing".into());
                None
            }
        };
        let pretrained = match a.get("pretrained") {
            Some(Value::Bool(b)) => *b,
            Some(_) => {
                d(&at("pretrained"), "must be true or false".into());
                false
            }
            None => {
                d(&at("pretrained"), "missing".into());
                false
            }
        };
        let teacher = a.get("uses_teacher").and_then(Value::as_bool) == Some(true);
        let refs = match a.get("background_refs") {
            None => Vec::new(),
            Some(Value::Array(r)) => r
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            Some(_) => {
                d(&at("background_refs"), "must be an array of strings".into());
                Vec::new()
            }
        };
        arms.push(Arm {
            id: id.to_string(),
            role,
            needs_background: pretrained || teacher,
            refs,
        });
    }
    for role in ARM_ROLES {
        if !list.is_empty() && !arms.iter().any(|a| a.role.as_deref() == Some(role)) {
            d("arms", format!("no arm has role {role}"));
        }
    }
    arms
}

fn check_baseline_before_eval(
    top: &Map<String, Value>,
    arms: &[Arm],
    d: &mut impl FnMut(&str, String),
) {
    match top.get("baseline_before_eval") {
        None => d(
            "baseline_before_eval",
            "missing: baselines must be run and frozen before any evaluation".into(),
        ),
        Some(b) => {
            if b.get("required").and_then(Value::as_bool) != Some(true) {
                d("baseline_before_eval.required", "must be true".into());
            }
            match b.get("baseline_arms").and_then(Value::as_array) {
                Some(list) if !list.is_empty() => {
                    let named: BTreeSet<&str> = list.iter().filter_map(Value::as_str).collect();
                    for (i, x) in list.iter().enumerate() {
                        let field = format!("baseline_before_eval.baseline_arms[{i}]");
                        match x.as_str() {
                            None => d(&field, "must be a string".into()),
                            Some(id) => match arms.iter().find(|a| a.id == id) {
                                None => d(&field, format!("\"{id}\" is not a declared arm")),
                                Some(a) if a.role.as_deref() != Some("baseline") => {
                                    d(&field, format!("arm \"{id}\" does not have role baseline"))
                                }
                                Some(_) => {}
                            },
                        }
                    }
                    for a in arms {
                        if a.role.as_deref() == Some("baseline") && !named.contains(a.id.as_str()) {
                            d(
                                "baseline_before_eval.baseline_arms",
                                format!("baseline arm \"{}\" is not listed", a.id),
                            );
                        }
                    }
                }
                _ => d(
                    "baseline_before_eval.baseline_arms",
                    "must be a non-empty array of arm ids".into(),
                ),
            }
        }
    }
    let Some(phases) = top.get("phases").and_then(Value::as_array) else {
        d("phases", "missing or not an array".into());
        return;
    };
    let names: Vec<&str> = phases.iter().filter_map(Value::as_str).collect();
    let pos = |p: &str| names.iter().position(|n| *n == p);
    let (freeze, baseline, eval) = (pos(PHASE_FREEZE), pos(PHASE_BASELINE), pos(PHASE_EVAL));
    for (p, at) in [
        (PHASE_FREEZE, freeze),
        (PHASE_BASELINE, baseline),
        (PHASE_EVAL, eval),
    ] {
        if at.is_none() {
            d("phases", format!("missing {p}"));
        }
    }
    if freeze.is_some_and(|f| f != 0) {
        d("phases", format!("{PHASE_FREEZE} must be the first phase"));
    }
    if let (Some(b), Some(e)) = (baseline, eval) {
        if b > e {
            d(
                "phases",
                format!("{PHASE_BASELINE} must come before {PHASE_EVAL}"),
            );
        }
    }
    if let Some(t) = pos(PHASE_TRAINING) {
        if baseline.is_some_and(|b| t < b) {
            d(
                "phases",
                format!("{PHASE_TRAINING} must come after {PHASE_BASELINE}"),
            );
        }
        if eval.is_some_and(|e| t > e) {
            d(
                "phases",
                format!("{PHASE_TRAINING} must come before {PHASE_EVAL}"),
            );
        }
    }
}

fn check_turing(
    top: &Map<String, Value>,
    status: Option<&str>,
    arms: &[Arm],
    d: &mut impl FnMut(&str, String),
) {
    let Some(t) = top.get("turing_profile") else {
        d("turing_profile", "missing".into());
        return;
    };
    match get(t, "G.background") {
        None => d(
            "turing_profile.G.background",
            "missing: G must declare every adviser, model and toolchain".into(),
        ),
        Some(Value::Array(bg)) if bg.is_empty() => d(
            "turing_profile.G.background",
            "empty: G must declare every adviser, model and toolchain".into(),
        ),
        Some(Value::Array(bg)) => {
            let mut ids = BTreeSet::new();
            for (i, e) in bg.iter().enumerate() {
                let at = |f: &str| format!("turing_profile.G.background[{i}].{f}");
                for f in ["id", "kind", "owner", "revision"] {
                    if e.get(f).and_then(Value::as_str).is_none() {
                        d(&at(f), "missing".into());
                    }
                }
                if let Some(id) = e.get("id").and_then(Value::as_str) {
                    if !ids.insert(id.to_string()) {
                        d(&at("id"), format!("duplicate background id \"{id}\""));
                    }
                }
                if let Some(r) = e.get("revision").and_then(Value::as_str) {
                    if r == PENDING_PIN {
                        if status != Some("SCOPED") {
                            d(
                                &at("revision"),
                                format!("{PENDING_PIN} is allowed only while status is SCOPED"),
                            );
                        }
                    } else if !is_pinned(r) {
                        d(
                            &at("revision"),
                            format!("\"{r}\" is not a commit id or sha256:<64 hex>"),
                        );
                    }
                }
            }
            for (i, a) in arms.iter().enumerate() {
                if a.needs_background && a.refs.is_empty() {
                    d(
                        &format!("arms[{i}].background_refs"),
                        format!(
                            "arm \"{}\" is pretrained or uses a teacher but declares no background",
                            a.id
                        ),
                    );
                }
                for r in &a.refs {
                    if !ids.contains(r) {
                        d(
                            &format!("arms[{i}].background_refs"),
                            format!("\"{r}\" is not declared in turing_profile.G.background"),
                        );
                    }
                }
            }
        }
        Some(_) => d("turing_profile.G.background", "must be an array".into()),
    }
    match get(t, "B.arm").and_then(Value::as_str) {
        None => d("turing_profile.B.arm", "missing".into()),
        Some(b) => {
            if !arms
                .iter()
                .any(|a| a.id == b && a.role.as_deref() == Some("baseline"))
            {
                d(
                    "turing_profile.B.arm",
                    format!("\"{b}\" is not a declared baseline arm"),
                );
            }
        }
    }
    if get(t, "D.blinded").and_then(Value::as_bool) != Some(true) {
        d("turing_profile.D.blinded", "must be true".into());
    }
    for f in P_FIELDS {
        if get(t, &format!("P.{f}")).and_then(Value::as_str).is_none() {
            d(&format!("turing_profile.P.{f}"), "missing".into());
        }
    }
    for f in ["T_ideal", "T_real"] {
        if get(t, &format!("{f}.definition"))
            .and_then(Value::as_str)
            .is_none()
        {
            d(&format!("turing_profile.{f}.definition"), "missing".into());
        }
    }
    match get(t, "tj.allowed") {
        Some(Value::Bool(true)) => {
            if get(t, "energy.status").and_then(Value::as_str) != Some("MEASURED") {
                d(
                    "turing_profile.tj.allowed",
                    "T/J requires turing_profile.energy.status MEASURED".into(),
                );
            }
        }
        Some(Value::Bool(false)) => {}
        _ => d("turing_profile.tj.allowed", "must be true or false".into()),
    }
}

fn check_sealing(top: &Map<String, Value>, d: &mut impl FnMut(&str, String)) {
    let Some(s) = top.get("heldout").and_then(|h| h.get("sealing")) else {
        d("heldout.sealing", "missing".into());
        return;
    };
    if s.get("mechanism").and_then(Value::as_str).is_none() {
        d("heldout.sealing.mechanism", "missing".into());
    }
    for f in [
        "prediction_commit_before_reveal",
        "judge_independent",
        "single_use",
    ] {
        if s.get(f).and_then(Value::as_bool) != Some(true) {
            d(&format!("heldout.sealing.{f}"), "must be true".into());
        }
    }
}

fn check_sample_size(top: &Map<String, Value>, d: &mut impl FnMut(&str, String)) {
    let Some(s) = top.get("sample_size") else {
        d("sample_size", "missing".into());
        return;
    };
    if s.get("computed_with").and_then(Value::as_str).is_none() {
        d("sample_size.computed_with", "missing".into());
    }
    let studies = match s.get("studies").and_then(Value::as_array) {
        Some(l) if !l.is_empty() => l,
        _ => {
            d("sample_size.studies", "must be a non-empty array".into());
            return;
        }
    };
    for (i, st) in studies.iter().enumerate() {
        let at = |f: &str| format!("sample_size.studies[{i}].{f}");
        let mut n = std::collections::BTreeMap::new();
        for f in STUDY_NUMBERS {
            match st.get(f).and_then(Value::as_f64).filter(|x| x.is_finite()) {
                Some(x) => {
                    n.insert(f, x);
                }
                None => d(&at(f), "missing or not a finite number".into()),
            }
        }
        if n.len() != STUDY_NUMBERS.len() {
            continue;
        }
        if (n["tasks_planned"] * n["units_per_cluster"] - n["units_planned"]).abs() > EPS {
            d(
                &at("units_planned"),
                format!(
                    "{} != tasks_planned * units_per_cluster = {}",
                    n["units_planned"],
                    n["tasks_planned"] * n["units_per_cluster"]
                ),
            );
        }
        let deff = 1.0 + (n["units_per_cluster"] - 1.0) * n["icc_assumed"];
        if (deff - n["design_effect"]).abs() > EPS {
            d(
                &at("design_effect"),
                format!(
                    "{} != 1 + (units_per_cluster - 1) * icc_assumed = {deff}",
                    n["design_effect"]
                ),
            );
        }
        let eff = (n["units_planned"] / deff).floor();
        if (eff - n["effective_n_planned"]).abs() > EPS {
            d(
                &at("effective_n_planned"),
                format!(
                    "{} != floor(units_planned / design_effect) = {eff}",
                    n["effective_n_planned"]
                ),
            );
        }
        if eff < n["exact_min_effective_n"] {
            d(
                &at("effective_n_planned"),
                format!(
                    "underpowered: {eff} < exact_min_effective_n {}",
                    n["exact_min_effective_n"]
                ),
            );
        }
        if n["exact_power_at_planned"] < n["power_target"] {
            d(
                &at("exact_power_at_planned"),
                format!(
                    "{} < power_target {}",
                    n["exact_power_at_planned"], n["power_target"]
                ),
            );
        }
        if n["alpha_per_contrast"] * n["confirmatory_contrasts"] > n["alpha_familywise"] + EPS {
            d(
                &at("alpha_per_contrast"),
                format!(
                    "{} * {} contrasts exceeds alpha_familywise {}",
                    n["alpha_per_contrast"], n["confirmatory_contrasts"], n["alpha_familywise"]
                ),
            );
        }
    }
}

fn check_stopping_rules(top: &Map<String, Value>, d: &mut impl FnMut(&str, String)) {
    match top.get("stopping_rules").and_then(Value::as_array) {
        Some(list) if !list.is_empty() => {
            for (i, r) in list.iter().enumerate() {
                for f in ["id", "trigger", "action"] {
                    if r.get(f).and_then(Value::as_str).is_none() {
                        d(&format!("stopping_rules[{i}].{f}"), "missing".into());
                    }
                }
            }
        }
        _ => d("stopping_rules", "must be a non-empty array".into()),
    }
}

fn enum_field<'a>(
    top: &'a Map<String, Value>,
    field: &str,
    allowed: &[&str],
    d: &mut impl FnMut(&str, String),
) -> Option<&'a str> {
    match top.get(field) {
        None => {
            d(field, "missing".into());
            None
        }
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Some(s.as_str()),
        Some(Value::String(s)) => {
            d(
                field,
                format!("\"{s}\" is not one of {}", allowed.join(", ")),
            );
            None
        }
        Some(_) => {
            d(field, "must be a string".into());
            None
        }
    }
}

/// A 40-character commit id or `sha256:` followed by 64 lowercase hex characters.
fn is_pinned(r: &str) -> bool {
    is_commit(r)
        || r.strip_prefix("sha256:").is_some_and(|h| {
            h.len() == 64 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
}
