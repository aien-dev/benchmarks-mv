# NL1R independent host evidence, sealed handoff

Status at user priority change: IMPLEMENTED and EXERCISED for the independent native checker, public replay and native negative controls. Hidden inputs, raw predictions, native scores and custody reveal remain SEALED. The existing-NL4 task boundary adapter is not yet source-frozen; no end-to-end task boundary qualification is claimed. Nothing is MERGED or DEPLOYED. No GPU, model service, hardware, driver or boot operation was performed.

## Immutable chronology and source independence

- Immutable preregistration: `c8aefbc`, SHA-256 `2fe5c72edbfe24df38f70b72275bb008ac71779e66455501662ca8e7d911751c`.
- Custody commitment, before new learned outcomes: `39ce89eebff948ccc78f49f54564fa722efeaa8e618bda8bd0895f09d7ec2772`.
- Independent checker source: `1e77119`; actual checkpoint verifier `021b074`, extended to the producer's schema-2 task checkpoint at `1ffdbf1` BEFORE any private execution. The immutable manifest was not amended.
- Capacity product source freeze: train `fed670a`, evidence/docs `9061d3b`, draft [train PR 40](https://github.com/aien-dev/aien-train/pull/40).
- Task product source freeze: train `1bae5bda767c9024c1408ae866c7993eb3d7deb0`.
- Both independently rebuilt producers are byte-identical to the producer binaries. `source-freezes.json` binds module/compiler/source/binary hashes.
- All seven hidden final checkpoints and all-input raw predictions were hashed and publicly committed at `8a78e38` BEFORE native withheld scoring. `hidden-prediction-commitment.json` contains hashes only. No hidden input, split, seed, score or aggregate has been disclosed.
- The hidden campaign is NOT public replay. Sealed private outputs and their independent repeat/import evidence are retained outside the repository. User's Aienux priority supersedes release; custody remains sealed until the parent resumes NL1R and freezes the existing-NL4 adapter. This is procedural custody under one Unix principal, not vendor/model or adversarial isolation.

Draft [benchmark PR 59](https://github.com/aien-dev/benchmarks-mv/pull/59) links [train issue 39](https://github.com/aien-dev/aien-train/issues/39), [benchmarks issue 55](https://github.com/aien-dev/benchmarks-mv/issues/55) and [architecture issue 349](https://github.com/aien-dev/aien-architecture-mv/issues/349). Original train/benchmark negative evidence and freezes remain immutable.

## Native verification actually exercised

`native-qualification-1.stdout`: OM2GATE PASS, 30 cases. `native-qualification-2.stdout`: OM2GATE PASS, 23 cases. Combined cases include positive direct/four-coordinate/task/binary fixtures, malformed protocol and actual-binary checkpoint refusals, and twelve targeted guard-removal mutants. Each mutant uses an exact-once native conductor substitution, accepts the bad fixture only after the corresponding guard is removed, and is killed by the original refusal oracle. Refusals exit 90, emit no partial metric report and use exact stderr. This proves the tested guards, not arbitrary security or production authorization.

`public-independent-receipts.json`: every one of the 48 capacity and 36 task public invocations was independently rebuilt and rerun from the frozen sources. Every original raw stream passed the native judge. Replayed checkpoint/semantic rows were byte-exact after excluding only measured encode/decode clock columns and T; no score tolerance was changed for observed data. The native actual-binary checkpoint packer matched all producer checkpoint files exactly; its decoder accepted all. Every capacity binary checkpoint was imported by the independent rebuilt producer, with exact parameter and semantic row replay, then accepted again by the native judge. Source and evaluator independence are claimed; model/vendor independence is not.

The native judge independently validates every actual transmitted carrier byte, parameter carrier, reconstructed sender message, receiver score, threshold decision, active-output abstention, context/effect metadata, omission status, complete row coverage and target-dependent metric. It does not trust producer summaries. Task receiver-only baselines receive no sender payload; mismatch is the frozen `(id+1) mod16` derangement, not a new random control. Omission refuses. No sender target ID is used by the receiver.

## Public scientific outcomes, all seeds retained

`public-native-metrics.json` is the complete native per-seed report, including all splits, arms, quantization/ablation modes, omissions, wrong-accepted cases, abstentions, feature errors and collision counts. `public-independent/` retains each original, independent replay, native report, actual checkpoint and decoded parameter carrier. Failed learning arms are not suppressed or rerun for selection.

Full-fit capacity clean, grid-1/16 and grid-1/32 results are identical within each listed seed:

| Capacity arm | seed 17 accepted-correct/16 | seed 29 | seed 43 |
| --- | --- | --- | --- |
| Historical 4->3->4 | 14 | 14 | 14 |
| Four-coordinate 4->4->4 | 16 | 16 | 16 |
| Nonlinear 4->3->8ReLU->4 | 14 | 15 | 14 |
| Direct one-byte fixed code | 16 | 16 | 16 |

These are training-only full-fit results, not confirmatory generalization. The nonlinear hand codec and the same-architecture constructive nonlinear witness both validate all finite states as separate handweight controls; they are NOT successful learned runs. Neither observed historical 14 nor learned nonlinear 15 is called a training-system optimum.

Public task heldout behavior varies sharply with split, task and channel. The dense reconstruction arm recovers the historical parity complement but fails the public equal-bit splits; binary factorization loses additional reconstruction information. The parity learner fails most heldout arms, with all wrong-accepted and abstained outcomes retained. Receiver side information helps its declared task but does not guarantee success on the equal02 split. The exact fixed code and full-evidence comparator remain separate controls. No hidden outcome is inferred from these public results, and no arm is selected from them.

Task dense/factorized channels are inference-only categorical ablations of the SAME trained checkpoint per task. Training remains the exact continuous softsign relaxation, with no straight-through estimator or alternate optimizer. This measured discrete deployment gap is not independent discrete-channel training or a learned communication advantage. Full text is a four-byte comparator, not an equal-budget control. Regret for all three finite tasks is exact 0/1 task error, derivable from the recorded total and exact counts, never a production-work metric.

## Independent mathematical and gradient evidence

`independent-mathematics-query.wl` and its raw mounted Wolfram result reproduce:

- WolframLanguageEvaluator: with `r[n_,d_]:=Sum[Binomial[n,k],{k,0,d}]`, `r[4,3] -> 15`; `2Sum[Binomial[3,k],{k,0,2}] -> 14` for central strict planes, not biased affine planes. Exact combined query and raw result are retained beside this report.
- The recurrence `R(n,d)<=R(n-1,d)+R(n-1,d-1)` follows because a new plane splits at most the arrangement formed on that plane; bases are one. Strict sign patterns are convex open-halfspace intersections, so there are no disconnected regions with the same strict sign vector. Positive margin excludes boundary ties.
- An explicit four-plane arrangement attains fifteen strict patterns. `bounded-arrangement-query.wl` and raw result additionally give exact rational witnesses inside `(-1,1)^3` for planes `{x,y,z,1/4-x-y-z}`. This is a tight receiver-region bound, not proof that a particular sender or training method reaches it.
- Four-coordinate witness: all sixteen states, scores `{1/3,2/3}`, margin `1/6`; the frozen grid quantizers preserve its coordinates.
- Issue-39 hand codec and same-architecture nonlinear witness: exact recovery; nonlinear score margin `1/4`.
- Task proper colorings and equal clique lower witnesses: reconstruction 16 colors/4 useful bits, parity 2/1, receiver-side-info 4/2. The actual declared one-byte payload still charges its full byte. These are finite-domain exact witnesses, not population or economic advantages.

`independent-gradient-query.wl` compares all native parameter derivatives and staged updates for all three capacity shapes against independently differentiated exact declared losses in mounted Wolfram. Both comparisons pass at absolute 1e-12. `independent-task-gradient-query.wl` independently checks all three task masks the same way; all pass. Native raw derivative carriers and exact queries/results are retained. `parent-oracles.json` appends the parent's separate provenance without editing old evidence; mathematical tool evidence is not itself native floating-point or hardware evidence.

## Cost and measurement boundary

`public-cost-ledger.json` contains every original CSV byte count, C/P/R metadata cost, actual checkpoint byte count, raw T interval and independently measured replay/checking clock endpoints. Source and binary sizes/hashes are in the build inventory. Capacity materializes a 32-byte carrier, with useful bytes 24/32/24/1 and padding charged separately. Task transmits only 1/1/1/0/4 payload bytes for its five arms; the rest of C32 is zero log padding. All omitted task payloads are zero bytes and refused. No invented eight-byte task transport header was added.

Capacity T covers initialization/training and raw checkpoint/row emission, so it is NOT a training-only measurement. Task T covers initialization plus training. Per-row encode/decode and independent checking intervals are distinct. CSV and checkpoint metadata are not free or silently identified with a network packet. Existing NL4 charges its separate 1024-byte header and 32-byte padded payload; parent adapter replay is pending, not claimed here.

Energy, physical DRAM traffic, network, GPU, real-model adapters, authorized completed-work throughput and population amortization are NOT_MEASURED/NOT_RUN, never zero. No merge, deployment, hardware qualification, authorized effects or neural communication advantage is claimed.

## Remaining exact obligations after user resumes NL1R

The existing-NL4 adapter must be source-frozen before any hidden release. Then verify commitment bytes and chronology, reveal the original custody document without altering it, publish every sealed raw/checkpoint/native per-seed report and exact costs, perform the fixed NL4 pure-identity chunk replay and negative controls, and integrate the task source PR and current guides through the parent. No training, hyperparameter, selection, split, seed, metric or authority change is authorized. This draft deliberately does not claim the full original mission complete while those release/boundary obligations remain.
