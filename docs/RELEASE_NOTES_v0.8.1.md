# MT Developer DNA v0.8.1

Backward-compatible fix release by MT SOFTWARE.

## Fixes

- Skill description is language-independent. With the `ask_user` profile the
  placeholder "Kullanıcı seçimi bekleniyor" no longer leaks into the English
  description used for skill discovery.
- Global installs neutralize the profile (language `ask_user`, UI and database
  `Auto`), so a language-specific profile no longer locks every project to
  that language. The CLI prints a note when this happens.
- Codex installed through npm (`codex.cmd` / `codex.ps1`) is detected, and
  `--provider-path` accepts those shims.
- `deploy sync --apply --yes` reports the real per-provider outcome instead of
  a fixed "no files changed" note.

## Improvements

- `references/` ships with every skill (project and global scope, including
  the Antigravity CLI override path); the generated `SKILL.md` links each file.
- The skill reads the selected language from `.mt-dna/project.json` first and
  asks only when nothing is recorded.
- Single skill source: the drifting hand-written `mt-developer-dna/SKILL.md`
  was removed; `mt-developer-dna/references/` is the only source folder.
- `dna-deploy` and `dna-cli` were split into modules without behavior change.
- Version-specific history documents moved to `docs/history/`.

## Upgrade

```powershell
cargo install --path crates/dna-cli --locked
dna-cli deploy sync --profile .\profiles\universal.json --provider all --scope global
dna-cli deploy sync --profile .\profiles\universal.json --provider all --scope global --apply --yes
```

The first command is a dry-run. Files you changed by hand are reported as
`conflict` and are not overwritten.

## Validation

See `reports/v0.8.1-validation.md`.
