#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-om.sh
stage=$(mktemp -d build/om/.test.XXXXXX)
trap 'rm -rf "$stage"' EXIT HUP INT TERM
cat om/prime/reference.om tests/om/reference.om > "$stage/reference.om"
"$OMC" < "$stage/reference.om" > "$stage/reference-test"
chmod 755 "$stage/reference-test"
rustc --edition=2021 -O -D warnings tests/om/reference_oracle.rs -o "$stage/oracle"
rustc --edition=2021 -O --test prime-drag-race/src/reference.rs -o "$stage/reference-rust-test"
"$stage/reference-rust-test" --include-ignored > "$stage/rust-tests.txt"
"$stage/oracle" > "$stage/expected"
"$stage/reference-test" > "$stage/actual"
cmp "$stage/expected" "$stage/actual"
printf 'PASS: full reference corpus matches Rust byte-for-byte\n'
expect() {
    result=$(printf '%s\n' "$1" | build/om/prime-reference)
    [ "$result" = "$2" ] || { echo "Wrong result for $1" >&2; exit 1; }
}
expect 'c 0' 0
expect 'c 1' 0
expect 'c 2' 1
expect 'c 10' 4
expect 'c 100' 25
expect 'c 1000' 168
expect 'c 10000' 1229
expect 'c 100000' 9592
expect 'c 1000000' 78498
expect 'c 10000000' 664579
expect 'p 3215031751' 0
expect 'p 18446744073709551557' 1
expect 'p 18446744073709551615' 0
reject() {
    code=0
    printf '%s' "$1" | build/om/prime-reference > "$stage/rejected" || code=$?
    [ "$code" = "$2" ] && [ ! -s "$stage/rejected" ] || {
        echo 'Invalid request was not refused cleanly' >&2
        exit 1
    }
}
reject '' 64
reject 'p ' 64
reject 'p -1' 64
reject 'p 18446744073709551616' 64
reject 'p 1 garbage' 64
reject 'p 0000000000000000000000000000000000000000000000000000000000000000' 64
reject 'z 10' 64
reject 'b 10000001' 65
reject 'c 18446744073709551615' 65
reject 'p 2
p 3
' 64
printf 'PASS: known prime counts, u64 boundaries, malformed input and bitmap limits\n'
cat "$stage/rust-tests.txt"
