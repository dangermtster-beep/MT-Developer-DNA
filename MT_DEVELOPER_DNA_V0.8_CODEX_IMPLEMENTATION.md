# MT Developer DNA V0.8 — Universal Language Profile & Provider Expansion

**Status:** Implementation specification (not implemented)  
**Primary interface:** AI coding-agent skill, invoked as `/mt-developer-dna` where supported; use provider-native invocation otherwise.  
**Language of user interaction:** Turkish.  
**Out of scope:** Desktop GUI, unrelated research/reporting skills, publishing to GitHub in this release.

## 1. Objective

Extend the existing V0.1–V0.7 Rust workspace without breaking its APIs, memory files, quality gates, deployment registry, or rollback. Replace the Rust-only test profile used during initial deployment with a **language-neutral universal profile**. Add adapters for **Google Antigravity**, **Cursor**, **Cline**, and **OpenCode**; retain Codex, Claude Code, and Gemini CLI. Do not assume all providers support identical slash commands or skill layouts.

## 2. Mandatory preflight

1. Inspect `Cargo.toml`, all `crates/*`, existing profile schema and fixture, deploy adapter implementations, generated `SKILL.md`, project-memory format, tests, and `ARCHITECTURE.md`.
2. Run `cargo test --workspace` and `cargo build --workspace --release`; record baseline, including failures.
3. Determine actual CLI command names using `dna-cli --help` and subcommand `--help`; do not invent flags.
4. Check current provider documentation or locally installed version for skill/rules layout, global/project scope and invocation syntax. Record evidence in `docs/provider-compatibility.md`. Unsupported or uncertain features must be marked `unknown` or `unsupported`.
5. Do not modify existing global skill files during preflight.

## 3. Universal Language Profile

Add a real profile file (e.g. `profiles/universal.json`) matching the existing schema; evolve the schema with backward compatibility only when necessary. Do not use `tests/fixtures/profile-rust.json` as the global default.

**Language selection policy:**
- Supported choices initially: Rust, C#, Python, C++, Go, Java, TypeScript; allow extensible additions.
- `language: null` / `ask_user` (use the actual schema-compatible representation) means no fixed language.
- If the user specifies a language, obey it. If absent, ask once when material; optionally recommend alternatives with trade-offs. Never silently replace the user's choice.
- UI, database, target OS and frameworks are also optional and must not be inherited from the Rust fixture.
- Existing-project detection may suggest the repository's primary language, but must not overwrite explicit user instructions.
- Global quality standards: modularity without overengineering; secure secrets handling; tests; documentation; low idle CPU/RAM and no unnecessary GPU redraw. Performance claims require measurements.
- Preserve project-specific overrides and V0.3 memory decisions; define precedence: explicit current request > project configuration/decisions > universal defaults > provider fallback.
- Keep language-specific standards in separate files or registry entries; validate each against the chosen language.

**Example interaction:** `/mt-developer-dna plan` -> asks what to build and preferred language only if missing; selecting C# does not insert Slint/Cargo/SQLite defaults.

## 4. Provider Adapter Registry

Use a data-driven adapter interface rather than branching on provider names throughout the code. Each adapter describes:
- Stable ID, display name, version/capability detection, invocation mechanism and limitations.
- Project/global paths if officially supported; path resolution must account for Windows user home and environment variables.
- Skill/rules/command generation, validation, install, update, status, sync, backup and rollback capabilities.
- Explicit capabilities (`skill`, `rules`, `slash_command`, `global_install`, `project_install`) with supported/unsupported/unknown states.
- Safe conflict policy: dry-run first, no overwrite by default, backup before authorized modifications, atomic file writes, reversible registry changes.

**Initial adapters:** `codex`, `claude`, `gemini`, `antigravity`, `cursor`, `cline`, `opencode`. Do not claim provider support until the actual generated artifacts and invocation are verified. In particular, verify Google Antigravity's installed version and supported rules/workflows/skills before choosing a target directory; do not guess one.

Provider outputs must be derived from the **same canonical standards and language profile**, not independently maintained copies.

## 5. CLI / skill interaction

