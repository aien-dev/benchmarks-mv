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

# ORIG-3 evaluator protocol (benchmarks-mv#45)

The preregistration is `manifests/prereg/orig3_selector.json`; it is checked by
`cargo run --release --bin bench_verify_prereg -- manifests/prereg`. This page is the
operator runbook for the phases `checkpoint_pin`, `sealed_tap` and `sealed_eval` of that
file. Nothing here is evidence: receipts live with the run that produced them.

Status words: PASS, FAIL, INCONCLUSIVE, NOT_RUN, BLOCKED. Decision words: GO, NO-GO,
INCONCLUSIVE. Host-only results are development results, never hardware qualification.
GB10 is NOT_RUN; energy is NOT_MEASURED.

## 1. Preconditions

1. The prereg is FROZEN (every `turing_profile.G.background` revision pinned).
2. The candidate checkpoint (mlp_scratch image) and the affine image are final; their
   sha256 and in-image FNV-1a 64 are committed under `docs/orig3/` together with the
   training file FNV-1a 64. A checkpoint retrained after this commit is a new prereg_id.
3. The five baseline receipts exist (`baseline_before_eval`).
4. The host is uncontended: no other measurement, no model load, no GPU experiment. The
   resident service on port 18091 is left alone; the tap runs on host cores only.

## 2. Sealed tap (one run, executed only after step 1)

In an omega-mv checkout at the tap commit `1ec813726aef5f5abf8ab449c5d8bd9738f88d2e`
(branch `feat/731-orig4-shadow-advice`), with the aienos lock repository available:

```sh
git checkout 1ec813726aef5f5abf8ab449c5d8bd9738f88d2e
make AIENOS_LOCK_REPO=$HOME/workspace/aienos-b84c0a67 build/rx_empirical_optimizer_test
sha256sum build/rx_empirical_optimizer_test
mkdir -p -m 0700 $HOME/.local/state/aien-train/orig1/sealed
OMEGA_EMPIRICAL_RECORDS=$HOME/.local/state/aien-train/orig1/sealed/records-<UTC stamp>.bin \
EMPIRICAL_ALLOW_NO_WIN=1 ./build/rx_empirical_optimizer_test
```

Record in `run-<stamp>.meta` next to the file: source commit, whether the tree was dirty,
the binary sha256, start and end timestamps, exit code. The record file stays mode 0600 and
local; only its FNV-1a 64, sha256 and per-family, per-core record counts are published.

Expected content per core class present: 60 H1 (family 2), 60 H2 (family 3) and 60 H3
(family 4) records, plus the harness's own train and validation families (0 and 1), which
the evaluation ignores. One run is about 64 s of host wall; that cost is counted once in the
cost ledger.

Then add one line `<decimal fnv> AUTHORIZED orig3-sealed-final-<stamp>` to the allow-list.
The file is single use: it is scored once per arm and never used for training, lr choice
or early stopping. If admission refuses it (any `oxload` status below 0) the set is void
(INCONCLUSIVE) and the reserve rule of the prereg applies: one more run, then stop.

## 3. Sealed evaluation (.om)

Evaluation is the `selmain` driver of aien-train (`native/si19/selmain.om`) in mode `E`
(first stdin byte `E`), compiled as one concatenated unit with the seeded compiler whose
sha256 is `191b776ef98821597c3e30655da6d680a9b66edbc9ceaca3c39c61ea6df098e9` (the unit must
also compile unchanged under the R6 compiler `734f0a39`). The stdin layout follows the
aien-train#25 driver interface: config word block, allow-list, the pinned image, then the
sealed records to EOF; `sleval` totals go to stdout per partition and core class, and
per-record lines go to fd 4. The exact invocation is fixed by aien-train#25 and is copied
here verbatim when `orig2_scorer_and_trainer` is pinned; until then this is the shape:

```sh
OMC=/path/to/omc-191b776e
cat deps/add32.om deps/mul32.om deps/metadata.om deps/affine.om \
    native/si19/fixture.om native/si19/world.om native/si19/omexp.om \
    native/si19/omexpfix.om native/si19/select.om native/si19/selmain.om > eval.om
"$OMC" < eval.om > eval.elf && chmod +x eval.elf
{ printf 'E'; cat config.bin allow.txt mlp_scratch.img sealed.bin; } | ./eval.elf 4>per-record.txt > totals.txt
```

Run it once per arm image (mlp_scratch, affine, untrained_control, shuffled_control,
no_update_control). fixed_rule, always_reference, best_static, retrieval and the oracle are
columns of the same per-record output (`rule`, `ref`, `static`, `near`, `oracle`), so they
need no image. The exit code must be 0; 65 (image refused), 66 (dataset refused) or 67
(unauthorized) voids the run for that arm and is reported as such.

## 4. Decision

Apply `metrics.decision_rule` of the prereg on SEALED-FINAL H1 + H2 per core class:

- A: mlp_scratch total ps <= 0.95 x fixed_rule total ps, every core class, each of H1 and H2.
- B: <= always_reference overall. C: <= affine overall.
- D: one-sided exact sign test vs fixed_rule over discordant records: at least 37 wins of
  60, at least 70 of 120 (smallest k with `1 - CDF[BinomialDistribution[n, 1/2], k - 1] <
  1/20`; computed with the Wolfram Language evaluator, see the prereg `sign_test` block).
- E: neither untrained_control nor shuffled_control satisfies A; no_update_control is
  bit-identical to untrained_control (mode `C`, exit 0).

GO when A to E all hold. NO-GO when A, B and C all fail and the mirrored sign test is
significant for fixed_rule. Anything else is INCONCLUSIVE. H3 (pressure) and the
development TEST families of the training run are reported under their own headings and
never enter the decision. The optional `existing_c_costmodel` comparison, if its per-record
choice is recoverable, is reported with its own cost column and is never a teacher.

## 5. Report

One status per claim in the prereg `claims` list. Keep negative, void and inconclusive
results. State on every report that the evaluator and the trainer are the same owner and
that sealing is temporal (the records did not exist before the checkpoint pin), which is
weaker than an independent judge.
