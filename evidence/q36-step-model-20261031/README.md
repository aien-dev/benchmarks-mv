# q36 step-time model, version 1 (DRA-64)

Refs aien-dev/benchmarks-mv#65, Linear [DRA-64](https://linear.app/drake-stapleton/issue/DRA-64/pin-the-fitted-step-time-model-and-its-receipts-as-a-reproducible).

The fitted step-time model of one Qwen3.6-35B-A3B decode stroke (n heads, one token each) on one GB10:

    t(n) = 9.13 + 3.29 n + 0.696 U(n) + 4.81 Ceiling[n/4]   ms per stroke
    U(n) = experts read per layer per stroke = routed-expert union of the n heads + 1 shared expert

`q36-step-model.wl` is the versioned model (`q36-step-model/1`). It is frozen: a change is a new versioned file opened
by a model-revision issue, never an edit of this one. Every number below is copied from
`q36-step-model-wolfram-raw.json` (hosted Wolfram Language evaluator, MCP tool `WolframLanguageEvaluator`); none is hand
computed. Host evidence only: no chip was used for this directory, and the ladder it fits is labelled
"shared chip, not a speed receipt" in its source README.

## Files

| file | what |
|---|---|
| `inputs/` | copies of the source receipts, pinned by `SHA256SUMS` (source: aien-sovereign-core-mv `d3dec0eb`, `evidence/motor-chip-20261021/`) |
| `derived/union-cells-*.json` | per-cell routed-expert union counts (31 decode steps x 40 layers) from the host goldens, by `scripts/union-cells.sh` |
| `derived/header-bytes.json` | weight bytes per launch class from the checkpoint safetensors headers, by `scripts/header-bytes.sh` |
| `inputs.wl` | the Wolfram association `q36in`, generated from the pinned files by `scripts/extract-inputs.sh` |
| `q36-step-model.wl` | the model: frozen parameters, `predict[n, U]`, `predictTokS`, `recordLever`, the fit procedure, the checks, the report |
| `q36-step-model-query.wl` | exactly what was sent: `inputs.wl` + `q36-step-model.wl` |
| `q36-step-model-wolfram-raw.json`, `q36-step-model-rerun-wolfram-raw.json` | two evaluations of the same query; `q36-step-model-rerun.diff` (0 bytes) is their text diff |
| `negctl-768.wl`, `negctl-perturbed.wl` | the two negative-control overrides; `negctl-*-query.wl` = inputs + override + model; `negctl-*-wolfram-raw.json` their outputs |

Goldens used by `scripts/union-cells.sh` (not copied, 3 MB; aien-sovereign-core-mv `d3dec0eb` `evidence/q36-host-20261020/`):

    cf0e0119aec93c4325cfed86fe7b2a2b669f768edbfe816082a3f11a0a066207  gen-p1.json
    7d4bd2b795f9d65f943c4c804e1ab06b8a77723b27a355c6daba81a38e08241a  gen-p2.json
    78168f26a505ee086a83b3efde3ba69041c4e381c63a17b2c95263aa68ca8142  gen-p3.json
    beb224ed1062a20b12dd5d0d47e3a3bd480d296688dc2c03db6feb333479932c  gen-p4.json
    3dad6e46beb121036173cc12c304c4739a54ed96fbc5f99fb81bacd2c3ebee69  gen-chat-code.json
    d1266bdda1c6281423fc1b9338dce3087234fb56b2a0917173e0c9ac5a738f5c  gen-chat-prose.json
    394634b60fd0032e88651444694387a4a02fe1051cfcffd33c95c136468d1191  gen-chat-prose2.json
    570daa064d83fdaae1db69485b2d72dab807e0df7e84aab9a0b3c3916ce428b3  gen-chat-chat.json
    03e1df06010ac7f2e96431354a6c62678dc64af7747c4b93525dddffdb98fe9f  gen-chat-review.json

