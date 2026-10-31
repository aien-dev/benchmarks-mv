#!/usr/bin/env bash
# Weight bytes per launch class of the Qwen3.6-35B-A3B FP8 checkpoint, read from the safetensors headers only
# (data_offsets end - start per tensor; no tensor data is read, no model is loaded). Layers 0-39 and lm_head;
# the MTP file (mtp.safetensors) is excluded. Classes follow the lever-2 stamp kinds of the stroke timeline:
#   expert   one routed expert: experts.0.{gate,up,down}_proj.weight + .weight_scale_inv (every routed expert has the same shapes)
#   shared   the shared expert: shared_expert.{gate,up,down}_proj.weight + .weight_scale_inv
#   gate_up  the gate+up part of one routed expert, down the down part
#   mix      linear_attn.in_proj_{qkv,z} and self_attn.{q,k,v}_proj, FP8 weight + scale
#   o_proj   linear_attn.out_proj and self_attn.o_proj, FP8 weight + scale
#   ab       linear_attn.in_proj_{a,b} (bf16)
#   router   mlp.gate.weight (bf16 256 x 2048)
#   lm_head  lm_head.weight (bf16, outside.safetensors)
# Per-layer classes are summed over layers 0-39. Every value is an integer byte count read from a header.
# usage: header-bytes.sh CHECKPOINT_DIR > derived/header-bytes.json
set -eu
dir=$1
hdr() { # print the JSON header of a safetensors file
  local n
  n=$(od -An -t u8 -N8 "$1" | tr -d ' ')
  tail -c +9 "$1" | head -c "$n"
}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for l in $(seq 0 39); do hdr "$dir/layers-$l.safetensors" > "$tmp/l$l.json"; done
hdr "$dir/outside.safetensors" > "$tmp/outside.json"
shas=$(for f in "$tmp"/l*.json "$tmp"/outside.json; do printf '%s %s\n' "$(basename "$f")" "$(sha256sum < "$f" | cut -c1-64)"; done \
  | jq -R -s -c 'split("\n") | map(select(length > 0) | split(" ") | {(.[0]): .[1]}) | add')
jq -s -c --argjson shas "$shas" --arg config_sha "$(sha256sum < "$dir/config.json" | cut -c1-64)" '
  def sz($h; $re): [$h | to_entries[] | select(.key != "__metadata__") | select(.key | test($re))
                    | .value.data_offsets[1] - .value.data_offsets[0]] | add // 0;
  def per_layer($re): [.[0:40][] as $h | sz($h; $re)] | add;
  {
    source: "safetensors headers of layers-0..39.safetensors and outside.safetensors; mtp.safetensors excluded",
    config_sha256: $config_sha,
    header_sha256: $shas,
    layers: 40,
    expert_per_layer_first: sz(.[0]; "\\.mlp\\.experts\\.0\\.(gate|up|down)_proj\\.weight"),
    expert: (per_layer("\\.mlp\\.experts\\.0\\.(gate|up|down)_proj\\.weight") / 40),
    gate_up: (per_layer("\\.mlp\\.experts\\.0\\.(gate|up)_proj\\.weight") / 40),
    down: (per_layer("\\.mlp\\.experts\\.0\\.down_proj\\.weight") / 40),
    shared: (per_layer("\\.mlp\\.shared_expert\\.(gate|up|down)_proj\\.weight") / 40),
    routed_expert_tensor_sets_layer0: ([.[0] | keys[] | select(test("\\.mlp\\.experts\\.[0-9]+\\.down_proj\\.weight$"))] | length),
    mix: per_layer("\\.(linear_attn\\.in_proj_(qkv|z)|self_attn\\.(q|k|v)_proj)\\.weight"),
    o_proj: per_layer("\\.(linear_attn\\.out_proj|self_attn\\.o_proj)\\.weight"),
    ab: per_layer("\\.linear_attn\\.in_proj_(a|b)\\.weight$"),
    router: per_layer("\\.mlp\\.gate\\.weight$"),
    lm_head: sz(.[40]; "^lm_head\\.weight$")
  }' $(for l in $(seq 0 39); do printf '%s ' "$tmp/l$l.json"; done) "$tmp/outside.json"
