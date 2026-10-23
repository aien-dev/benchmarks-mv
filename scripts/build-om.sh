#!/bin/sh
# Build with the exact qualified native Omega seed compiler. No system installation.
set -eu
cd "$(dirname "$0")/.."
: "${OMC:?Set OMC to the qualified native Omega compiler executable}"
expected=e5d60db892c8f21cc8c9128af26d8bded0e96d7f92946f5a549397c43a872649
actual=$(sha256sum "$OMC" | cut -d ' ' -f 1)
[ "$actual" = "$expected" ] || { echo 'Unsupported Omega compiler identity' >&2; exit 65; }
[ "$(uname -sm)" = 'Linux aarch64' ] || { echo 'This compiler requires Linux ARM64' >&2; exit 69; }
mkdir -p build/om
stage=$(mktemp -d build/om/.compile.XXXXXX)
trap 'rm -rf "$stage"' EXIT HUP INT TERM
cat om/prime/reference.om om/prime/main.om > "$stage/prime.om"
"$OMC" < "$stage/prime.om" > "$stage/prime-reference"
cat om/data/sha256.om om/data/hash-main.om > "$stage/hash.om"
"$OMC" < "$stage/hash.om" > "$stage/om-sha256"
for program in prime-reference om-sha256; do
readelf -h "$stage/$program" | grep -q 'Machine:.*AArch64'
if readelf -l -d "$stage/$program" | grep -Eq 'INTERP|NEEDED'; then
    echo 'Unexpected dynamic runtime dependency' >&2
    exit 65
fi
chmod 755 "$stage/$program"
done
for program in prime-reference om-sha256; do
mv "$stage/$program" "build/om/$program"
done
printf 'Built native Omega prime reference and SHA-256 utility\n'
