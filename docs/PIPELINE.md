# Verification and merge policy

## Current process (since 2026-10-24)

The [`AIEN` workflow](../.github/workflows/aien-verification.yml) runs two different jobs:

- **AIEN Checks** runs on pull requests and pushes to `main`. It checks crumbs and, for enrolled repositories, runs the portable smoke gates named at the base ref. It is advisory; it does not fetch or verify a signed Spark bundle.
- **AIEN Alpha** runs on `alpha/*` tag pushes only. It confirms that the tagged commit is on `main` and verifies its signed full Spark bundle from [aien-receipts/benchmarks](https://github.com/aien-dev/aien-receipts/tree/main/benchmarks) against the base-ref trust anchors.

The current GitHub branch rules and classic protection require no status check for merges (checked 2026-10-26). A green advisory PR check is not an Alpha qualification or permission to merge. The Spark worker may still produce bundles, but the PR workflow no longer depends on their publication. A maintainer reviews and authorizes each change; the `ready-to-merge` label is not an instruction to infer from an advisory check.

## Historical process (2026-10-16 to 2026-10-23)

The Spark worker (`aien-test worker --github` in aien-sovereign-core) ran selected gates plus `.aien/always` for each PR head and signed the result. The local pipeline published the bundle to `benchmarks/<sha>/bundle.json` in aien-receipts and reran the former `AIEN Verification` hosted workflow, which downloaded and checked it. Under that earlier policy a `ready-to-merge` label, passing Spark bundle and required hosted check were used to promote a PR. This describes the old receipt bridge, **not** the current PR workflow.
