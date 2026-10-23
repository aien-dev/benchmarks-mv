# Omega rewrite

Requested by Drake on 2026-10-23. This branch starts the rewrite in `.om`.
It does not yet replace the Rust suite or its live measurements.

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

The program reads one request from standard input and writes the result to
standard output:

| Request | Result |
| --- | --- |
| `p NUMBER` | `1` if prime, otherwise `0`, followed by a newline |
| `c LIMIT` | Decimal prime count followed by a newline |
| `b LIMIT` | Canonical bitmap bytes, without text framing |

Input is unsigned decimal, with one optional final newline, and at most 63
bytes. Exit codes: 0 success, 64 malformed input, 65 unsupported bitmap limit,
74 input/output failure. Malformed input produces no result. This is a new
reference utility, not a replacement for the `prime_race run` interface.

## Build and check

The compiler is pinned to the native Omega seed built from
[`29b12760`](https://github.com/aien-spark/omega/commit/29b12760f4c962606411d6bf25032aedef063301).
That commit exists on GitHub but is not the default branch of `omega-mv`.
The build therefore requires an explicitly supplied compiler; it does not
pretend a fresh checkout of the current public default branch provides one.

Compiler SHA-256:
`191b776ef98821597c3e30655da6d680a9b66edbc9ceaca3c39c61ea6df098e9`.

From a checkout of the pinned Omega source, `make omega-from-seed` produces
`seed/out/omc`. From this repository on Linux ARM64:

```sh
OMC=/absolute/path/to/omega/seed/out/omc sh scripts/build-om.sh
printf 'c 1000000\n' | build/om/prime-reference
OMC=/absolute/path/to/omega/seed/out/omc quiet-build sh scripts/test-om.sh
```

`quiet-build` is the Spark build coordinator. On another Linux ARM64 host, run
the test script directly. The test script also needs `rustc` for the independent
reference. Rust is a test oracle only; the new executable neither links Rust
nor launches it. The build checks the compiler hash and rejects executables
with a dynamic loader or shared-library dependencies.

The test compares native output byte for byte with the existing Rust reference:
100,001 small primality decisions, 100 values near the 64-bit maximum, 10,000
modular products and powers, all bitmaps for limits 0 through 2,000, and the
100,000, 1,000,000 and 10,000,000 limits. It also checks known prime counts,
the four-base pseudoprime, malformed requests, overflow, wrong bitmap lengths,
missing primes, extra composites and tail bits. All six original reference
tests run, including the normally ignored scaling test.

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

The inspected seed compiler supports integer arithmetic, bounded arrays and
loops, functions, and `read`/`write` on existing file descriptors. It does not
expose file opening, directory traversal, sockets, a monotonic clock, process
launch/wait, command-line arguments, floating-point arithmetic or direct calls
to the existing Rust runtime. Its compiler source, `seed/omc/omc.om`, is the
capability evidence. A full native replacement requires extending Omega's host
facilities and qualifying those changes. A temporary host adapter is a separate
design choice, not a completed `.om` rewrite.

No historical records, benchmark claims, runtime services or frozen contracts
are changed by this first port. Host extensions and a temporary adapter are
both unimplemented; the remaining migration requires choosing that boundary.
