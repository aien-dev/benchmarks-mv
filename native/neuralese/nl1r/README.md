# NL1R independent NL6 continuation

This directory appends to NL6. Original benchmark manifests, freezes and evidence are unchanged. The immutable registration is `prereg.json`, benchmarks commit `c8aefbc`, SHA-256 `2fe5c72edbfe24df38f70b72275bb008ac71779e66455501662ca8e7d911751c`. It binds capacity and the later three task-conditioned experiments before learned outcomes. See draft benchmarks-mv PR 59 and aien-train issue 39, benchmarks issue 55, aien-architecture issue 349.

## Independent implementation

`io.om`, `arithmetic.om`, `judge.om` form the native judge. It links no producer forward, decoder, gradient or summary code. It consumes the actual C/P/R/T stream, checks the full checkpoint and serialized carrier, reconstructs the expected wire from checkpoint and input, independently decodes the actual bytes, recomputes scores, predictions, abstention, semantic errors and payload collisions, and refuses malformed input without a partial report. The score comparison tolerance is absolute 1e-12; this is floating-point implementation tolerance, not a training or success threshold. The deployed channel is not inferred from a producer's success flag.

Capacity CSV has exactly 32 carrier columns: the producer materializes the fixed 32-byte carrier; useful payload is separately 24, 32 or 1 bytes. Task CSV has 32 logging columns but only payload_len bytes are transmitted; unused columns are zero log padding. No additional transport header is implied. Existing NL4 boundary-envelope costs remain separate. Input/sample IDs and expected targets are judge metadata, not permitted receiver side channels. Task context is receiver-private and allowed only for task 2.

`fixture.om` produces synthetic protocol controls. Its timestamps 1 and 2 are fixture constants, never performance evidence. `host.cases` runs the existing seeded handwritten Omega OM2GATE conductor, including guard-removal mutants, exact refusal stdout/stderr and exit codes. This is reuse of the existing native runner, not a foreign replacement evaluator. The conductor, pinned seeded compiler, git, SHA-256 tooling and read-only orchestration remain verification oracles.

Compiler binary SHA-256: `d6290c7663c2b4bafb59eb3fdf812476e977f4c144def71095c6248ef9b96b84`. Compiler source SHA-256: `3f937160c823d3a90a6b4cc296f00f517f36ede7fd27a06505c998985ec62240`, source authority omega `1fe5976d398646b17acabbf6d9dfae96b017e439`. The local compiler checkout HEAD may be newer; binary/source digests, not its branch name, identify the executable used.

## Custody and release

Hidden custody commitment: `39ce89eebff948ccc78f49f54564fa722efeaa8e618bda8bd0895f09d7ec2772`, created 2026-10-28T22:05:44Z. The salted canonical custody document is held outside both product trees in a mode-0700 private directory with a mode-0600 file. It contains the hidden split and seed. Neither producer nor parent receives these before both source/config freezes. The judge independently rebuilds the frozen producer and executes it privately. Checkpoints and all-input predictions are committed by hash before withheld scoring. The split/seed/salt reveal is delayed until BOTH capacity and task implementations and prediction commitments are frozen: an early capacity reveal would contaminate the later task campaign. This is procedural isolation under the same Unix principal, not an adversarial security boundary. A leak invalidates confirmatory evidence, and public replay is not a substitute.

All public seeds, arms, splits and failed outcomes are retained; no tuning, model selection, seed selection or run-until-pass. Constructive handweights and hand codecs are analytic controls, not learned successes. Training uses the declared continuous softsign relaxation. Dense/factorized task channels are inference-only quantizations of the same per-task checkpoint, not independently trained systems; their continuous/discrete deployment gap is measured honestly.

## Evidence semantics

The mathematical oracle queries/results, exact source/compiler/binary/checkpoint/raw hashes, native qualification logs, public and withheld per-seed reports, commitment chronology and costs are appended under `evidence/`. Historical receipts are never edited. Host synthetic execution does not qualify a device, GPU, live service or production authorization. Energy, physical DRAM and network measurements remain NOT_RUN unless measured. Production capability/reservation/execution remains NOT_RUN. NL4 pure identity proposal checking is separate from semantic correctness and does not authorize effects.
