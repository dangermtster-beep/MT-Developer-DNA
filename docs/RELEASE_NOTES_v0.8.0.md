# MT Developer DNA v0.8.0

First public release by MT SOFTWARE.

## Highlights

- Universal Language Profile with explicit user language selection.
- PLAN, BUILD, and REVIEW workflow contracts.
- Provider-independent Project Memory and handoff.
- On-demand Quality Gates, security checks, and bounded performance measurement.
- Controlled Auto-Fix preview/apply with transaction-scoped rollback.
- Dry-run-first provider deployment, synchronization, backup, Doctor, and rollback.
- Adapters for Codex, Claude Code, Gemini CLI, Google Antigravity, Cursor, Cline, and OpenCode.
- Windows npm shim discovery for OpenCode and VS Code extension discovery for Cline.

## Windows asset

`mt-developer-dna-v0.8.0-windows-x86_64.zip` contains `dna-cli.exe`, the Universal
Profile, canonical skill files, installation documentation, release notes, and
the MIT license.

## Safety notes

- Installation and synchronization remain dry-run until explicit approval flags are supplied.
- Existing provider files are not silently overwritten.
- Live provider acceptance is distinct from discovery and structural package validation.
- GitHub update checks and downloads are manual in v0.8.0.

## Validation

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo test --workspace`: PASS — 95 passed, 0 failed
- `cargo build --workspace --release`: PASS
- Universal Profile validation and seven-provider render tests: PASS
- Extracted Windows package smoke test: PASS
- Bounded local secret scan: PASS; clean results are not a security guarantee
- Dependency advisory scan: NOT_RUN because `cargo-audit` was not installed
- Live interactive acceptance was not repeated for every provider

See `PUBLIC_RELEASE_VALIDATION.md` in the package for the full evidence boundary.
