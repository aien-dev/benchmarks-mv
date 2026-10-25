# ORIG3-SELECTOR-v2: verdict NO-GO (frozen v2 rule, applied verbatim)

Prereg `manifests/prereg/orig3_selector_v2.json` + `docs/orig3/PROTOCOL-v2.md` + judge
`bench_orig3_judge_v2`, frozen and pushed in benchmarks-mv `0ca6b9c` (2026-10-25T12:08:28Z).
Host-only (DGX Spark AArch64 cores). GB10 NOT_RUN, energy NOT_MEASURED. The v1 prereg,
its INCONCLUSIVE verdict and `evidence/ORIG3-SELECTOR/` are unchanged.

## Order of events

| step | time (UTC) | identity |
|---|---|---|
| prereg freeze commit pushed | 12:08:28 / 12:08:30 | benchmarks-mv `0ca6b9c` |
| collection C2 (one run) | 14:15:15 to 14:16:25 (70.6 s), rc 0, 44 checks 0 failures | omega-mv `85641e1e`, binary sha256 `37f4c229...`, clean tree; load 3.57 at start, 4.31 at end; 11 / 3 established TCP connections on 18091 (proxy for live traffic, nothing sent to it); `collection-c2-*.meta` |
| C2 file | 1938 records, fnv `6887491069590809475`, sha256 `a2dad5aa...fa8f` | local only (0600), `allow-v2.txt` |
| v2 admission | 1714 admitted, 224 refused (all reason -15 duplicate state: the harness's second TRAIN pass and repeated shapes); TRAIN 976 / VALIDATION 315 / HELD-OUT 423 records; 89 / 28 / 32 families | aien-train `evidence/SI19-ORIG2-v2/c2-admission.stdout` |
| training (once per arm) | candidate 10.0 s (64 epochs, 312320 updates), shuffled 2.7 s, untrained 0 | aien-train `692d8510`, `evidence/SI19-ORIG2-v2/` |
| overhead (7 interleaved reps) | medians: disabled 31291 us, MLP 169856 us, retrieval 141121 us over 1714 scored decisions | omega-mv `evidence/ORIG4-SHADOW-v2/overhead/` (copied to `overhead/`) |

## Verdict table (HELD-OUT, pressure 0; `judge.out`)

Overhead per decision: MLP and its controls 80.84 us, retrieval 64.08 us, heuristics 0.
Total = sum over decisions of calls x ps + overhead; ratios to the comparator
min(retrieval, heuristic) and, choice only, to the oracle.

| arm | X925 (n 205) total / min | X925 choice / oracle | A725 (n 183) total / min | A725 choice / oracle |
|---|---|---|---|---|
| mlp_scratch_v2 (candidate) | 1.7728 | 1.0018 | 1.2844 | 1.0203 |
| retrieval | 1.6118 | 1.0007 | 1.2038 | 1.0100 |
| heuristic (X925 best_static, A725 always_reference, chosen on VALIDATION) | 1.0000 | 1.0005 | 1.0000 | 1.0835 |
| best_static | 1.0000 | 1.0005 | 1.0173 | 1.1022 |
| fixed_rule (incumbent) | 1.5805 | 1.5813 | 1.5487 | 1.6780 |
| always_reference | 1.4634 | 1.4641 | 1.0000 | 1.0835 |
| untrained_control | 1.9199 | 1.1489 | 1.3887 | 1.1333 |
| shuffled_label_control | 2.2976 | 1.5268 | 1.7713 | 1.5478 |
| constant_control | 2.2349 | 1.4641 | 1.3427 | 1.0835 |

- PRIMARY (P1 and P2) fails for the candidate on both classes (0/205 and 0/183 decisions
  cheaper than retrieval; 0/205 and 10/183 cheaper than the heuristic). All three controls
  fail PRIMARY: the run is VALID.
- Non-inferiority fails on both classes (1.7728 and 1.2844 > 1.01): **NO-GO**.
- Amortization: the saving per decision against the heuristic is negative on both classes,
  so break-even decisions = none (training 10.0 s + collection 70.6 s are never repaid).
  Choice only, the candidate saves 15225 ps per call on A725 against always_reference; its
  80.84 us overhead would break even at 5310 calls per decision (the mean is 903.5). On X925
  it is worse than best_static even before overhead.
- Reported only: HELD-OUT pressure 1 (n 18 / 17), choice / oracle MLP 1.0000 / 1.0108,
  retrieval 1.0000 / 1.0395, heuristic 1.0000 / 1.0743.

## Why it loses

1. Overhead: about 81 us per decision for an out-of-process, software-binary32 scorer
   against a median decision of a few hundred microseconds of matvec work. The heuristic
   pays nothing.
2. Choice quality: on workload families absent from training the MLP does not beat retrieval
   on either class (1.0018 vs 1.0007; 1.0203 vs 1.0100), and on X925 a single static arm is
   already within 0.05 % of the oracle. The only real gap is A725 (always_reference 8.35 %
   over oracle), and retrieval closes more of it than the MLP.

## Single smallest justified next experiment

Not a larger model (the cap stays 705). Precompute the retrieval choice per (core class,
workload family) on TRAIN into a constant table inside the consumer (a lookup, no inference,
no file admission) and evaluate it under a new prereg on fresh held-out families, A725 only,
against always_reference: it attacks the one measured gap with zero per-decision overhead.

## Files

`judge.out` (reproduced by `tests/orig3_judge_v2.rs`), `v2-{mlp,shuffled,untrained}.records`
(selmain mode E per-record lines), `overhead/` (summary + COST lines), `allow-v2.txt`,
`collection-c2-*.meta`, `collection-c2-*.harness.stdout`, `SHA256SUMS`.
Not done: GB10, energy, a second operator; everything here is HOST_TESTED development
evidence, not deployment.
