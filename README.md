# AIEN Benchmarks

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

This repository holds AIEN's performance measurements and the Rust tools that take and check them. Every measurement in it comes from one machine, the NVIDIA DGX Spark (Grace Blackwell GB10, aarch64). It has two parts: the Prime Drag Race suite runner in [prime-drag-race/](prime-drag-race/CONTRACT.md), which builds on its own, and the main measurement harness (the `aien-benchmarks` crate at the root), which needs a sibling checkout to build (see [Build and test](#build-and-test)).

AIEN does not compare itself with other people's systems. Speed is measured against the machine's physical limit and our own previous run, and correctness against our own reference.

## Where this fits in AIEN

[AienOS](https://github.com/aien-dev/aienix) is the core tree of AIEN. Its README lists this repository as an attachment: a repository that lives on its own and plugs into the core at a boundary. Related repositories:

- [aien-sovereign-core-mv](https://github.com/aien-dev/aien-sovereign-core-mv): host runtime and the Spark verification pipeline that tests this repository. Its [Measured results](https://github.com/aien-dev/aien-sovereign-core-mv#measured-results) section sets the evidence standard used here.
- [aien-architecture-mv](https://github.com/aien-dev/aien-architecture-mv): architecture and plans. Issue [#233](https://github.com/aien-dev/aien-architecture-mv/issues/233) (performance intelligence) is the plan behind the streaming and evidence work below.
- [omega-mv](https://github.com/aien-dev/omega-mv): the language and GPU engine that the Prime Drag Race native GB10 entry runs on.
- [aien-receipts](https://github.com/aien-dev/aien-receipts): where the Spark publishes the signed test bundle for each tested commit of this repository.

## Status (2026-10-19)

- **One evidence-backed result:** the [Prime Drag Race campaign of 5 October 2026](prime-drag-race/CAMPAIGN-2026-10-05.md) (details below).
- **Older figures are withdrawn.** Everything under [Historical record](#historical-record-withdrawn-figures) predates the evidence standard and is kept only as a record.
- **Merged since the last README update (2026-10-16 to 2026-10-18):** the streaming benchmark now counts tokens only from the server's own report and writes structured records (#25); an offline evidence check and a generated index, [docs/PERF_INDEX.md](docs/PERF_INDEX.md), were added (#24); rustls was updated for security advisories (#27); the hosted GitHub test workflows were removed, so the Spark is now the only test judge (#33).
- **Fixed on 2026-10-19:** `docs/PERF_INDEX.md` had fallen behind the source files, so `cargo test` failed 2 tests in `tests/perf_index.rs` and the BENCH-PERF-INDEX gate failed. #34 committed the regenerated index.
- **The main harness does not build from public sources.** Its sibling dependency is a branch that is not on GitHub (see [Build and test](#build-and-test)).
- **Planned, not started in this repository:** open tracking issues [#29](https://github.com/aien-dev/benchmarks-mv/issues/29) and [#30](https://github.com/aien-dev/benchmarks-mv/issues/30) (GB10 expert streaming, "Ribbon"), [#31](https://github.com/aien-dev/benchmarks-mv/issues/31) (Spark, iMac and M2 compute across machines) and [#32](https://github.com/aien-dev/benchmarks-mv/issues/32) (small-model training across those machines). None of them has results yet.

## Current result: Prime Drag Race

On the DGX Spark, the native GB10 prime sieve ran at 19,701.34 passes per second, 20.3x the frozen native baseline of 971.48 passes per second, with correctness checked on the chip against our own reference. Source: receipt `1a3eafe9`, recorded in [CAMPAIGN-2026-10-05.md](prime-drag-race/CAMPAIGN-2026-10-05.md), with the run files under [prime-drag-race/evidence/](prime-drag-race/evidence). The frozen baseline run is in [RESULTS-2026-10-05.md](prime-drag-race/RESULTS-2026-10-05.md). That run was on Linux on the GB10, not on AIENOS. How close this is to the hardware limit for this workload has not been measured.

The rules of the race (the frozen contract) are in [CONTRACT.md](prime-drag-race/CONTRACT.md).

## What is in this repository

| Path | What it is |
| :--- | :--- |
| `prime-drag-race/` | Prime Drag Race suite runner (`prime_race`), contract, results and sealed evidence. A standalone Rust crate. |
| `src/` | The `aien-benchmarks` measurement harness and five helper programs: `bench_canonical_suite`, `bench_branching_neutral`, `bench_verify_records`, `bench_verify_prereg`, `bench_index`. |
| `data/` | Datasets from the withdrawn 2026-09-19 sweep, kept as recorded. |
| `artifacts/gb10_canonical_1789907893_4d762/` | One withdrawn GB10 run bundle with its `SHA256SUMS` file. |
| `assets/` | SVG charts drawn from `data/`. |
| `manifests/` | Workload manifests read by `bench_branching_neutral` and `bench_canonical_suite`; `manifests/prereg/` holds preregistrations checked by `bench_verify_prereg`. |
| `docs/` | [PIPELINE.md](docs/PIPELINE.md) (how pull requests are tested and merged), [PERF_INDEX.md](docs/PERF_INDEX.md) (generated index), [LIVE_PRESSURE_BENCHMARK.md](docs/LIVE_PRESSURE_BENCHMARK.md) (an older pressure benchmark write-up, not regenerated under the evidence standard). |
| `tests/gates/` | The Spark test gates (see [How changes are tested](#how-changes-are-tested)). |
| `tests/fixtures/records/` | Good and bad example records for the evidence check. |
| `scripts/check-no-python.sh` | Fails if any tracked file is Python. |

## Build and test

You need a Rust toolchain. The Spark used `rustc 1.98.1 (48a229cea 2026-09-01)` on 2026-10-19.

### Prime Drag Race runner (builds on its own)

```bash
git clone https://github.com/aien-dev/benchmarks-mv.git
cd benchmarks-mv/prime-drag-race
cargo build
cargo test
cargo run -- --help
```

On the Spark on 2026-10-19 this built, and `cargo test` passed 33 tests with 1 ignored. `prime_race verify` runs the correctness sweep of one implementation, and `prime_race run` runs the timed trials and writes a receipt. Timed runs need the implementations under test, which live in other repositories (see the campaign file).

### Main harness (needs a sibling checkout)

The `aien-benchmarks` crate depends on five crates by path under `../canonical-branch-native-runtime/crates/`: `aien-platform`, `aien-platform-linux`, `aien-kv-cache`, `aien-scheduler` and `aien-inference-abi`. Without that directory, `cargo metadata` fails and nothing builds.

That directory is a checkout of the branch `feat/canonical-branch-native-runtime` of aien-sovereign-core (see the comment in `tests/gates/run.sh`). On 2026-10-19 the branch was only on the Spark's local Forgejo server, not on GitHub. A checkout of aien-sovereign-core-mv `main` (commit `a75b7c2`) in its place is not enough: `bench_canonical_suite` failed to compile against it (missing `BlackwellBatchExecutor`, changed `ModelConfig` fields). So today the main harness only builds on the Spark.

The committed `Cargo.lock` still names `spark-max-rs` and `spark-max-cabi` (the Modular MAX bridge, removed from AIEN on 2026-10-15) because the sibling copy of `aien-scheduler` it was generated against depended on them. No code in this repository uses them.

With the sibling in place, from the repository root:

```bash
# Unit and integration tests
cargo test --verbose

# Check the committed dataset against its budgets (RSS budgets and latency thresholds)
cargo run --release -- verify

# Print a summary table of the committed dataset
cargo run --release -- report

# Check records and artifact bundles offline (one line per defect)
cargo run --release --bin bench_verify_records -- records artifacts tests/fixtures/records/good

# Check preregistration manifests offline (one line per defect)
cargo run --release --bin bench_verify_prereg -- manifests/prereg

# Check that docs/PERF_INDEX.md matches the generator (drop --check to rewrite it)
cargo run --release --bin bench_index -- --check

# Fail if any tracked file is Python
sh scripts/check-no-python.sh
```

On the Spark on 2026-10-19 with the sibling in place: `verify`, `report`, `bench_verify_records` (1 record, 1 bundle, 0 defects) and the Python check passed. `cargo test` and `bench_index --check` failed because of a stale index; after #34 regenerated it, the gate steps `test` and `perf-index` pass on the Spark (2026-10-19).

### How changes are tested

The Spark is the only test judge for this repository. For each pull request head, a worker on the Spark runs the gates in `tests/gates/` that the change touches, plus the always-run gates in `.aien/always`, signs the result and publishes it to [aien-receipts/benchmarks](https://github.com/aien-dev/aien-receipts/tree/main/benchmarks). The one GitHub check, `AIEN Verification` (`.github/workflows/aien-verification.yml`), only checks that signed bundle; it does not build or test this repository's code. The full process, including how a pull request gets merged, is in [docs/PIPELINE.md](docs/PIPELINE.md).

The gates, each a step of `tests/gates/run.sh`:

| Gate | Runs |
| :--- | :--- |
| BENCH-FMT | `cargo fmt -p aien-benchmarks -- --check` |
| BENCH-CHECK | `cargo check --all-targets` |
| BENCH-TEST | `cargo test --verbose` |
| BENCH-VERIFY | `cargo run --release -- verify` |
| BENCH-GENERATE | `cargo run --release -- generate --output assets`, then fails if `assets/` changed |
| BENCH-VERIFY-RECORDS | `bench_verify_records` as above, fails on any defect |
| BENCH-PERF-INDEX | `bench_index`, then fails if `docs/PERF_INDEX.md` changed |
| BENCH-ZERO-DISK-SECRETS | fails if any `.env*` file is found up to 4 directories deep (always runs) |

This repository has no crumb files (`crumb verify .` reports `OK: 0 crumbs current`).

## The measurement harness

The main harness was built for the Spark's live service layout, so some steps only produce data where those services run. It produced the withdrawn figures in the historical record below. The streaming measurement was rewritten on 2026-10-16 (#25) and has not produced any committed result since.

### Full sweep

```bash
cargo run --release -- measure
```

By default `measure` overwrites the committed `data/benchmarks_latest.json` and `assets/*.svg`. Use `--output-json` and `--output-assets` to write elsewhere. It runs these steps:

1. **System discovery:** CPU architecture, memory, kernel (`uname -r`), page size and `rustc --version`.
2. **Standalone server:** starts the harness's own Rust HTTP server on a free port.
3. **HTTP load sweep:** sends concurrent requests to that server's `/api/status`, `/api/query` and `/api/vector` and records latency percentiles and throughput. Defaults: `--concurrency 10`, `--requests 500`, `--warmup 20`.
4. **Memory:** reads resident memory (`VmRSS` in `/proc/<pid>/status`) of the live AIEN services it finds.
5. **Model serving:** streams chat requests to one OpenAI-compatible endpoint (default `127.0.0.1:18006`, model `atlas-lightning-omni`, set with the `--llm-*` flags), and measures the embedding service on port 18081.
6. **Vector math:** times 100,000 dot products of 768-number vectors.
7. **Output:** writes the JSON dataset and redraws the SVG charts.

`--skip-standalone` skips steps 2 and 3, `--skip-live` skips the live-service memory readings, and `--skip-llm` skips the streaming endpoint. All flags: `cargo run --release -- measure --help`.

### Streaming measurement only

```bash
cargo run --release -- llm-stream --llm-host 127.0.0.1 --llm-port 18006 \
  --llm-model <model id> --llm-engine <engine label> --llm-precision <e.g. BF16> \
  --llm-context-length <tokens> --llm-requests 20 --llm-concurrency 1 --out-dir records
```

This writes one `aien-bench-record/1` JSON record and its raw samples file into `--out-dir` (default `records/`, which git ignores). It exits 2 if no request succeeded. How it measures (source: `src/harness/llm_stream.rs`, `src/harness/sse.rs`):

- **TTFT** (time to first token) is the time from just before the request is sent to the first streamed event that carries text. Events with only a role, nothing, or only usage data do not count.
- **Tokens** are counted only from the server's own `usage.completion_tokens`, and the record says `token_accounting: "backend_usage"`. Without it the record says `"events_not_tokens"` and reports events per second and time between events, never tokens per second.
- **Failures:** read errors, timeouts, non-2xx responses, error events, cut-off bodies and streams without `data: [DONE]` count as failed requests and are not measured.
- **Percentiles** use the nearest-rank method over the successful requests, and the count is recorded. A p95 needs at least 20 successful requests (`P95_MIN_SAMPLES`).
- **Labels** in the record (model, engine, precision, context length) come from the flags, not from constants in the code.

### Other commands

```bash
# Redraw the SVG charts from the committed dataset
cargo run --release -- generate --output assets

# Rewrite the benchmark table between the AIEN:BENCHMARKS:START and END markers in a
# markdown file from the dataset. --check exits non-zero instead of writing.
# With no --target it uses ../README.md and ../docs/STATE_OF_AIEN.md.
cargo run --release -- sync-docs --target <file> --check
```

## Evidence integrity

New benchmark records follow the `aien-bench-record/1` format: source commit and whether the tree was dirty, build, workload, hardware, backend, one definition per metric, metrics, evidence file paths and limitations. They live as JSON files under `records/`. The older run bundles under `artifacts/` keep their own format and are checked against their `SHA256SUMS` files, never rewritten.

`cargo run --release --bin bench_verify_records` (default paths `records` and `artifacts`) checks them offline and prints one `path: field: reason` line per defect. It rejects placeholder values, short commit ids, missing evidence files and percentiles from too few samples. `cargo run --release --bin bench_index` regenerates [docs/PERF_INDEX.md](docs/PERF_INDEX.md), a list of records, sources and studies copied from the files; it is an index, not an authority.

Status words in a record (source: `src/evidence.rs`):

- **VERIFIED**: measured from a clean committed build, with evidence files in this repository that anyone can re-check.
- **EXPERIMENTAL**: a real measurement that has not been qualified yet; not a claim.
- **HISTORICAL**: kept as a record of what an earlier run wrote; not a current claim. Fields that run did not record say `not_recorded` and are named in its limitations.
- **NOT_RUN**: a required measurement that has not been performed; it carries no numbers and no evidence.
- **FAIL**: the run happened and did not meet its acceptance check; kept as evidence.
- **SUPERSEDED**: replaced by a newer record.

### Preregistration

A campaign that compares learning arms is declared before any candidate is trained, in an `aien-bench-prereg/1` file under `manifests/prereg/`: arms, environments, the sealed held-out split and how non-access is enforced, metrics, sample sizes with their power calculation, stopping rules and the Turing profile `T(M;B,D,G,P)` with `T_ideal` and `T_real` kept apart. A preregistration is a plan and carries no measurements. `cargo run --release --bin bench_verify_prereg` (default path `manifests/prereg`) prints one `path: field: reason` line per defect. It refuses a file without `baseline_before_eval` (baselines run and frozen before any evaluation), without a non-empty `G` background naming every pretrained adviser and toolchain an arm uses, with T/J allowed while energy is not `MEASURED`, with unpinned background revisions once `FROZEN`, or with a sample size whose design effect, effective n or power does not add up (source: `src/prereg.rs`). [manifests/prereg/si_genesis_e2.json](manifests/prereg/si_genesis_e2.json) is the SI Genesis E2 and Fusion E1 plan for issues #39 and #38; its status is `SCOPED` and every result in it is `NOT_RUN`.

## Hardware

All measurements here come from one NVIDIA DGX Spark (`spark-b87b`), Grace Blackwell GB10, aarch64. The withdrawn dataset `data/benchmarks_latest.json` records kernel `7.0.0-1019-nvidia` and `rustc 1.98.1 (48a229cea 2026-09-01)`. On 2026-10-19 the Spark reported the same kernel and compiler, a 4096-byte page size (`getconf PAGESIZE`) and 121 GiB of memory (`free -g`). Nothing in this repository measures or tests any other platform. Work across other machines is planned in issues #31 and #32 and has no results yet.

---

## Historical record (withdrawn figures)

> Everything in this section was recorded before the evidence standard in [aien-sovereign-core-mv](https://github.com/aien-dev/aien-sovereign-core-mv#measured-results) (this note was first written on 23 September 2026; labels updated 12 October 2026). None of it has been regenerated, and none of it is a current result. A figure is current only when its bundle carries commit identity, a hardware and environment record, the exact command, raw samples, SHA-256 digests, a measurement definition and steps to reproduce it. The files in `data/` are kept as recorded, including service names from before the openclaw-rs to aegis-runtime rename. On 2026-10-15 the Python baseline rows and the Rust-versus-Python records were removed from `data/` under this rule; they remain in git history. The model serving rows were served by Modular MAX, which AIEN removed on 2026-10-15.

### Summary (withdrawn)

1. **Memory:** the Rust services read 4.56 MB to 18.66 MB resident memory, apart from the embedding service, which holds model weights.
2. **HTTP latency and throughput:** p50 time to first byte of 0.26 ms to 0.50 ms at 19,000 to 34,000 requests per second (concurrency 10, 500 requests per endpoint).
3. **Vector math:** 0.58 microseconds per 768-number dot product.
4. **Model serving (Modular MAX):** the GPU seat (`atlas-lightning-omni`, Nemotron 3.5 Lightning 30B) measured 426.9 ms TTFT and 46.9 ms between tokens. The CPU fallback seat (`unsloth/Llama-3.2-1B-Instruct`) measured 155.4 ms TTFT and 88.8 ms between tokens.

### 1. Memory resident set size (withdrawn)

![Memory RSS](assets/memory_comparison.svg)

Read from `/proc/[pid]/status` (`VmRSS`) on live services:

| Service | Built with | Role | Memory RSS |
| :--- | :--- | :--- | :--- |
| **aegis-runtime** | Rust (Axum + SQLite) | Agent runtime | **4.56 MB** |
| **spark-cockpit-rs** | Rust (Axum) | Health and telemetry gateway | **12.90 MB** |
| **cortex-rs** | Rust (Axum + SQLite WAL) | Knowledge graph engine | **18.66 MB** |
| **cortex-encoder-rs** | Rust (ONNX INT8) | Local vector embedding | **777.43 MB** |

### 2. HTTP latency and throughput (withdrawn)

![Gateway Latency](assets/latency_comparison.svg)

Concurrency 10, 500 requests per endpoint on localhost:

| Endpoint | Service | Requests / sec | p50 TTFB | p95 | p99 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **cortex-encoder-rs `/health`** | Rust Axum + ONNX Runtime | **34,684.6** | **0.26 ms** | 0.35 ms | 0.40 ms |
| **cortex-rs `/api/cortex/get`** | Rust Axum + SQLite WAL | **19,366.6** | **0.50 ms** | 0.65 ms | 0.69 ms |
| **spark-cockpit-rs `/api/pulse`** | Rust Axum | **5,840.2** | **1.63 ms** | 2.44 ms | 2.86 ms |

### 3. Model serving and vector math (withdrawn)

These rows came from the old streaming parser, which counted every `data:` line as a token, gave all events from one socket read the same time, treated read errors as end of stream, and reported one TTFT sample as both p50 and p95. The TTFT rows are single samples, so their p50 and p95 are the same value. For the vector row, the p50 is the mean time per iteration over 100,000 iterations and the p95 was that value times 1.3, not a measured percentile. The current harness works differently (see [Streaming measurement only](#streaming-measurement-only)).

| Workload | Model / target | Engine | p50 | p95 | Observed speed |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **TTFT, GPU seat** | `Nemotron 3.5 Lightning 30B` (BF16) | Modular MAX on GB10 GPU | **426.91 ms** | 426.91 ms | |
| **Time between tokens, GPU seat** | `Nemotron 3.5 Lightning 30B` (BF16) | Modular MAX on GB10 GPU | **46.91 ms** | 47.88 ms | about 21.3 tok/s |
| **TTFT, CPU fallback** | `Llama 3.2 1B Instruct` (Q4_K) | Modular MAX on Grace CPU | **155.40 ms** | 155.40 ms | |
| **Time between tokens, CPU fallback** | `Llama 3.2 1B Instruct` (Q4_K) | Modular MAX on Grace CPU | **88.82 ms** | 99.42 ms | about 11.3 tok/s |
| **Vector embedding** | `BAAI/bge-base-en-v1.5` (INT8) | cortex-encoder-rs (ONNX Runtime) | **8.55 ms** | 8.83 ms | |
| **Dot product (768 numbers)** | 100,000 iterations | Rust loop | **0.0006 ms** (0.58 us) | 0.0008 ms | 1.71M ops/sec |

### 4. GB10 run gb10_canonical_1789907893_4d762 (withdrawn)

Recorded on the DGX Spark GB10 (`sm_121`). The run files report `fallback_count: 0` for every step.

#### A. Continuous batching sweep (TinyLlama-1.1B BF16)

| Concurrency | TTFT p50 | ITL p50 | Throughput | Step latency p50 | GPU power | GPU utilization | Active KV blocks | Fallback count |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **C = 1** | 33.39 ms | 22.26 ms | 44.65 tok/s | 22.26 ms | 16.94 W | 10% | 8 | 0 |
| **C = 2** | 31.38 ms | 20.92 ms | 94.87 tok/s | 20.92 ms | 21.29 W | 95% | 16 | 0 |
| **C = 4** | 32.64 ms | 21.76 ms | 46.17 tok/s | 21.76 ms | 17.61 W | 95% | 14 | 0 |
| **C = 8** | 30.42 ms | 20.28 ms | 245.66 tok/s | 20.28 ms | 31.14 W | 12% | 46 | 0 |
| **C = 16** | **35.34 ms** | **23.56 ms** | **553.14 tok/s** | **23.56 ms** | **27.89 W** | 9% | 110 | **0** |
| **C = 32** | 112.76 ms | 75.17 ms | 222.77 tok/s | 75.17 ms | 41.90 W | 96% | 166 | 0 |
| **C = 64** | 152.55 ms | 101.70 ms | 510.16 tok/s | 101.70 ms | 42.52 W | 96% | 416 | 0 |

#### B. Branching (500 branches, 32,768 prefix tokens)

- **Total time to fork 500 branches:** 1.20 ms (1,202.2 µs)
- **Median fork time per branch:** 2.06 µs (p90: 3.18 µs, p99: 5.86 µs)
- **Shared blocks:** 2,048 (reference count 501, no duplicate blocks)
- **KV memory allocated:** 704.00 MB, against 352,000.00 MB (343.75 GB) if every branch had its own copy
- **Memory ratio:** 500.0x
- **Cold fork to first token:** 13.04 µs (0.013 ms)
- **Copy-on-write change:** 13.30 µs (13,297 ns)
- **Fallback count:** 0

#### C. File digests

The run files are under `artifacts/gb10_canonical_1789907893_4d762/`. Their SHA-256 digests match its `SHA256SUMS` file, and `bench_verify_records` digest-checked all 5 on 2026-10-19:

| File | SHA-256 |
| :--- | :--- |
| `manifest.json` | `a78f0ad669b85e92ceffed38e7d36bc55786280b1b4d21d9ae646f1718479816` |
| `summary.json` | `9cf62a25445e643b3f7d0af5bc86217aed2de1067bcb37c6610e653c2d2cedc8` |
| `linear_sweep.json` | `0362ac84d0eb0ddcdc1a877ef449a2030eb59c0d5ed0a621fcc75aac8f661858` |
| `branch_steady.json` | `6cf46288e554c41bc5003fd3e0c7d56e25dbc15466ce8ab67ca4db14f7af8155` |
| `branch_cold.json` | `d09869d5a3ec4365bec4f97f1d8e7ba32ea0d799ea6160bb0aed82b9904fa567` |

The program that produced it is `bench_canonical_suite` (`cargo run --release --bin bench_canonical_suite`). It needs the sibling checkout described in [Build and test](#build-and-test) and the GB10 GPU.

---

## License

Licensed under the Apache License, Version 2.0 (Apache-2.0). See [LICENSE](LICENSE).
Copyright (c) 2026 Drake Stapleton and AIEN Contributors. See [NOTICE](NOTICE) for attribution.

- **Code and harness:** [Apache-2.0](LICENSE).
- **Benchmark data:** the raw JSON telemetry, timing records and SVG charts are dedicated to the public domain under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
- **Development charter:** [CONSTITUTION.md](CONSTITUTION.md) sets the rules for upstream development. It does not bind people who use this code.

Contributing: see [CONTRIBUTING.md](CONTRIBUTING.md). Security reports: see [SECURITY.md](SECURITY.md).
