# Contributing to MT Developer DNA

Thank you for contributing. MT Developer DNA is developed by MT SOFTWARE and
released under the MIT License.

## Development setup

1. Fork and clone `https://github.com/dangermtster-beep/MT-Developer-DNA`.
2. Install Rust `1.81` or later.
3. Create a focused branch and preserve existing crate/API boundaries.
4. Do not commit credentials, `.env`, private `.mt-dna` memories, generated
   provider installations, `target/`, or personal absolute paths.

Before opening a pull request, run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

## Design expectations

- Keep PLAN and REVIEW read-only by default.
- Require explicit approval for writes, sync apply, Auto-Fix, and rollback.
- Never silently overwrite provider or user-modified files.
- Keep user language selection stable across provider changes.
- Prefer bounded, short-lived work over daemons, watchers, or polling.
- Report missing tools as `NOT_RUN` and unsupported metrics as `UNAVAILABLE`.
- Add tests for new profile, renderer, memory, quality, discovery, and deployment behavior.
- Update English and Turkish user documentation when CLI behavior changes.

Pull requests should explain the problem, scope, security/performance impact,
tests run, and any live provider checks that remain unverified.
