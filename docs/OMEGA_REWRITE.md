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
[`f0499f41`](https://github.com/aien-dev/omega-mv/commit/f0499f41).
That commit is on the native-host development branch, not the default branch of `omega-mv`.
The build therefore requires an explicitly supplied compiler; it does not
pretend a fresh checkout of the current public default branch provides one.

Compiler SHA-256:
`e5d60db892c8f21cc8c9128af26d8bded0e96d7f92946f5a549397c43a872649`.

From a checkout of the pinned Omega source, `sh seed/omega-from-seed.sh` produces
`seed/out/omc`. From this repository on Linux ARM64:

The native coordinator is `om/build/main.om`, with compilation units in
`om/build.plan`. Its initial executable can be built using the pinned compiler:

```sh
mkdir -p build/om
cat om/core/strings.om om/core/literals.om om/data/sha256.om om/core/files.om om/core/process.om om/build/main.om > build/om/bootstrap.om
/path/to/omc < build/om/bootstrap.om > build/om/om-build
chmod +x build/om/om-build
build/om/om-build /path/to/omc
build/om/om-test /path/to/omc
build/om/bench_verify_records records artifacts tests/fixtures/records/good
build/om/bench_index --check
```

The native coordinator checks the pinned compiler digest, concatenates declared
sources, invokes the compiler directly, verifies static ARM64 executable headers,
and installs outputs atomically. The test coordinator runs native test programs,
frozen hash/oracle checks and compiled negative controls. Neither coordinator
invokes a shell, Rust compiler, Python or JavaScript. Git remains the source
identity and tracked-file inventory interface. Old scripts still present in this
migration branch are pending deletion after final gate cutover.

The build also emits `build/om/om-sha256`: native SHA-256 of standard input,
with a declared 1 GiB input limit. It emits 64 lowercase hexadecimal digits and
a newline, exits 65 for oversized input, and 74 for I/O failure. Input failures
produce no digest. This executable has no helper process or shared library.
The installed checksum utility is used only to qualify padding/read boundaries
outside the delivered implementation. The native build checks compiler identity with its own SHA-256.

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
  this is not a claim that they fit a finite metric. Native record schema checks and bounded decimal arithmetic now enforce metric requirements. Negative, fractional and exponent spellings
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
  The caller must exclude failed requests. Binary64 arithmetic and decimal formatting now support rates, variance and final metrics.
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
caught mutants. All tests are compiled `.om`; `om/test/` now coordinates them natively.

The compiler's same-lineage self-rebuild and regression checks establish the
reported reproducibility results. Independent compiler ancestry correspondence
under [Omega #650](https://github.com/aien-dev/omega-mv/issues/650) is **NOT_RUN**;
matching self-rebuild hashes are not substituted for that separate qualification.
No durable reuse system is claimed by this work; [Omega #651](https://github.com/aien-dev/omega-mv/issues/651)
remains with its existing compiler/runtime owners.

## Remaining migration

| Existing component | Required work |
| --- | --- |
| Prime race runner, validation and receipts (`build/om/prime_race`) | Native `verify` and `run` commands qualified across sweep, tag, child-process, statistics, content-addressed receipt storage, validation and end-to-end execution; `prime-drag-race/src/*.rs`, `prime-drag-race/tests/negative.rs`, `prime-drag-race/Cargo.toml` and `prime-drag-race/Cargo.lock` deleted. |
| `llm-stream` (`build/om/bench_llm_stream`) | Native HTTP/SSE framing, usage-only token accounting, IPv4/IPv6/hosts/DNS transport, process-pool concurrency and record/sample emission qualified. Shared Rust `src/harness/llm_stream.rs` and `sse.rs` await retirement of the Rust `measure` command. |
| `measure` | Native system/memory sampling (`om/measure/system.om`, `om/measure/memory.om`) and binary32 vector workload (`om/measure/vector.om`) qualified; active endpoint sweep and sibling-runtime boundary remain open. |
| `verify`, `report`, `generate`, `sync-docs` (`build/om/bench_dataset`) | Native dataset CLI, schema/budget validation, markdown table sync and SVG rendering qualified (`tests/om/dataset-cli.om`). Gate cutover and deletion of `src/main.rs`, `src/models.rs` and `src/svg_chart.rs` await retirement of the Rust `measure` command. |
| `bench_verify_records`, `bench_index` | Native commands qualified; old Rust implementations and corresponding Rust tests deleted. Shared dependencies await their last other consumer. |
| `bench_branching_neutral`, `bench_canonical_suite` | Replace direct use of the five sibling Rust crates without changing measured work. |
| Build and verification gates | Port scripts and project-owned tests to `.om`; switch qualified consumers, then delete superseded code and dependencies. Existing gates still test Rust. |

Omega now supplies 35 native host operations for files, directories, clocks,
sockets, polling, process launch/wait, pipes and command-line arguments. The
implementation is in its own `seed/omc/omc.om`; the seed chain builds it and
proves byte-identical self-reproduction. Native tests cover success and failure
paths, TCP loopback, 41 negative controls and preservation of 70 original
compiler fixtures. See the pinned compiler's `seed/host/README.md` and
`docs/one_layer/OMEGESE.md` section 10 for exact contracts and limits.

No temporary host adapter was added. Remaining work includes consumer integration and qualification, active measurement commands, and the boundary to the sibling runtime. The compiler still has integer types only; floating-point
types and direct Rust calls are not claimed as implemented.

No historical records, benchmark claims, runtime services or frozen contracts
are changed by this port. The host-extension decision is settled; the remaining
benchmark commands still require implementation, consumer qualification and
deletion of the superseded implementation and dependencies.

## Numeric qualification

`om/data/double.om`, `decimal.om` and `single.om` implement software
binary64/binary32 arithmetic using Omega integer operations. These do not claim
hardware floating-point or SIMD throughput. Arithmetic rounds to nearest with
ties to even and supports subnormals.

Decimal input accepts finite values with up to 1,100 significant digits.
Overflow and capacity violations are refused. JSON output uses 17 significant
digits in scientific notation, preserving the binary64 value on readback;
shortest spelling and historical serialization byte equality are not promised.
Fixed decimal output supports 0 through 18 fractional digits.

Qualification data is frozen in `tests/fixtures/numerics/`: independent V8
binary64/binary32 results and exact BigInt rational decimal rounding. No foreign
implementation or generator is shipped or invoked by native programs.
The native tests cover 1,044 binary64 operations, 316 decimal parse/round-trip
cases, 163 fixed-format cases and 1,024 binary32 operations. Boundary cases
include negative zero, subnormals, halfway rounding and maximum finite values.
These are development tests, not an untouched model evaluation set.
