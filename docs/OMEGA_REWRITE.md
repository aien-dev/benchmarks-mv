# Omega rewrite

Requested by Drake on 2026-10-23. Drake selected a full native rewrite and
authorized extending Omega first, with no outside implementation help. The
host extension is implemented and qualified. The benchmark migration remains
in progress; this branch does not yet replace the full Rust suite.

The final-state requirement is replacement and deletion, as recorded in
[architecture #300](https://github.com/aien-dev/aien-architecture-mv/issues/300).
After a native consumer passes qualification, remove its superseded first-party
implementation and obsolete dependencies from the active tree. This includes
build/release scripts, automation and project-owned tests. No permanent reference
directory, wrapper, foreign fallback or dual production path is allowed. Existing
Rust and shell sources are outstanding migration work. Historical source belongs
in Git history; immutable historical evidence is preserved as evidence.

The [migration inventory](om-migration.json) records the outstanding groups,
dependencies and deletion gates. None is marked retired merely because a library
function has a native counterpart.

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

The build also emits `build/om/om-sha256`: native SHA-256 of standard input,
with a declared 1 GiB input limit. It emits 64 lowercase hexadecimal digits and
a newline, exits 65 for oversized input, and 74 for I/O failure. Input failures
produce no digest. This executable has no helper process or shared library.
The installed checksum utility is used only to qualify padding/read boundaries
and to check the compiler identity during the still-unported shell build step.

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

## Native data and measurement foundation

`om/data/` now contains native JSON reading/writing, file I/O, SHA-256 and exact
integer timing statistics. The implementation invokes no foreign-language
runtime or helper. The current APIs are bounded and use one private instance
per process; they are not concurrent shared parser/hash state.

- JSON input, decoded strings and output each allow 1,000,000 bytes; the tree
  allows 131,072 tokens and at most 128 value levels. Limits are checked and
  refused, not silently truncated. These capacities cover every tracked JSON
  document, including the two large model manifests.
- Strings validate UTF-8, reject unpaired UTF-16 surrogate escapes, and decode
  escaped NUL without confusing it with an end marker. Object lookup selects
  the last duplicate direct key, matching the existing map behavior.
- Numeric spellings are preserved exactly. Unsigned integer conversion checks
  the full 64-bit range. The syntax reader accepts JSON numbers such as `1e9999`;
  this is not a claim that they fit a finite metric. Schema checks and decimal
  arithmetic remain consumer work. Negative, fractional and exponent spellings
  are explicitly refused by the unsigned-integer accessor.
- The writer emits compact JSON and preserves key order, duplicates and number
  spelling. It does not implement serde's pretty layout or claim byte-for-byte
  equivalence with historical receipt serialization.
- File output exclusively creates mode-0444 files, writes all bytes, syncs and
  checks close. Existing files are refused. A failed write may leave a partial
  file; publication/commit recovery for content-addressed receipts remains open.
- Timing samples retain integer nanoseconds. Up to 8,192 observations yield
  exact min/max, mean quotient/remainder, median whole/half and nearest-rank
  p50/p95. P95 requires at least 20 samples, as in the streaming implementation.
  The caller must exclude failed requests. Rates, standard deviation, coefficient
  of variation and final metric formatting remain unported.
- SHA-256 is incremental, with checked message length and explicit finalize/reset
  state. The native checksum utility now checks the frozen prime reference corpus.

Qualification includes native syntax and Unicode rejection cases, all prefixes
of a nested record, exact and exceeded capacity limits, maximum integer values,
all permutations of four timing observations, sorting/rank checks for lengths
1 through 128, maximum-capacity samples, standard hash vectors and boundary
comparisons with the installed independent checksum oracle. All tracked JSON
documents are parsed, serialized, reparsed and serialized again with stable
bytes. This checks syntax and round trips, not the record schema; deliberately
schema-invalid fixtures must still parse.

`tests/om/measurement.om` runs 20 real prime-reference calculations, verifies
the known count, records native monotonic durations, calculates percentiles,
saves a private qualification record and checks its bytes with native hashing
on readback. It also refuses an overwrite and detects an altered metric through
both a value assertion and a digest mismatch. The temporary
`omega-bench-qualification/1` record is test evidence only, not a replacement for
the frozen Prime Drag Race receipt or a published performance result.

Three compiled mutants (wrong hash constant, wrong percentile, accepted unpaired
surrogate) must fail specific runtime assertions. Build failures do not count as
caught mutants. All tests are compiled `.om`; `scripts/test-om.sh` still coordinates
them and is itself an outstanding replacement task.

The compiler's same-lineage self-rebuild and regression checks establish the
reported reproducibility results. Independent compiler ancestry correspondence
under [Omega #650](https://github.com/aien-dev/omega-mv/issues/650) is **NOT_RUN**;
matching self-rebuild hashes are not substituted for that separate qualification.
No durable reuse system is claimed by this work; [Omega #651](https://github.com/aien-dev/omega-mv/issues/651)
remains with its existing compiler/runtime owners.

## Remaining migration

| Existing component | Required work |
| --- | --- |
| Prime race runner, validation and receipts | Port declarations, trial scheduling, process execution, report parsing, statistics and content-addressed evidence storage. |
| `llm-stream` | Port HTTP/SSE parsing and accounting; provide sockets, monotonic timing, deadlines and concurrency. Preserve usage-only token counting and failure exclusion. |
| `measure` | Port system/memory sampling and endpoint workloads; replace in-process Rust runtime dependencies at an explicit boundary. |
| `verify`, `report`, `generate`, `sync-docs` | Integrate native JSON; port schema and budget validation, decimal metrics, tables, SVGs and document updates. |
| `bench_verify_records`, `bench_index` | Port schema and hash checks, directory traversal, source identity and Markdown generation. |
| `bench_branching_neutral`, `bench_canonical_suite` | Replace direct use of the five sibling Rust crates without changing measured work. |
| Build and verification gates | Port scripts and project-owned tests to `.om`; switch qualified consumers, then delete superseded code and dependencies. Existing gates still test Rust. |

Omega now supplies 32 native host operations for files, directories, clocks,
sockets, polling, process launch/wait, pipes and command-line arguments. The
implementation is in its own `seed/omc/omc.om`; the seed chain builds it and
proves byte-identical self-reproduction. Native tests cover success and failure
paths, TCP loopback, 18 negative controls and preservation of 70 original
compiler fixtures. See the pinned compiler's `seed/host/README.md` and
`docs/one_layer/OMEGESE.md` section 10 for exact contracts and limits.

No temporary host adapter was added. Remaining work includes JSON schema checks,
decimal metrics, the command implementations, and the boundary to the
sibling runtime. The compiler still has integer types only; floating-point
types and direct Rust calls are not claimed as implemented.

No historical records, benchmark claims, runtime services or frozen contracts
are changed by this port. The host-extension decision is settled; the remaining
benchmark commands still require implementation, consumer qualification and
deletion of the superseded implementation and dependencies.
