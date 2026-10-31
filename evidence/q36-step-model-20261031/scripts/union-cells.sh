#!/usr/bin/env bash
# Per-cell routed-expert union counts across N host goldens, for the Wolfram fit (DRA-64).
# Same cell definition as the pinned inputs/expert-union.sh (aien-sovereign-core-mv d3dec0eb,
# evidence/motor-chip-20261021/scripts/expert-union.sh): decode steps 1.. aligned by index, every layer,
# |union of the N rows' routed expert ids|. It emits the integer cell list (step-major, layer-minor) instead of
# the mean, so the mean is computed by Wolfram, not by jq. The shared expert is not in the routes; the model adds it.
# usage: union-cells.sh gen-a.json gen-b.json ...
set -eu
jq -s -c '
  (map(.routes | length) | min) as $steps
  | (.[0].routes[0] | length) as $layers
  | {n: length, decode_steps: ($steps - 1), layers: $layers,
     cells: [range(1; $steps) as $s | range(0; $layers) as $l | ([.[] | .routes[$s][$l][]] | unique | length)]}
' "$@"
