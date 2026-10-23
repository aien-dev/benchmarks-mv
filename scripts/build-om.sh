#!/bin/sh
# Build with the exact qualified native Omega seed compiler. No system installation.
set -eu
cd "$(dirname "$0")/.."
: "${OMC:?Set OMC to the qualified native Omega compiler executable}"
expected=9c655bb98bc651f5dbada342df48eb9ef25a5aa19b4dec2a34c50174a8db26be
actual=$(sha256sum "$OMC" | cut -d ' ' -f 1)
[ "$actual" = "$expected" ] || { echo 'Unsupported Omega compiler identity' >&2; exit 65; }
[ "$(uname -sm)" = 'Linux aarch64' ] || { echo 'This compiler requires Linux ARM64' >&2; exit 69; }
mkdir -p build/om
stage=$(mktemp -d build/om/.compile.XXXXXX)
trap 'rm -rf "$stage"' EXIT HUP INT TERM
cat om/prime/reference.om om/prime/main.om > "$stage/prime.om"
"$OMC" < "$stage/prime.om" > "$stage/prime-reference"
readelf -h "$stage/prime-reference" | grep -q 'Machine:.*AArch64'
if readelf -l -d "$stage/prime-reference" | grep -Eq 'INTERP|NEEDED'; then
    echo 'Unexpected dynamic runtime dependency' >&2
    exit 65
fi
chmod 755 "$stage/prime-reference"
mv "$stage/prime-reference" build/om/prime-reference
printf 'Built native Omega prime reference\n'
