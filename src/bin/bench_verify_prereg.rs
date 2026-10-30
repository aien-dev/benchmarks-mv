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

//! Offline preregistration check: validates `aien-bench-prereg/1` manifests before any run.
//!
//! Usage: `bench_verify_prereg [--root <repo>] [PATH...]` (default path: `manifests/prereg`).
//! Prints one `path: field: reason` line per defect on stdout and exits 1 when there is any or
//! when no manifest was found; exits 0 when every manifest passes. Reads files only: no
//! network, no GPU, no live server.

use aien_benchmarks::prereg::verify_prereg_paths;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bench_verify_prereg")]
#[command(about = "Validate aien-bench-prereg/1 preregistration manifests (offline)")]
struct Args {
    /// Repository root that paths are relative to
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Files or directories to check, relative to the root
    #[arg(default_values_t = ["manifests/prereg".to_string()])]
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let report = verify_prereg_paths(&args.root, &args.paths);
    for note in &report.notes {
        eprintln!("bench_verify_prereg: {note}");
    }
    for d in &report.defects {
        println!("{d}");
    }
    eprintln!(
        "bench_verify_prereg: {} manifests, {} defects",
        report.manifests,
        report.defects.len()
    );
    if report.manifests == 0 || !report.defects.is_empty() {
        std::process::exit(1);
    }
}
