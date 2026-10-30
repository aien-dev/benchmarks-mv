# AGENTS.md

Operating instructions for autonomous AI agents collaborating on the AIEN Benchmarks repository.

## Operational Directives

1. **Hardware Grounding**: Hardware performance metrics must represent real measurements recorded on dedicated hardware. Synthetic correctness fixtures and MODELED statistical planning are allowed only when explicitly separated from physical performance evidence; neither qualifies hardware.
2. **Native Systems Priority**: New first-party evaluation components and test runners use seeded handwritten Omega `.om`, as required by #38, #43 and #55. Existing compiled native Rust tooling remains unchanged until a qualified replacement exists. Do not introduce Python, Node or other interpreter wrappers in the product path; independent Wolfram and read-only verification oracles remain external evidence tools.
3. **Unslop Standard**: Zero em dashes and zero en dashes across documentation, code comments, and commit messages. Use plain hyphens or standard punctuation.
4. **Zero Disk Secrets**: Secrets and tokens must never be written to disk in benchmark configurations or test fixtures.
5. **Linear Genesis History**: The main branch maintains a clean linear commit history.
