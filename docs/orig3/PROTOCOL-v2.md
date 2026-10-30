<!--
SPDX-License-Identifier: Apache-2.0
Copyright 2026 Drake Stapleton (AIEN / aien-dev)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
-->

# ORIG-3 v2 protocol (benchmarks-mv#45, superseding v1 for new runs)

Preregistration: `manifests/prereg/orig3_selector_v2.json` (checked by
`cargo run --release --bin bench_verify_prereg -- manifests/prereg`). Executable rule:
`src/bin/bench_orig3_judge_v2.rs` (tests `tests/orig3_judge_v2.rs`). The v1 prereg
`orig3_selector.json`, its sealed INCONCLUSIVE verdict and `evidence/ORIG3-SELECTOR/` are
unchanged. This file and the manifest are committed and pushed before any v2 record exists.

Decision words: GO, NO-GO, INCONCLUSIVE, INVALID. Host-only results are development
results, never hardware qualification. GB10 NOT_RUN; energy NOT_MEASURED.

## 1. Why v2

The v1 sealed set re-measured exactly the development workloads (aien-train
`evidence/SI19-ORIG1-v2/`: 738/738 workload keys shared), half of its "unseen" shapes share
a log2 shape family with TRAIN, v1 control criterion F was mis-specified (any arm-4-leaning
chooser beats a 1.7x-oracle incumbent by 5 %), and v1 charged no per-decision overhead
although the scorer costs about 0.14 ms per decision against a nanosecond rule.

## 2. Data (one post-freeze collection, C2)

- Harness: omega-mv `85641e1ea14b1e4a2d2d5790001aacb0bac41093` (branch
  `feat/731-orig4-shadow-advice`), `make -j2 AIENOS_LOCK_REPO=$HOME/workspace/aienos-b84c0a67
  build/rx_empirical_optimizer_test`, binary sha256
  `37f4c229293d148313d6972898f9f4e8237775a8505bc1e53c20eb8ebf681339` (clean tree).
- Command (once, host cores only, never beside a host-gate run or a model load):

```sh
OMEGA_EMPIRICAL_RECORDS=$HOME/.local/state/aien-train/orig-v2/records-c2-<UTC>.bin \
OMEGA_EMPIRICAL_COVERAGE=600 EMPIRICAL_ALLOW_NO_WIN=1 \
nice -n 19 ./build/rx_empirical_optimizer_test
```

  `nice -n 19` is the machine rule; every arm of a table is timed in rotating order, best of
  5, so contention inflates the arms of one table together. The meta file records commit,
  dirty flag, binary sha256, start/end UTC, 1-minute load average at start and end, exit code.
  Expected: 738 harness records + 600 coverage records (family 5) per core class = 1938.
- Collection must start after the commit that froze this file was pushed; the meta file and
  the evidence README state both times.
- Admission: aien-train `oxadm2` (`docs/si19/ORIG1-CONTRACT-v2.md` at `e95862f`): per-record
  refusal with counted reasons (unsupported, shape, profile, unpaired, out of range,
  unverified, sequence, duplicate state). Refusal counts are reported.
- Splits by workload family `(floor log2 M, floor log2 N)`, frozen table
  `docs/si19/orig1-v2-splits.txt` (sha256
  `c072d49843ecb5162fa03323790254cd96c96b9138461c80562d6051ed84f4cb`, 168 families: 100 TRAIN,
  32 VALIDATION, 36 HELD-OUT). Held-out families never occur in TRAIN or VALIDATION
  (`oxdisj`). The record's family word is not used.
- Allow-list: one line `<C2 fnv> AUTHORIZED orig3-v2-c2-<UTC>` in
  `evidence/ORIG3-SELECTOR-v2/allow-v2.txt`, hash only; the bytes stay local (0600).

## 3. Arms

| arm | role | definition | overhead charged per decision |
|---|---|---|---|
| mlp_scratch_v2 | candidate | select.om MLP 20-32-1 (705 params), seed 19024 random init, lr 2^-8, at most 64 epochs, early stop after 16 epochs without VALIDATION improvement, best-on-VALIDATION params; trained on TRAIN only | O_mlp |
| retrieval | baseline | v2 rule: nearest TRAIN record of the same core class and pressure bucket by L1 on (lg16 M, lg16 N); its cheapest admissible arm | O_near |
| heuristic | baseline | per core class, the one of {fixed_rule, always_reference, best_static} with the lowest calls-weighted total on VALIDATION pressure-0 decisions (ties in that order) | 0 |
| best_static, fixed_rule, always_reference | baselines (reported) | as in ORIG1-CONTRACT.md s.7; best_static from TRAIN | 0 |
| untrained_control | control | the candidate's declared init, no update | O_mlp |
| shuffled_label_control | control | the candidate's trainer on targets permuted among TRAIN pairs (control 1) | O_mlp |
| constant_control | control | a constant score: every arm ties, the frozen tie rule picks arm 0 | O_mlp |
| oracle | bound | cheapest admissible arm | 0 |