Preserve existing commands. Extend only where supported by the current parser; document actual syntax from `--help`.

Target conceptual actions:
- `mt-dna profiles list|validate|show` (adapt naming to existing CLI)
- `mt-dna providers list|doctor`
- `mt-dna deploy plan --provider <id> --scope <scope> --profile <real-file-path>`
- `mt-dna deploy install ...` (requires explicit confirmation)
- `mt-dna deploy status`, `sync`, `rollback`

The above are **desired operations, not guaranteed existing CLI flags**. Keep native provider invocation syntax separate from CLI syntax. Where `/mt-developer-dna` cannot be registered natively, document the supported equivalent and avoid false success reports.

## 6. Installation and migration

1. Detect existing global Codex installation generated from `profile-rust.json`.
2. Present a migration plan showing exact files, hashes, before/after differences and profile change.
3. Do not replace the installed Rust profile until user approval; retain a backup and a rollback path.
4. Migrate to `profiles/universal.json` only after validation and a dry-run. Preserve unrelated user edits.
5. Run status/doctor after install and test a real skill invocation in the provider application.
6. Project scope should be tested separately from global scope.

## 7. Testing and acceptance criteria

**Unit tests:** universal profile validation; empty language; each explicit language; precedence; project-memory merge; invalid profiles; unsupported provider capability; path traversal rejection; escaping and path spaces; idempotent generation.

**Integration tests:** generate each provider package to a temporary directory; snapshot/parse generated content; dry-run no filesystem mutations; conflict abort; backup/rollback; repeat install without unintended changes; detect missing provider; migration from Rust fixture.

**Manual acceptance matrix:**
- Codex: plugin recognized, C# selected, no Rust-only defaults, PLAN creates no code.
- Claude Code: native skill invocation verified and same canonical rules loaded.
- Gemini CLI: invocation and paths verified for installed version.
- Antigravity: installed-version-specific supported mechanism verified; if unavailable, mark not tested rather than pass.
- Cursor, Cline, OpenCode: same approach, record capability and invocation evidence.
- BUILD/REVIEW/quality commands continue to work; no regressions in V0.3–V0.7.
- CPU/RAM behavior: no persistent background watcher unless requested; no idle polling introduced.

Acceptance states must be `PASS`, `FAIL`, `NOT_RUN`, or `UNSUPPORTED`, with reasons. Generated file existence alone does not prove native invocation works.

## 8. Documentation and deliverables

- `profiles/universal.json` and language profiles/registry changes.
- Adapter implementations and provider-specific generated artifacts.
- `docs/provider-compatibility.md` with evidence, paths, scope, invocation and limitations.
- `docs/universal-profile-migration.md` with dry-run, backup and rollback steps.
- Updated README, CLI help, changelog and tests.
- `reports/v0.8-validation.md` listing commands executed, real results and any unverified providers.

## 9. Implementation sequence for Codex

**Phase A:** Inspect repo, document existing schema and actual CLI; run baseline tests.  
**Phase B:** Implement language-neutral profile and precedence, with tests.  
**Phase C:** Refactor/add provider adapter registry while retaining current providers.  
**Phase D:** Implement Antigravity, Cursor, Cline, OpenCode adapters using verified layouts.  
**Phase E:** Safe migration plan from Rust-only global profile; no installation without approval.  
**Phase F:** Run all tests, release build, provider validation and report.

After each phase, show changed files and test status. If documentation or installed-provider capabilities cannot be verified, stop that adapter at a safe documented stub rather than inventing a working integration.

## 10. Copy/paste instruction to Codex

> Implement MT Developer DNA V0.8 according to this document. Start by inspecting the existing V0.7 code and running baseline tests. Preserve Rust core, project memory, PLAN/BUILD/REVIEW, quality gates, optimization, and deployment safety. First implement a language-neutral universal profile, then add verified adapters for Antigravity, Cursor, Cline, and OpenCode. Keep Codex/Claude/Gemini working. Do not build a desktop UI or publish to GitHub. Do not modify global provider installations without showing a dry-run plan and obtaining my approval. Report actual test results and unsupported features without guessing.