The pinned union script `inputs/expert-union.sh` run on these goldens reproduces all four pinned union files
(`expert-union-{4p,4chat,8prompts,8prompts-b}.json`) byte for byte (`cmp`, exit 0, 2026-10-31).
Checkpoint: `~/models/qwen3.6-35b-a3b-fp8-95a723d0` on the Spark; `derived/header-bytes.json` records the sha256 of
`config.json` and of each of the 41 safetensors header blobs it read (layers 0-39 and `outside.safetensors`;
`mtp.safetensors` excluded). Only headers are read; no tensor data and no model load.

## Reproduce

    cd evidence/q36-step-model-20261031
    sha256sum -c SHA256SUMS
    scripts/header-bytes.sh ~/models/qwen3.6-35b-a3b-fp8-95a723d0 | cmp - derived/header-bytes.json
    (cd <core d3dec0eb>/evidence/q36-host-20261020 && <this dir>/scripts/union-cells.sh gen-p1.json gen-p2.json) | cmp - derived/union-cells-ladder2-p1p2.json   # likewise for each union-cells file, sets below
    scripts/extract-inputs.sh | cmp - inputs.wl
    cat inputs.wl q36-step-model.wl   # send as one WolframLanguageEvaluator call; the text must equal q36-step-model-wolfram-raw.json .text

Union sets: `ladder1-p1` {p1}, `ladder2-p1p2` {p1, p2}, `ladder4-p1234` {p1..p4}, `ladder8-setb` {p1..p4, chat-code,
chat-prose2, chat-chat, chat-review}, `4chat` {chat-code, chat-prose, chat-chat, chat-review}, `8a` {p1..p4, chat-code,
chat-prose, chat-chat, chat-review}, `pair-code-p2`, `pair-code-review`. The ladder sets are the goldens of the
lever-2 ladder runs (`inputs/stroke{1,2,4,8}l2-real.json`, `conversations`).

## Checks (all True in the main run)

| check | meaning |
|---|---|
| ladderMediansMatchReadme | the gate median of each `stroke_chip_ns` list equals the README ladder (L247-250) to 0.0005 ms |
| ladderMediansMatchFileField | and equals the file's own `stroke_chip_ns_median` exactly |
| stampsMedianMatchesLog | the stamped run's median equals the 84.076 ms in its log |
| ladderHeadsAre1248 | the four ladder files are 1, 2, 4, 8 heads |
| pinnedUnionMeansRederived | Wolfram's mean of the union cells equals each pinned file's `mean_union` (1e-9) |
| expertBytesContract | header bytes per expert = 3*2048*512 FP8 + 3*(16*4) BF16 scales x 2 B = 3,146,112; shared expert the same; 256 routed experts; 40 layers |
| lmHeadBytes | lm_head = 248320 x 2048 x 2 B |
| mixBytesMatchReadme0944GB | mix bytes round to the README's 0.944 GB |
| fitPublishedReproducedLinearU | the fit procedure below reproduces a, b, c, h, RMS within half a unit of their published last digit |
| linearUMatchesPinnedUnions | Linear U(4) 30.7 and U(8) 55.6 are the pinned 4chat and mean(set a, set b) unions + 1; U(1) = 9 |
| contractT8 | t(8, 55.6) = 83.7676 ms and 95.50 tok/s |
| variantIndep | independent-draw U(8) rounds to 58.4 and gives 93.3 tok/s |
| weightClassesBelowCeiling | no weight class runs above 273 GB/s with the corrected bytes |

The gate median is the upper median (element Floor[len/2]+1 of the sorted list), the convention of the stroke gate and
of the README. Wolfram's `Median` (mean of the middle pair) gives 25.30227, 32.457668, 44.288881, 85.427837 ms; the fit
uses the gate's 25.320206, 32.464756, 44.350601, 85.427837 ms.

## Fit procedure (reproduces the published numbers)

    c = (gate_up + down chip time of the stamped lever-2 eight-head stroke) / U(8)  = 38.69792 ms / 55.6
    h = (lm_head chip time of that stroke) / (2 lm_head launches)                   = 9.619136 ms / 2
    a, b = least squares of  median(n) - c U(n) - h Ceiling[n/4]  over n = 1, 2, 4, 8