Commands (aien-train `feat/25-orig2-native-scorer` at `692d85104369cf0fc0bec9f35c4ca693cf746012`,
selmain ELF sha256 `01b6e63fe8a579a4db6300a9bba237e3dd30e143dba58e07781a9727ef00a62f`):

```sh
V2=1 RECORDS=<C2> ALLOW=<allow-v2.txt> OUT=<out> nice -n 19 sh tests/si19/run-select.sh
```

It trains the candidate and the shuffled and untrained controls once each and writes the
mode E per-record files `v2_eval_{mlp,shuffled,untrained}.records` (VALIDATION + HELD-OUT).

## 4. Overhead (per decision)

omega-mv `85641e1e`, `tests/orig4/run.sh` with `ORIG4_OVERHEAD=1 ORIG4_REPS=7
ORIG4_RECORDS=<C2> ORIG4_ALLOW=<allow-v2.txt> ORIG4_IMAGE=<candidate image>`: seven interleaved
repetitions of three shadow runs over the whole C2 file under v2 admission: disabled (kill
switch: process start and glue only), the candidate scorer, the retrieval advisor. Wall times
in microseconds. Per-decision overhead = (median wall of the advisor run - median wall of the
disabled run) / scored decisions, in ps. It contains record admission, feature extraction,
inference or nearest-neighbour search, argmin dispatch, the fallback checks and one
synchronous output line per decision; it overstates a resident in-process advisor (it also
pays admission of the file), and that bias is against the learned arms and retrieval
alike. The fixed rule, always_reference and best_static are charged 0 (a constant-time
comparison in C, not measured).

## 5. Primary criterion and verdict (frozen; `bench_orig3_judge_v2`)

Population, per core class c in {X925 = 0, A725 = 1}: HELD-OUT decisions at pressure 0.
Decision cost of arm p on decision r: `calls_r * ps_p(r) + O_p` (calls = the record's timed
calls, the consumer's batch per decision). Totals `C_p`. Comparator `C* = min(C_retrieval,
C_heuristic)`.

- P1(p, c): `C_p <= 0.99 * C*` (superiority margin 1 %).
- P2(p, c): exact one-sided sign test of per-decision cost against retrieval and against the
  heuristic separately (ties dropped), each with P(X >= wins | n discordant, 1/2) <= 0.0125
  (0.05 / 4 contrasts: 2 classes x 2 comparators).
- PRIMARY(p, c) = P1 and P2.
- INVALID: any control (untrained, shuffled-label, constant) meets PRIMARY on any class.
- GO: not INVALID and the candidate meets PRIMARY on both classes.
- NO-GO: not INVALID, not GO, and on some class `C_candidate > 1.01 * C*` (non-inferiority fails).
- INCONCLUSIVE: everything else.

Reported, not gated: choice-only (overhead-free) ratios to oracle and to the comparators,
held-out pressure-1 decisions, VALIDATION totals, best_static, fixed_rule, always_reference.

## 6. Cost accounting and amortization

Training wall (`v2_train_mlp` wall_ms) and collection wall (meta start to end) are reported
separately, never folded into the per-decision cost. Break-even decisions =
(training + collection wall, ps) / overhead-inclusive saving per held-out decision against
the cheaper comparator ("none" when the saving is not positive). Break-even calls per
decision = O_mlp / choice-only saving per call against the same comparator ("none" when not
positive): the batch size at which the scorer's overhead would be paid back.

## 7. Budget and stop rules

Training wall at most 30 min, collection wall at most 15 min, 705 parameters (no larger
model under this prereg), at most 2048 records, one collection, one training run per arm,
one evaluation, one overhead campaign. No retraining, no lr search, no record added after
the HELD-OUT outcomes are produced. A harness exit code other than 0, any digest mismatch, a
whole-file admission refusal, a missing arm or a budget overrun ends the campaign
INCONCLUSIVE with the reason; a new attempt needs a new prereg_id.
