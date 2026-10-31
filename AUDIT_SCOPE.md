# Audit scope

Written for an outside, independent security audit. Revision described:
`0f369dbc852b2e9364ed55c6e19a7e285cd424c1` on `main`, 2026-10-26. Everything below is taken from the repository's own files at
that revision; items marked NOT_RUN were not executed while writing this document.

## What this repository is

Performance measurements from one NVIDIA DGX Spark (GB10, aarch64) and the Rust tools that take
and check them. Two crates, both Apache-2.0:

- `prime-drag-race/` (`prime-race` 0.1.0): the Prime Drag Race suite runner (`prime_race`) with
  its frozen contract (`CONTRACT.md`), an independent reference, validation, the trial protocol
  and receipts, plus 1,038 sealed evidence files under `prime-drag-race/evidence/`.
- The root crate `aien-benchmarks`: the measurement harness (`src/harness/`), the record format
  and status words (`src/evidence.rs`), and the programs `bench_canonical_suite`,
  `bench_branching_neutral`, `bench_verify_records` and `bench_index`.

`data/` and `artifacts/gb10_canonical_1789907893_4d762/` hold withdrawn historical datasets kept as
recorded.

## What to review, in priority order

1. `src/evidence.rs` and `src/bin/bench_verify_records.rs`: the offline checker for
   `aien-bench-record/1` records and artifact bundles. `tests/evidence_records.rs` with
   `tests/fixtures/records/bad/` (nine malformed records) and `good/` exercises it.
2. `prime-drag-race/src/exec.rs`, `runner.rs`, `validate.rs`, `receipt.rs`: the runner executes
   the implementation binary it is given (`Command::new(imp)` with `--limit` and
   `--audit-passes`), reads its `report.json`, and validates results against `reference.rs`
   before any speed number (`CONTRACT.md`, "Correctness gates").
3. `src/harness/llm_stream.rs` and `src/harness/sse.rs`: streaming client; parses server-sent
   events from the endpoint named by `--llm-host` and `--llm-port`. Tokens are counted only from
   the server's `usage.completion_tokens`.
4. `src/harness/standalone.rs`, `http_bench.rs`: the harness's own HTTP server on
   `127.0.0.1:0`.
5. `src/harness/memory.rs`, `smaps.rs`, `system.rs`: reads `/proc` and runs system commands
   (`uname -r`, `rustc --version`).
6. `src/perf_index.rs`, `src/svg_chart.rs`, `sync-docs`: writers that rewrite tracked files.
   README: `measure` overwrites `data/benchmarks_latest.json` and `assets/*.svg` by default, and
   `sync-docs` with no `--target` writes `../README.md` and `../docs/STATE_OF_AIEN.md` outside this
   repository.

## Trust boundaries

- Untrusted: replies from measured endpoints; implementation binaries and their `report.json`;
  records and bundles given to the checker.
- Trusted: committed evidence and `SHA256SUMS` files once sealed; the sibling crates; the host.
- The harness was built for the Spark's live service layout and reads loopback services on that
  host (README).

## Threat model summary

- Assets: the honesty of published measurements; the integrity of sealed evidence.
- Attackers considered: fabricated or placeholder records; a measurement counted from the wrong
  source (events reported as tokens, p95 from too few samples); an implementation that reports a
  wrong prime count.
- Defended: offline record checker with rejection fixtures, reference validation before timing,
  `SHA256SUMS` on bundles, `.env` refusal gate, no-Python guard.
- Not defended / explicitly out of scope: a hostile implementation binary (it runs with the
  runner's privileges; no sandbox is described); measurements on any platform other than the one
  Spark.

## Build from source at a pin

README: the Spark used `rustc 1.98.1 (48a229cea 2026-09-01)`. `Cargo.lock` committed for both
crates (262 and 69 packages).

```sh
cd prime-drag-race && cargo build && cargo test && cargo run -- --help
```

The root crate depends by path on five crates under `../canonical-branch-native-runtime/crates/`
(`aien-platform`, `aien-platform-linux`, `aien-kv-cache`, `aien-scheduler`, `aien-inference-abi`).
README: that directory is a branch of aien-sovereign-core that was not on GitHub on 2026-10-19, so
the main harness does not build from public sources. An outside auditor can build only
`prime-drag-race/`.

## Run every test

NOT_RUN while writing this document. From README and `tests/gates/run.sh`:

```sh
(cd prime-drag-race && cargo test)
cargo test --verbose                                   # needs the sibling
cargo run --release -- verify
cargo run --release --bin bench_verify_records -- records artifacts tests/fixtures/records/good
cargo run --release --bin bench_index -- --check
sh scripts/check-no-python.sh
tests/gates/run.sh fmt &&
  tests/gates/run.sh check &&
  tests/gates/run.sh test &&
  tests/gates/run.sh verify &&
  tests/gates/run.sh generate &&
  tests/gates/run.sh verify-records &&
  tests/gates/run.sh perf-index &&
  tests/gates/run.sh zero-disk-secrets
```

README reports 33 tests passing with 1 ignored for `prime-drag-race` on 2026-10-19. `#[test]`
attributes in `src/`, `tests/` and `prime-drag-race/` at this revision: 81.

## Known limitations and NOT_RUN

- The main harness does not build from public sources (above).
- `Cargo.lock` still names `spark-max-rs` and `spark-max-cabi`; README says no code here uses them.
- The streaming measurement was rewritten on 2026-10-16 and has produced no committed result.
- How close the Prime Drag Race result is to the hardware limit has not been measured (README).
- The comment in `tests/gates/run.sh` mentions `.github/workflows/ci.yml`; no such file is
  tracked at this revision.
- Planned issues #29 to #32 have no results.

## Dependencies and provenance

- Root crate: 14 direct dependencies including tokio, reqwest 0.12, sha2 and the five path crates.
  README: rustls was updated for security advisories (#27).
- Workflows tracked: `.forgejo/workflows/ci.yml` (runs on a self-hosted `spark` runner:
  no-Python guard, fmt, check, test, verify) and `.github/workflows/aien-verification.yml`, which
  builds `aien-test` from `aien-spark/aien-sovereign-core` at
  `a51042b61b87355c0850ef82ba409a46e153a1ff`. Actions are referenced by tag, not commit sha.

## Evidence

- `prime-drag-race/CAMPAIGN-2026-10-05.md` and `RESULTS-2026-10-05.md`, with run files under
  `prime-drag-race/evidence/` (receipt `1a3eafe9`).
- `artifacts/gb10_canonical_1789907893_4d762/` with `SHA256SUMS` (withdrawn, historical).
- Status words are defined in `src/evidence.rs`: VERIFIED, EXPERIMENTAL, HISTORICAL, NOT_RUN,
  FAIL, SUPERSEDED. Signed Spark bundles are published in aien-dev/aien-receipts.
