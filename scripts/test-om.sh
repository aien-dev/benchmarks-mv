#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-om.sh
stage=$(mktemp -d build/om/.test.XXXXXX)
trap 'rm -rf "$stage"' EXIT HUP INT TERM
cat om/data/sha256.om tests/om/sha256.om > "$stage/sha256.om"
"$OMC" < "$stage/sha256.om" > "$stage/sha256"
chmod 755 "$stage/sha256"
"$stage/sha256"
# The installed checksum utility is an independent qualification oracle only.
# Neither the Omega hash implementation nor its executable invokes it.
for bytes in 1 55 56 63 64 65 119 120 127 128 129 65535 65536 65537; do
    head -c "$bytes" /dev/zero > "$stage/hash-input"
    expected=$(sha256sum "$stage/hash-input" | cut -d ' ' -f 1)
    actual=$(build/om/om-sha256 < "$stage/hash-input")
    test "$actual" = "$expected"
done
printf 'PASS: native SHA-256 standard vectors, padding and read-boundary checks\n'
for suite in json-api json-grammar json-write json; do
    cat om/data/json.om om/data/json-write.om "tests/om/$suite.om" > "$stage/$suite.om"
    "$OMC" < "$stage/$suite.om" > "$stage/$suite"
    chmod 755 "$stage/$suite"
    if [ "$suite" != json ]; then
        "$stage/$suite"
    fi
done
cat om/data/stats.om tests/om/stats.om > "$stage/stats.om"
"$OMC" < "$stage/stats.om" > "$stage/stats"
chmod 755 "$stage/stats"
"$stage/stats"
printf 'PASS: native JSON grammar, Unicode, exact counts, writing, limits and timing statistics\n'
# Schema-invalid fixtures remain valid JSON. This gate checks syntax, not their schema.
count=0
git ls-files '*.json' > "$stage/json-files"
while IFS= read -r record; do
    "$stage/json" < "$record" > "$stage/first.json"
    "$stage/json" < "$stage/first.json" > "$stage/second.json"
    cmp "$stage/first.json" "$stage/second.json"
    count=$((count + 1))
done < "$stage/json-files"
printf 'PASS: %s tracked JSON documents read and written twice with stable bytes\n' "$count"
cat om/data/json.om om/data/json-write.om om/data/json-file.om om/data/stats.om om/data/sha256.om \
    om/prime/reference.om tests/om/measurement.om > "$stage/measurement.om"
"$OMC" < "$stage/measurement.om" > "$stage/measurement"
chmod 755 "$stage/measurement"
(cd "$stage"; ./measurement)
test "$(stat -c %a "$stage/measured.json")" = 444
printf 'PASS: native workload, clock samples, saved JSON, readback and altered-metric refusal\n'
# Compiled wrong implementations must fail named runtime assertions, not merely fail to build.
sed 's/428a2f98/428b2f98/' om/data/sha256.om > "$stage/hash-mutant.om"
cat tests/om/sha256.om >> "$stage/hash-mutant.om"
"$OMC" < "$stage/hash-mutant.om" > "$stage/hash-mutant"
chmod 755 "$stage/hash-mutant"
code=0
"$stage/hash-mutant" || code=$?
test "$code" = 2
sed 's/mul n 95/mul n 94/' om/data/stats.om > "$stage/stats-mutant.om"
cat tests/om/stats.om >> "$stage/stats-mutant.om"
"$OMC" < "$stage/stats-mutant.om" > "$stage/stats-mutant"
chmod 755 "$stage/stats-mutant"
code=0
"$stage/stats-mutant" || code=$?
test "$code" = 17
sed 's/if and ge cp 56320 le cp 57343/if false/' om/data/json.om > "$stage/json-mutant.om"
cat tests/om/json-grammar.om >> "$stage/json-mutant.om"
"$OMC" < "$stage/json-mutant.om" > "$stage/json-mutant"
chmod 755 "$stage/json-mutant"
code=0
"$stage/json-mutant" || code=$?
test "$code" = 1
printf 'PASS: compiled hash, percentile and Unicode mutants fail their named assertions\n'
cat om/prime/reference.om tests/om/reference.om > "$stage/reference.om"
"$OMC" < "$stage/reference.om" > "$stage/reference-test"
chmod 755 "$stage/reference-test"
cp tests/om/reference.sha256 "$stage/reference.sha256"
"$stage/reference-test" > "$stage/reference.out"
expected=$(cut -d ' ' -f 1 "$stage/reference.sha256")
actual=$(build/om/om-sha256 < "$stage/reference.out")
test "$actual" = "$expected"
printf 'PASS: native trial division and the previously verified reference corpus\n'
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
test "$(build/om/prime-reference c 1000000)" = 78498
test "$(build/om/prime-reference p 18446744073709551557)" = 1
build/om/prime-reference b 10000 > "$stage/stdout.bitmap"
build/om/prime-reference b 10000 "$stage/native.bitmap"
cmp "$stage/stdout.bitmap" "$stage/native.bitmap"
(build/om/prime-reference b 10000 "$stage/closed-stdout.bitmap" 1>&-)
cmp "$stage/stdout.bitmap" "$stage/closed-stdout.bitmap"
code=0
build/om/prime-reference b 20 "$stage/native.bitmap" || code=$?
test "$code" = 74
cmp "$stage/stdout.bitmap" "$stage/native.bitmap"
code=0
build/om/prime-reference c 10 "$stage/unwanted" || code=$?
test "$code" = 64
test ! -e "$stage/unwanted"
printf 'PASS: native arguments and file output, including refusal to overwrite\n'
