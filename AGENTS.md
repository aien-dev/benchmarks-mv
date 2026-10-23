# AGENTS.md

Operating instructions for autonomous AI agents collaborating on the AIEN Benchmarks repository.

## Operational Directives

1. **Hardware Grounding**: All benchmark metrics must represent real measurements recorded on dedicated hardware. Synthetic, simulated, or fabricated benchmarks are strictly prohibited.
2. **Native Systems Priority**: Rewrite all first-party implementation, build/release tooling, automation and project-owned tests in compiled native Omega (`.om`), as requested by Drake on 2026-10-23 and governed by aien-architecture-mv#300. Qualify each replacement, switch its consumers, then delete its superseded sources and obsolete dependencies from the active tree. Consult historical implementations through Git history; retain no legacy directories, wrappers, fallback paths or dual production paths as a final state. Existing non-Omega code during migration is unfinished work, not an exemption. Preserve immutable evidence and Git history. Missing language features are implementation dependencies. Drake explicitly authorized subagents for this rewrite. Keep delivered implementation in native Omega; foreign implementation helpers remain prohibited.
3. **Unslop Standard**: Zero em dashes and zero en dashes across documentation, code comments, and commit messages. Use plain hyphens or standard punctuation.
4. **Zero Disk Secrets**: Secrets and tokens must never be written to disk in benchmark configurations or test fixtures.
5. **Linear Genesis History**: The main branch maintains a clean linear commit history.
