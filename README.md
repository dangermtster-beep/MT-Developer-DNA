# MT Developer DNA

[![Version](https://img.shields.io/badge/version-v0.8.0-blue.svg)](https://github.com/dangermtster-beep/MT-Developer-DNA/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.81%2B-orange.svg)](https://www.rust-lang.org/)

**MT Developer DNA** is a lightweight Rust CLI and portable Agent Skill that
applies consistent software-development standards across supported AI coding
environments. It provides deterministic provider packages, project memory,
quality gates, controlled optimization, and approval-gated deployment without
running a desktop application, daemon, watcher, or background polling loop.

Developed by **MT SOFTWARE**. Turkish documentation is available in
[README_TR.md](README_TR.md).

## Verified capabilities

- User-selected language profiles for Rust, C#, Python, C++, Go, Java, and TypeScript.
- Universal Language Profile that asks for a language instead of silently choosing one.
- PLAN, BUILD, and REVIEW contracts with read-only defaults and explicit approval boundaries.
- Provider-independent `.mt-dna` Project Memory with hashing, redaction, bounded scanning, handoff, and repair planning.
- On-demand test, format, lint, build, security, and performance checks with explicit `NOT_RUN` and `UNAVAILABLE` states.
- Controlled low-risk Auto-Fix preview/apply and transaction-scoped rollback.
- Dry-run-first provider deployment, synchronization, version discovery, backup, conflict detection, Doctor, and rollback.
- Package rendering for Codex, Claude Code, Gemini CLI, Google Antigravity, Cursor, Cline, and OpenCode.

Provider discovery or structurally valid output does not prove that a live model
session accepted the skill. See [provider compatibility](docs/PROVIDERS.md) for
the separate validation states.

## Workspace

| Crate | Responsibility |
|---|---|
| `dna-core` | Profiles, validation, modes, plans, reviews, and optimization contracts |
| `dna-render` | Deterministic provider-specific package rendering |
| `dna-memory` | Safe `.mt-dna` storage, redaction, scanning, handoff, and integrity checks |
| `dna-quality` | Bounded process execution, quality gates, security checks, and measurements |
| `dna-deploy` | Discovery, manifests, registry, dry-run, install, sync, backup, Doctor, and rollback |
| `dna-cli` | Turkish command-line interface that coordinates the workspace |

## Requirements and build

- Rust `1.81` or later with Cargo.
- Windows release package target: `x86_64-pc-windows-msvc`.
- A supported provider is required only for provider installation or live use.

```powershell
git clone https://github.com/dangermtster-beep/MT-Developer-DNA.git
cd MT-Developer-DNA
cargo build --workspace --release
cargo test --workspace
```

Detailed setup: [docs/INSTALL.md](docs/INSTALL.md).

## Quick start

All deployment plans are read-only. Installation requires the explicit `--yes`
flag and never silently overwrites a conflict.

```powershell
# Validate the language-neutral starting profile
.\target\release\dna-cli.exe profiles validate --profile .\profiles\universal.json

# Discover installed providers
.\target\release\dna-cli.exe providers list

# Preview a project installation without writing files
.\target\release\dna-cli.exe deploy plan `
  --profile .\profiles\universal.json `
  --provider codex --scope project --path C:\Projects\Example

# Apply only after reviewing the plan
.\target\release\dna-cli.exe deploy install `
  --profile .\profiles\universal.json `
  --provider codex --scope project --path C:\Projects\Example --yes
```

Invocation after installation depends on the provider: Codex uses
`$mt-developer-dna`; the verified slash-capable adapters expose
`/mt-developer-dna`. Run `dna-cli doctor` and follow the provider-specific live
verification steps before treating an integration as accepted.

## Universal Language Profile

`profiles/universal.json` stores `language: "ask_user"`. The selected language
must be supplied by the user and is not changed by deployment or provider
synchronization. Language-specific profiles are under
`mt-developer-dna/references/languages/`.

## Modes and memory

- **PLAN** performs bounded analysis and creates an implementation plan without source writes by default.
- **BUILD** produces an approval contract; the provider agent performs real edits through its own permission model.
- **REVIEW** reports evidence, severity, location, impact, and remediation while remaining read-only by default.

Project Memory lives only under the target project's `.mt-dna/` directory.
Write operations first produce a preview and require confirmation. Provider chat
history is never presented as shared memory.

## Quality, performance, Auto-Fix, and rollback

Quality checks run only on request and use bounded output and timeouts. A missing
required tool is `NOT_RUN`; an unsupported metric is `UNAVAILABLE`; neither is
reported as a pass. Performance claims require comparable measurements.

Auto-Fix is limited to previewed, low-risk deterministic changes with pre/post
quality gates and unchanged target hashes. Rollback touches only files recorded
by the matching transaction and stops when later user changes are detected.

## Synchronization and updates

`deploy sync` defaults to dry-run. Applying it requires both `--apply` and
`--yes`. Registry-external or hash-changed files are conflicts and are not
silently merged or overwritten. MT Developer DNA does not automatically download
or install GitHub releases; see [docs/UPDATE.md](docs/UPDATE.md).

## Documentation

- [Installation](docs/INSTALL.md)
- [Usage](docs/USAGE.md)
- [Providers](docs/PROVIDERS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Performance](docs/PERFORMANCE.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Update strategy](docs/UPDATE.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

## License

MIT License. See [LICENSE](LICENSE).

Developed by **MT SOFTWARE**.
