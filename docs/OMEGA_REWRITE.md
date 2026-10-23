# Omega rewrite

Requested by Drake on 2026-10-23. Drake selected a full native rewrite and
authorized extending Omega first, with no outside implementation help. The
host extension is implemented and qualified. The benchmark migration remains
in progress; this branch does not yet replace the full Rust suite.

## Working native replacement

`om/prime/reference.om` implements the Prime Drag Race correctness reference:
deterministic Miller-Rabin, overflow-safe modular multiplication, canonical
little-endian bitmaps, prime counts and first-differing-bit checks. Its bitmap
limit is 10,000,000, covering the frozen contract's scaling check. Larger bitmap
requests fail explicitly; individual primality checks cover the full unsigned
64-bit range.

Omega has no 128-bit integer type in this compiler. The port computes the same
modular result with overflow-safe addition and doubling when a 64-bit product
would overflow. This preserves the reference answer; it makes no performance
claim and does not change the frozen benchmark contract.

The program accepts native command-line arguments or one request from standard
input and writes the result to standard output:

| Request | Result |
| --- | --- |
| `p NUMBER` | `1` if prime, otherwise `0`, followed by a newline |
| `c LIMIT` | Decimal prime count followed by a newline |
| `b LIMIT` | Canonical bitmap bytes, without text framing |

`build/om/prime-reference b LIMIT NEW_FILE` writes the bitmap through Omega's
native file operations. It creates a private file exclusively, syncs it, and
checks close status. Existing files are refused. An I/O failure returns 74;
a file created before that failure may contain partial output.

Input is unsigned decimal, with one optional final newline, and at most 63
bytes. Exit codes: 0 success, 64 malformed input, 65 unsupported bitmap limit,
74 input/output failure. Malformed input produces no result. This is a new
reference utility, not a replacement for the `prime_race run` interface.

## Build and check

The compiler is pinned to the native host extension at
[`42f89c3b`](https://github.com/aien-dev/omega-mv/commit/42f89c3bf04265ac40384687cffb109e541c6ebe).
That commit exists on GitHub but is not the default branch of `omega-mv`.
The build therefore requires an explicitly supplied compiler; it does not
pretend a fresh checkout of the current public default branch provides one.

Compiler SHA-256:
`9c655bb98bc651f5dbada342df48eb9ef25a5aa19b4dec2a34c50174a8db26be`.

From a checkout of the pinned Omega source, `sh seed/omega-from-seed.sh` produces
`seed/out/omc`. From this repository on Linux ARM64:

```sh
OMC=/absolute/path/to/omega/seed/out/omc sh scripts/build-om.sh
printf 'c 1000000\n' | build/om/prime-reference
build/om/prime-reference c 1000000
build/om/prime-reference b 1000000 /path/to/new-bitmap.bin
OMC=/absolute/path/to/omega/seed/out/omc quiet-build sh scripts/test-om.sh
```

`quiet-build` is the Spark build coordinator. On another Linux ARM64 host, run
the test script directly. The default tests now use native Omega only, with
shell orchestration and file/hash checks. No Rust compiler or test helper is
invoked. The build checks the compiler hash and rejects executables
with a dynamic loader or shared-library dependencies.

The test checks the fixed digest of the previously verified reference corpus:
100,001 small primality decisions, 100 values near the 64-bit maximum, 10,000
modular products and powers, all bitmaps for limits 0 through 2,000, and the
100,000, 1,000,000 and 10,000,000 limits. It also checks known prime counts,
the four-base pseudoprime, malformed requests, overflow, wrong bitmap lengths,
missing primes, extra composites and tail bits. Independent native trial
division checks every primality decision through 100,000 and every bitmap
through limit 2,000. Native argument and file-output checks verify exact bytes
and refusal to overwrite an existing file.

Digest provenance: benchmark commit `7821a829e398870ba2e77fdcc22c5cac94987ece`
was tested byte for byte against the Rust reference, with all six original
reference tests passing. Its unmodified Omega corpus was replayed with the
original native compiler (SHA-256 `191b776ef98821597c3e30655da6d680a9b66edbc9ceaca3c39c61ea6df098e9`)
to freeze `tests/om/reference.sha256`. Subsequent native-only runs must match
that exact output. A digest change requires investigating the changed result,
not simply regenerating the expected digest.

These are correctness tests. They are not new hardware performance results.

## Remaining migration

| Existing component | Required work |
| --- | --- |
| Prime race runner, validation and receipts | Port declarations, trial scheduling, process execution, report parsing, statistics and content-addressed evidence storage. |
| `llm-stream` | Port HTTP/SSE parsing and accounting; provide sockets, monotonic timing, deadlines and concurrency. Preserve usage-only token counting and failure exclusion. |
| `measure` | Port system/memory sampling and endpoint workloads; replace in-process Rust runtime dependencies at an explicit boundary. |
| `verify`, `report`, `generate`, `sync-docs` | Port JSON handling, budget validation, decimal metrics, tables, SVGs and document updates. |
| `bench_verify_records`, `bench_index` | Port schema and hash checks, directory traversal, source identity and Markdown generation. |
| `bench_branching_neutral`, `bench_canonical_suite` | Replace direct use of the five sibling Rust crates without changing measured work. |
| Build and verification gates | Switch only after corresponding replacements pass parity and failure tests. Existing gates still test Rust. |

Omega now supplies 32 native host operations for files, directories, clocks,
sockets, polling, process launch/wait, pipes and command-line arguments. The
implementation is in its own `seed/omc/omc.om`; the seed chain builds it and
proves byte-identical self-reproduction. Native tests cover success and failure
paths, TCP loopback, 18 negative controls and preservation of 70 original
compiler fixtures. See the pinned compiler's `seed/host/README.md` and
`docs/one_layer/OMEGESE.md` section 10 for exact contracts and limits.

No temporary host adapter was added. Remaining work includes JSON handling,
metric formatting, the command implementations, and the boundary to the
sibling runtime. The compiler still has integer types only; floating-point
types and direct Rust calls are not claimed as implemented.

No historical records, benchmark claims, runtime services or frozen contracts
are changed by this port. The host-extension decision is settled; the remaining
benchmark commands still require implementation and qualification.
