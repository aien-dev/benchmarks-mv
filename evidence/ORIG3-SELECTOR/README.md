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

# ORIG-3 sealed evaluation of the scratch selector (benchmarks-mv#45)

Preregistration: `manifests/prereg/orig3_selector.json` (prereg_id
`orig3-selector-prereg-2026-10-25`). Protocol: `docs/orig3/PROTOCOL.md`.

ORIG3_VERDICT=INCONCLUSIVE (preregistered rule applied verbatim, `judge.out`).

Real-record evidence only; no fixture numbers appear here. Hardware timings of a public
harness, no personal data. The record files themselves stay local
(`~/.local/state/aien-train/orig1/` training, `~/.local/state/aien-train/orig3/` sealed);
their hashes are in the prereg and in `collection-*.meta`.

## Identities

| item | value |
|---|---|
| candidate mlp_scratch image | sha256 8dbd9b04a446b02d8e7bf2f1e2aa808ca439e813abfcb52fb34e57d40538acb5, 11560 B, image fnv 9232652764948363499, aien-train 745922dc (2026-10-25T04:21:26Z) |
| affine image | sha256 f04bc778b9347daa10a175a6d2a300a3cc513f6efc53e5412361ba1e9804121b |
| shuffled_control image | sha256 918747d38badb0482d3d4dd17a5bf7988a034198bc5f20a79bfabe2637e81c4b |
| untrained_control image | sha256 8ad6de5392c253163ebaeb3122873100cac392bc016d6ab16130af8d6e8a26e9 |
| evaluator | selmain unit source sha256 715ad950a47d3d484d3c10d63ba2b4b524bc0f63c954520c59f2e2b91a475cc0, omc 191b776e, ELF sha256 9b2d63ed23b04ec1683c1b58fa93b9351cf55f772cae06fefac57db9e6884002 |
| training records | 738 records, fnv1a64 16622604375676129865, sha256 403156f5ca34479715d2f96fabcbd11b9cab3fd4668e9dd3a41a6eb89b128d37, collected 2026-10-25T03:47:37Z to 03:48:39Z |
| SEALED-FINAL records | 738 records (188928 B), fnv1a64 12602815733741495377, sha256 99da7bd92c7591265a6044e96aff38d53cd0e2e91457af49002d429e682f7fd7, collected 2026-10-25T04:24:09Z to 04:25:10Z at omega-mv 1ec81372 (tree clean, binary sha256 d26aec0296c77b39d0c5147dd60e4be936d5b032c127d9eb0928a9d7e76f81ee); harness receipt 20221 digest-checked runs, 0 mismatches, 44 checks 0 failures |

## Sealed result, H1 + H2 per core class (n = 120 each), total chosen ps / oracle ps

| arm | Cortex-X925 | Cortex-A725 |
|---|---|---|
| mlp_scratch (candidate) | 1.0002 | 1.0019 |
| affine | 1.0003 | 1.0908 |
| fixed_rule (incumbent) | 1.7053 | 1.7967 |
| always_reference | 1.6292 | 1.1412 |
| best_static | 1.0116 | 1.0285 |
| retrieval | 1.0002 | 1.0025 |
| shuffled_control | 1.0143 | 1.1809 |
| untrained_control | 1.2175 | 1.0479 |

Criteria (both classes): A (<= 0.95 x rule) HOLD (0.587, 0.558); B (<= reference) HOLD;
C (<= affine) HOLD (0.9999, 0.9185); D (sign test vs rule 120/0 >= 70) HOLD; E (<= min(static,
retrieval)) HOLD (1.0000, 0.9995); F (controls fail A) FAIL on both: shuffled 0.595/0.657 and
untrained 0.714/0.583 of the fixed rule. Level 2 NO-GO does not trigger (mlp <= 1.01 x
min(static, retrieval) on both). Verdict INCONCLUSIVE.

Reported, not gated: sign test vs retrieval X925 +0/-0 (identical choices on all 120 records),
A725 +6/-12 (not significant); vs best_static X925 +24/-0, A725 +62/-5 (significant). H3
(pressure) mlp/oracle 1.0000 on X925, 1.0355 on A725 (best_static 1.0085, retrieval 1.0039).
Energy NOT_MEASURED (no meter). GB10 NOT_RUN.

## Reading

The scratch MLP learns, beats the incumbent, the reference and the affine baseline, and is at
parity with retrieval and better than best_static on the sealed set. It is NOT shown superior
to the cheap retrieval heuristic. The preregistered rule was mis-specified in F: the incumbent
fixed rule is so far from the oracle (1.71 to 1.80x) that any arm-4-leaning chooser, including
the untrained and shuffled controls, beats it by more than 5%, so criterion A carries no
evidential weight and F cannot hold. A superseding prereg (v2) should replace A and F with
"controls must fail E" and gate the primary comparison on retrieval. v2 is proposed, NOT run:
running it on the same sealed records would be post hoc.

This evidence does not authorize deployment and does not close benchmarks-mv#39 or
aien-train#19.

Review status: UNISOLATED. The evaluator is the same operator fleet (omp session) as the
trainer; the sealing is temporal (the sealed records were collected after the candidate was
committed), not organisational.

## Files

- `sealed-{mlp,affine,shuffled,untrained}.records`: selmain mode E per-record lines (fd 4),
  columns `seq fam core M N model rule ref static near oracle cost_model cost_oracle cost_rule
  cost_ref cost_static cost_near`; families 1 (DEV, from the harness's own run) and 2 to 4.
- `sealed-*.stdout`: the evaluator report (key=value lines and `eval part= core=` totals).
- `sealed-runs.txt`: exit code, wall ms and image sha256 per arm.
- `allow-sealed.txt`, `sealed.fnv`: the allow-list line and fnv used for admission.
- `judge.out`: output of `docs/orig3/judge.sh` over the four `.records` files;
  `cargo run --release --bin bench_orig3_judge -- evidence/ORIG3-SELECTOR/sealed-{mlp,affine,shuffled,untrained}.records`
  reproduces it byte for byte (`tests/orig3_judge.rs`).
- `collection-train-*.meta`, `collection-sealed-*.meta`, `collection-*.harness.stdout`: the two
  tap runs (training file and sealed file) and the harness stdout of each.
- `SHA256SUMS`: `sha256sum -c SHA256SUMS` from this directory.
