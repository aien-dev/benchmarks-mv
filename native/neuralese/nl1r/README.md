# NL1R independent NL6 continuation

This directory appends to NL6. Original benchmark manifests, freezes and evidence are unchanged. The immutable registration is `prereg.json`, benchmarks commit `c8aefbc`, SHA-256 `2fe5c72edbfe24df38f70b72275bb008ac71779e66455501662ca8e7d911751c`. It binds capacity and the later three task-conditioned experiments before learned outcomes. See draft benchmarks-mv PR 59 and aien-train issue 39, benchmarks issue 55, aien-architecture issue 349.

## Independent implementation

`io.om`, `arithmetic.om`, `judge.om` form the native judge. It links no producer forward, decoder, gradient or summary code. It consumes the actual C/P/R/T stream, checks the full checkpoint and serialized carrier, reconstructs the expected wire from checkpoint and input, independently decodes the actual bytes, recomputes scores, predictions, abstention, semantic errors and payload collisions, and refuses malformed input without a partial report. The score comparison tolerance is absolute 1e-12; this is floating-point implementation tolerance, not a training or success threshold. The deployed channel is not inferred from a producer's success flag.

Capacity CSV has exactly 32 carrier columns: the producer materializes the fixed 32-byte carrier; useful payload is separately 24, 32 or 1 bytes. Task CSV has 32 logging columns but only payload_len bytes are transmitted; unused columns are zero log padding. No additional transport header is implied. Existing NL4 boundary-envelope costs remain separate. Input/sample IDs and expected targets are judge metadata, not permitted receiver side channels. Task context is receiver-private and allowed only for task 2.

`fixture.om` produces synthetic protocol controls. Its timestamps 1 and 2 are fixture constants, never performance evidence. `host.cases` runs the existing seeded handwritten Omega OM2GATE conductor, including guard-removal mutants, exact refusal stdout/stderr and exit codes. This is reuse of the existing native runner, not a foreign replacement evaluator. The conductor, pinned seeded compiler, git, SHA-256 tooling and read-only orchestration remain verification oracles.

`checkpoint.om` independently packs the C/P bit carriers into the declared actual little-endian binary checkpoint (mode 0), or validates and decodes the exact binary header and all binary64 parameters before emitting B/P (mode 1). Validation refuses unsupported schema, dimensions, ranges, nonfinite parameters, truncation and trailing bytes atomically. It is a verifier-side native conversion/decoder, not a trainer; the full raw campaign still must pass the complete judge.

Compiler binary SHA-256: `d6290c7663c2b4bafb59eb3fdf812476e977f4c144def71095c6248ef9b96b84`. Compiler source SHA-256: `3f937160c823d3a90a6b4cc296f00f517f36ede7fd27a06505c998985ec62240`, source authority omega `1fe5976d398646b17acabbf6d9dfae96b017e439`. The local compiler checkout HEAD may be newer; binary/source digests, not its branch name, identify the executable used.

`mutate.om` generates native field-level task negatives; `task-checkpoint.cases` exercises task context/effect/score/omission guards and actual-binary checkpoint negatives/mutants. The exercised machine-bound case manifests retain the exact checkout paths and external conductor/compiler paths of this experiment; they are evidence, not a portable general-purpose runner API.

## Custody and release

Hidden custody commitment: `39ce89eebff948ccc78f49f54564fa722efeaa8e618bda8bd0895f09d7ec2772`, created 2026-10-28T22:05:44Z. Custody remained outside both product trees under mode-0700/0600 until both source/config and all seven checkpoint/prediction commitments were frozen. After the historical pause, Main explicitly resumed this lane and froze the existing-NL4 adapter at train `18b3d08cdf383033e2c8d674a37325d11f010fc4`. The judge independently rebuilt and hash-matched that adapter BEFORE opening custody. Original canonical bytes are now published as `evidence/custody-reveal.json`; `reveal-chronology.json` verifies the commitment and every original raw/checkpoint hash. This is procedural same-Unix-principal custody, not an adversarial security boundary or vendor/model independence. Public replay was not substituted for the withheld campaign.

All public seeds, arms, splits and failed outcomes are retained; no tuning, model selection, seed selection or run-until-pass. Constructive handweights and hand codecs are analytic controls, not learned successes. Training uses the declared continuous softsign relaxation. Dense/factorized task channels are inference-only quantizations of the same per-task checkpoint, not independently trained systems; their continuous/discrete deployment gap is measured honestly.

## Evidence semantics

The mathematical oracle queries/results, exact source/compiler/binary/checkpoint/raw hashes, native qualification logs, complete per-seed reports, commitment chronology and raw costs are appended under `evidence/`. Historical receipts are never edited. All original withheld raw outputs, checkpoints and native reports are now published under `evidence/hidden/`. Host synthetic execution does not qualify a device, GPU, live service or production authorization. Energy, physical DRAM and network measurements remain NOT_MEASURED/NOT_RUN. Production capability/reservation/execution remains NOT_RUN. Truth-referenced NL4 pure identity checking does not improve learner accuracy or authorize effects.

## Exercised public result and historical sealed handoff

See `evidence/REPORT.md`, `public-native-metrics.json`, `public-independent-receipts.json`, `public-cost-ledger.json` and the source/build inventory. Both native qualification suites passed (30 and 23 cases, including twelve targeted guard-removal mutants). All 84 frozen public producer runs passed independent actual-wire verification, exact deterministic semantic replay and actual-binary checkpoint decoding; all 48 capacity checkpoints also passed native import/replay.

The historical sealed pause remains documented in `evidence/REPORT.md` at commit `3f42fc1`. Its pending-release/boundary status is superseded by the completed report below; historical receipts are unchanged. No tuning, replacement training run, changed protocol or selection occurred on resumption.

## Completed withheld evaluation and full NL4 replay

`evidence/FINAL-REPORT.md` is the complete current result. `all-seed-report.tsv` publishes all 2691 public/withheld native metric rows, including every failed arm, quantization/ablation variant, wrong-accepted outcome and abstention. Exact finite regret for all three tasks is supplied by the mounted Wolfram query/result, not host arithmetic. Original seed 3729207599 and mask 63840 are revealed only after the frozen artifact commitments; all seven original streams/checkpoints match their pre-score hashes.

`boundary-oracle.om` composes with `io.om`, `arithmetic.om`, and `judge.om` lines 1..321 (excluding its main). It is a post-reveal transport exporter, not a change to the frozen scientific scorer. It validates the entire task stream before emitting any case, independently recomputes deployed-byte predictions, abstention and trusted targets, then emits exact expected pure NL4 receipts. An extra CLI argument emits eighteen fixed argument/envelope/mismatch/abstention controls. Native logic, not the read-only host orchestration, supplies these expectations.

`boundary-campaign.json` indexes every one of the 9360 decisions from all 36 public and three withheld task streams. Every native expected status/stdout/stderr/frame length matched the independently rebuilt frozen adapter. All actual envelope bytes, per-decision arguments, hashes and clock endpoints are retained in `boundary-replay/`. All eighteen boundary controls passed, all four adapter guard mutants were detected, and all eight malformed task streams were atomically refused by the native exporter.

`resumed-build-inventory.json` and `boundary-independent-build.json` bind exact new source/compiler/binary hashes and prove the original scientific judge source unchanged. `resumed-cost-ledger.json` adds hidden costs and the actual separate NL4 envelope bytes; it does not turn task CSV padding into transmitted payload or invent a new header. No net communication/economic advantage, hardware qualification, production effects, merge or deployment is claimed. Source drafts are aien-train PRs 40/41; Main owns shared-guide updates and integration.