| U set | a | b | c | h | RMS ms | residuals n = 1, 2, 4, 8 (ms) |
|---|---|---|---|---|---|---|
| Linear (9, 16.3, 30.7, 55.6) | 9.12862 | 3.288456 | 0.696006 | 4.809568 | 2.418624 | 1.82951, 0.604763, -4.108787, 1.674514 |
| ladder-matched (9, 16.749194, 30.374194, 55.541129) | 9.01357 | 3.31065 | 0.696743 | 4.809568 | 2.314708 | 1.915726, 0.350426, -3.87816, 1.612008 |
| independent draws (9, 16.75, 31.531006, 58.420831) | 9.53677 | 3.248511 | 0.662399 | 4.809568 | 2.28722 | 1.763763, 0.526208, -3.875897, 1.585926 |

The published (rounded) parameters against the four ladder medians: RMS 2.418561 ms, residuals 1.826206, 0.599956,
-4.116599, 1.660237 ms. A free four-parameter solve (a -66.42103, b -80.574258, c 12.016275, h 64.169018) passes
through all four points by construction and is not a model; it is reported only to show that four points cannot
discriminate four free parameters, which is why c and h come from the stamped timeline.

U difference report: the Linear U values do not come from the ladder's own conversations. Ladder-matched minus Linear:
0, +0.449194, -0.325806, -0.058871. U(2) = 16.3 has no pinned source: the ladder pair {p1, p2} gives 16.749194; the
closest pinned pairs are chat-code + p2 (16.342742) and chat-code + chat-review (16.289516). Only the Linear U set
reproduces the published parameters; the ladder-matched set gives the second row of the table above (h unchanged).

## Variants at n = 8

| variant | U(8) | t ms | tok/s |
|---|---|---|---|
| measured union (Linear) | 55.6 | 83.7676 | 95.502318 |
| independent-draw union, exact 1 + 256(1-(31/32)^8) | 58.420831 | 85.730898 | 93.315248 |
| independent-draw union, Linear rounding | 58.4 | 85.7164 | 93.331031 |
| ladder-matched (set b) | 55.541129 | 83.726626 | 95.549055 |

## Per-class achieved bandwidth, corrected bytes

