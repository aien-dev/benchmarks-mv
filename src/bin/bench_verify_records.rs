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

//! Offline evidence-integrity check: validates benchmark records and SHA256SUMS artifact bundles.
//!
//! Usage: `bench_verify_records [--root <repo>] [PATH...]` (default paths: `records`, `artifacts`).
//! Prints one `path: field: reason` line per defect on stdout and exits 1 when there is any;
//! exits 0 when every file passes. Reads files only: no network, no GPU, no live server.

use aien_benchmarks::evidence::verify_paths;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bench_verify_records")]
#[command(about = "Validate aien-bench-record/1 records and SHA256SUMS bundles (offline)")]
struct Args {
    /// Repository root that evidence paths are relative to
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Files or directories to check, relative to the root
    #[arg(default_values_t = ["records".to_string(), "artifacts".to_string()])]
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let report = verify_paths(&args.root, &args.paths);
    for note in &report.notes {
        eprintln!("bench_verify_records: {note}");
    }
    for d in &report.defects {
        println!("{d}");
    }
    eprintln!(
        "bench_verify_records: {} records, {} bundles ({} files digest-checked), {} defects",
        report.records,
        report.bundles,
        report.bundle_files,
        report.defects.len()
    );
    if !report.defects.is_empty() {
        std::process::exit(1);
    }
}
