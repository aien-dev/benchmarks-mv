# Noise: how big a speed difference has to be before it means anything

Status (2026-10-19): the rules and tools below are in this repository; the measured noise of
our own Spark is in the last section, with what it does not show.

## Measured elsewhere: two GB10s, identical code, 4 to 6 percent apart

TensorFold (github.com/ashhart/TensorFold, `docs/recipes/adding-a-zig-family.md`, "Traps")
records that two DGX Spark (GB10) machines ran identical code 4 to 6 percent apart, and draws
the rule "compare engines on the same box". Their `CONTRIBUTING.md` "Performance receipts"
adds: when a difference is a few percent, alternate baseline and candidate runs to separate it
from temperature and load. We have not reproduced the two-box figure (we have one Spark); it is
quoted from their write-up.

What follows for AIEN:

- A speed figure from one Spark is never compared with a figure from another machine.
- On one box, a difference under the measured A/A spread (below) is noise unless an ABBA run
  with more rounds separates it.
- The regression fences (below) cannot call a change under 5 percent "large".

## The three tools

1. **ABBA mode** (`aien-benchmarks abba`, source `src/harness/abba.rs`). Cells run baseline,
   candidate, candidate, baseline, so a steady drift (warming, throttling, load arriving) adds
   the same to both arms. Every request is greedy (`temperature` 0), serial and streamed. A
   cell's figure is the median of its per-request decode rates
   (`(usage.completion_tokens - 1) / (last content event - first content event)`, which leaves
   out queueing, the server's GPU-lock wait and prefill). An arm's figure is the geometric mean
   of its cell medians. Every cell is kept, and the slowest cell of each arm is printed beside
   the mean. HTTP 503 (server busy) is retried with backoff and never measured. The receipt
   (`aien-abba-receipt/1`) names the harness commit, GPU, driver, engine commit, checkpoint
   revision, server binary hash and every shared library it has loaded (read from `/proc`), every
   request parameter, every cell and request, and what the run does not show.
2. **Token check before speed** (`llm-stream` and `abba`). Each complete reply carries the
   SHA-256 of its streamed text. A tokens-per-second figure is printed, and written into a
   record, only when `temperature` 0 was sent and every reply equals the serial greedy reference
   at the same settings: in `abba`, a reference pass on the baseline before any cell; in
   `llm-stream`, the first warmup request (pass `--llm-temperature 0`). Otherwise the output
   says `UNVERIFIED tokens`. TensorFold's reason: a build once looked 25 percent faster because
   a bug produced wrong tokens that were easier to draft.
3. **Regression fences** (`bench_regress <dir>...`, source `src/regress.rs`). Read-only scan of
   ABBA receipts (and, with `--pipeline-wall`, the judge's wall time per aien-receipts bundle).

## Regression thresholds and why

Ideas from rustc-perf (`docs/comparison-analysis.md`) and the Lean radar
(radar.lean-lang.org/about). For each metric, points are put in time order and each change is
the relative delta from the previous point.

| Rule | Value | Why |
| --- | --- | --- |
| Fence | Q3 + 3 * IQR of the absolute relative deltas **before** this one | rustc-perf's outer fence: a change counts only if it is an outlier against that metric's own history. Lower fence ignored: absolute deltas are bounded by 0. |
| Minimum history | 8 earlier deltas | With 7 or fewer, each quartile rests on one or two values and one noisy run moves the fence by itself. Below this nothing is flagged. |
| Small / medium / large | over the fence by up to 2x / 2x to 4x / more than 4x | How far past the fence, as rustc-perf buckets by distance over the threshold. |
| Raw-size caps | under 1 percent is at most small; under 5 percent is at most medium | 5 percent is the top of the TensorFold box-to-box spread: one box cannot call a change that size large, however quiet its history. |
| Significant commit | score of 4 or more, with small 1, medium 2, large 4 | Lean radar: a few large, a medium number of medium, or many small changes. One large, two medium or four small reach 4; mixes add up. |
| Direction | metrics with `tok_s` or `per_sec` are higher-is-better; others (times) lower-is-better | Flags say regression or improvement. |

A flag is a report, not a gate: `bench_regress` exits 0 whatever it finds.

### What the aien-receipts history holds today

Run on 2026-10-19 over `~/workspace/aien-receipts` at commit `0339230` (441 bundles):

- Default mode: **0 benchmark metrics**. Bundles carry gate verdicts and timestamps, no speeds.
- `--pipeline-wall`: 11 series (one per repository), 38 flags, 18 "significant" commits. The
  fences come out at 170 to 670 percent, because a bundle's wall time swings from 1 s to hours
  depending on whether gate results and build caches were reused. These flags mean "the cache
  missed", not "the code got slower". This mode is a pipeline health signal only.

Speed history starts when ABBA receipts accumulate (or when bundles carry a speed field).

## Our own noise (A/A on the live server)

**Not measured yet.** On 2026-10-19 the A/A run (both arms pointed at the one live server,
`aien-serve-http` on 127.0.0.1:18091, model `qwen3.8-27b-fp8`, checkpoint directory
`qwen3.8-27b-fp8-017b9c7a`, driver 580.173.02) was tried four times between 17:22 and 18:20
UTC, each capped under 15 minutes. None got a single reply:

- The server takes the shared GPU lock (`/tmp/aien-gb10.lock`) itself for every request, waits
  up to 300 s for it, then answers 503. Five other GPU jobs were queued on that lock during the
  first attempt. (So the client must not hold the lock: the server would then wait on the
  client.)
- The server runs one generation at a time and answers 503 at once while busy. Two other
  clients were connected to it; in attempts 2 to 4 the harness retried 110 times at 5 s
  intervals until its deadline and never found the slot free.

The receipts of attempts 2 to 4 are complete apart from the measurement (clean harness commit,
driver, server binary SHA-256, every library the server has loaded), which shows the receipt
path works; they carry no speed. To measure: run

```bash
cargo run --release -- abba --llm-port 18091 --llm-model qwen3.8-27b-fp8 --llm-engine omega \
  --llm-precision FP8 --llm-max-tokens 32 --rounds 2 --candidate-warmup-passes 0 \
  --baseline-pid <server pid> --candidate-pid <server pid> --out-dir records
```

when no other client is using the server, and replace this section with its output. The
within-arm spread and the candidate-over-baseline figure of that A/A run are our noise.