Stamped lever-2 eight-head stroke (`inputs/stroke8l2-stamps.json` `timeline_last_stroke`, 1469 launches, total_ns 77738400).
Stamps cost chip time and the chip was shared: these are stamped-timeline rates, not speed receipts. Expert bytes use
the header-read 3,146,112 B per expert and U(8) = 55.541129 (set b, the stamped run's conversations).
[INFERENCE] The timeline is the last stroke; its exact union is not recorded, so the mean set-b union stands in for it.
[INFERENCE] The gate_up and down launches include the shared expert (one launch per layer per kind, 40 each).

| class | bytes per stroke | launches | ms | GB/s | % of 273 GB/s |
|---|---|---|---|---|---|
| experts gate_up + down | 6.989544501677419e9 | 80 | 38.69792 | 180.618093 | 66.160474 |
| experts gate_up | 4.659696334451613e9 | 40 | 22.839968 | 204.015012 | 74.730773 |
| experts down | 2.329848167225806e9 | 40 | 15.857952 | 146.919865 | 53.8168 |
| lm_head bf16 (2 four-head reads) | 2.03423744e9 | 2 | 9.619136 | 211.478187 | 77.464537 |
| mix (FP8 + scales) | 9.438336e8 | 40 | 6.501856 | 145.163719 | 53.173524 |
| o_proj (FP8 + scales) | 3.3558528e8 | 40 | 2.433344 | 137.911154 | 50.516906 |
| router (bf16 256 x 2048) | 4.194304e7 | 40 | 2.131328 | 19.679299 | 7.208534 |
| ab (bf16 in_proj_a, in_proj_b) | 7.86432e6 | 30 | 0.419776 | 18.734563 | 6.862477 |
| superseded: experts at 768-wide 3 x 2048 x 768 B | 1.0483037084903227e10 | 80 | 38.69792 | 270.894071 | 99.228597 |

The source README computed its expert rows with the 768-wide expert (4,718,592 B), which put the experts near the
ceiling (last row). With the header bytes the expert GEMVs run at 66 % of 273 GB/s: the experts are not at the
bandwidth wall, and the remaining headroom is a lever input, not a fixed cost. gdn, attn and the small kinds read
state rather than weights and have no row.

## Projection rows

| n | U independent draws | t ms | tok/s | status |
|---|---|---|---|---|
| 1 | 9 | 23.494 | 42.564059 | fitted range (lever-2 ladder) |
| 2 | 16.75 | 32.178 | 62.154267 | fitted range (lever-2 ladder) |
| 4 | 31.531006 | 49.04558 | 81.556788 | fitted range (lever-2 ladder) |
| 8 | 58.420831 | 85.730898 | 93.315248 | fitted range (lever-2 ladder) |
| 16 | 102.962162 | 152.671665 | 104.800062 | architecture projection until OMEGA_GPU_Q36_MAX_HEADS > 8 is qualified |
| 32 | 164.313846 | 267.252437 | 119.736981 | architecture projection until OMEGA_GPU_Q36_MAX_HEADS > 8 is qualified |
| 64 | 223.442488 | 452.165971 | 141.540947 | architecture projection until OMEGA_GPU_Q36_MAX_HEADS > 8 is qualified |

`OMEGA_GPU_Q36_MAX_HEADS` is 8 (omega-mv `474197e4` `src/omega_gpu_q36_graph.h` L66). Rows with n >= 16 are outside
every measurement and stay projections until a qualified build raises that limit.

## Procedure for later lever issues (10 % rule)

Every lever issue that changes the stroke records predicted against measured stroke time with this model, before
claiming a speedup:

1. Measure the lever's stroke median with the stroke gate (gate median, `stroke_chip_ns_median`) at n heads, in a
   spark-window, and its union U(n) with `scripts/union-cells.sh` on the run's goldens (Wolfram `1 + Mean[cells]`).
2. Evaluate `inputs.wl` + `q36-step-model.wl` + `recordLever["DRA-NN lever", n, U, measuredMs]` in the
   WolframLanguageEvaluator; save the raw json beside the lever's receipt.
3. `verdict` is `WITHIN_MODEL` when |measured - predicted| / predicted <= 10 %, else `OPEN_MODEL_REVISION_ISSUE`:
   open a model-revision issue in aien-dev/benchmarks-mv citing the raw json; the revision adds a new versioned
   `.wl` (q36-step-model/2) with its own pinned inputs, and lever predictions keep citing the version they used.
   A lever that is designed to change a term (for example h after an XBATCH change) predicts with the revised term
   and states which term it changed.
4. `scope` marks n > 8 as an architecture projection.

The main run carries two worked examples: measured = 1.05 x predicted gives residual 0.05, `WITHIN_MODEL`;
1.15 x predicted gives 0.15, `OPEN_MODEL_REVISION_ISSUE`. Applying the rule to a later lever is NOT_RUN here.

## Negative controls

| control | override | checks that went False | allChecksPass |
|---|---|---|---|
| `negctl-768` | expert bytes = 3 x 2048 x 768 | expertBytesContract | False |
| `negctl-perturbed` | n = 4 ladder strokes x 1.05 | ladderMediansMatchReadme, ladderMediansMatchFileField, fitPublishedReproducedLinearU (a 9.610692, b 3.307739, RMS 1.487754) | False |

The 768-wide expert keeps `weightClassesBelowCeiling` True (270.894071 GB/s, under 273): the ceiling check alone would
not catch it, the header-bytes contract does.

## Status

| item | status |
|---|---|
| inputs pinned by sha256, model reproduces the published fit | PASS (host, Wolfram) |
| measured-union and independent-draw variants | PASS (host, Wolfram) |
| per-class bandwidth from corrected bytes | PASS (host, Wolfram; stamped shared-chip timeline, not a speed receipt) |
| predicted vs measured procedure and `predict[n, U]` | IMPLEMENTED, exercised on two synthetic examples; application to later levers NOT_RUN |
| n >= 16 rows labelled projections | PASS |
| rerun determinism | PASS (two evaluations, text diff empty) |
| chip qualification of the model | NOT_RUN (no chip run in this directory) |
