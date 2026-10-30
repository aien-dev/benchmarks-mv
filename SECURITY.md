# Security policy

## Reporting a vulnerability

Report privately. Do not open a public issue, pull request or discussion about it.

Report vulnerabilities privately through the repository's Security tab, "Report a vulnerability",
at https://github.com/aien-dev/benchmarks-mv/security/advisories/new.

Include the commit or release you tested, the component (path), steps to reproduce, and what an
attacker gains. If the report involves a secret, say where it was exposed and do not send the
secret itself. Do not post proof-of-concept payloads publicly.

## Response targets

This project has one maintainer; the targets below are best effort and stated honestly.

- Acknowledgement: within 3 working days.
- Triage and severity: within 7 days of acknowledgement.
- Fix or mitigation for confirmed issues: aim for 30 days for high and critical, 90 days
  otherwise, with a status update at least every 14 days until closed.
- Disclosure: coordinated. An advisory is published with credit (unless you ask otherwise)
  once the fix is on the default branch. Default embargo 90 days, negotiable.

## Scope

This repository holds AIEN's performance measurements and the Rust tools that take and check
them: the Prime Drag Race suite runner in `prime-drag-race/` (a standalone crate), and the
`aien-benchmarks` measurement harness at the root with four helper programs.

In scope:

- A record or artifact bundle that passes `bench_verify_records` while it carries placeholder
  values, short commit ids, missing evidence files or percentiles from too few samples.
- Tampering with committed evidence (`artifacts/*/SHA256SUMS`, `prime-drag-race/evidence/`)
  that the checks do not detect.
- Telemetry leaks: the harness reads system details and live-service memory; report a path that
  publishes more than the record format names.
- Secrets in benchmark configurations or fixtures. This repository's rule: API keys, hardware
  tokens and credentials must never reside on disk in plaintext; secrets and tokens must never be
  written to disk in benchmark configurations or test fixtures.
- The harness's standalone HTTP server (bound to `127.0.0.1` on an ephemeral port) or the
  streaming client mishandling hostile server replies.

Out of scope:

- The implementations under test in other repositories, the model and embedding services the
  harness measures, the sibling runtime crates, and the host operating system.

## Supported versions

Only the default branch `main`; there is no release (both crates are version `0.1.0`). Reports against older commits are welcome but fixes land on the default branch only.

## Status

README (status 2026-10-19): one evidence-backed result, the Prime Drag Race campaign of
5 October 2026; all older figures are withdrawn. No certification, no bug bounty. See AUDIT_SCOPE.md for what to review and how to build and test it.
