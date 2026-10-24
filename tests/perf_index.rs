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

//! The committed `docs/PERF_INDEX.md` must be exactly what the generator produces from this tree.

use aien_benchmarks::perf_index::{generate, INDEX_PATH};
use std::path::Path;
use std::process::Command;

#[test]
fn committed_index_matches_generator() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed = std::fs::read_to_string(root.join(INDEX_PATH)).expect("docs/PERF_INDEX.md");
    let fresh = generate(root);
    assert!(
        committed == fresh,
        "{INDEX_PATH} is out of date; run: cargo run --release --bin bench_index"
    );
    // Deterministic: a second run gives the same bytes.
    assert_eq!(fresh, generate(root));
}

#[test]
fn check_mode_agrees() {
    let status = Command::new(env!("CARGO_BIN_EXE_bench_index"))
        .arg("--root")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--check")
        .status()
        .expect("run bench_index");
    assert!(status.success());
}
