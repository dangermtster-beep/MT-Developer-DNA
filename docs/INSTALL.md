# Kurulum

Bu belge MT Developer DNA v0.8.1 içindir. Developed by MT SOFTWARE.

## Kaynaktan derleme

Gereksinimler: Git, Rust `1.81+` ve Cargo. Windows release paketi
`x86_64-pc-windows-msvc` hedefiyle hazırlanır.

```powershell
git clone https://github.com/dangermtster-beep/MT-Developer-DNA.git
cd MT-Developer-DNA
cargo build --workspace --release
.\target\release\dna-cli.exe --version
```

Hazır ZIP kullanılıyorsa arşivi yazılabilir bir klasöre çıkarın ve komutları paket
kökünde çalıştırın. Kurulumdan önce daima salt-okunur plan üretin:

```powershell
.\dna-cli.exe providers list
.\dna-cli.exe deploy plan --profile .\profiles\universal.json `
  --provider PROVIDER --scope project --path C:\Projects\Example
```

Plan beklendiği gibiyse aynı parametrelerle `deploy install ... --yes` çalıştırın.
Var olan, registry dışı veya kullanıcı tarafından değiştirilmiş dosya varsayılan
olarak `conflict` olur ve üzerine yazılmaz.

## Codex

- Provider kurulumu: resmi Codex kurulumu tamamlanmalı ve `codex --version` çalışmalıdır. npm ile kurulan `codex.cmd`/`codex.ps1` shim'leri PATH üzerinde otomatik algılanır; gerekirse `--provider-path` ile açıkça seçilebilir.
- Her kurulumda skill klasörünün yanına `references/` da yazılır. Global kurulum profildeki dil/arayüz/veritabanı seçimini skill'e gömmez.
- Proje hedefi: `.agents/skills/mt-developer-dna/SKILL.md`.
- Kullanıcı hedefi: `~/.agents/skills/mt-developer-dna/SKILL.md`.
- Kurulum: `--provider codex --scope project` veya `--scope global`.
- Etkinleştirme: Codex içinde `$mt-developer-dna` yazın. `/mt-developer-dna` desteği varmış gibi kabul etmeyin.
- Doğrulama: `dna-cli doctor --provider codex ...`; ardından gerçek Codex oturumunda skill'i çağırın.

Resmi skill belgesi: https://developers.openai.com/codex/skills

## Claude Code

- Provider kurulumu: Claude Code kurulumundan sonra `claude --version` çalışmalıdır.
- Proje hedefi: `.claude/skills/mt-developer-dna/SKILL.md`.
- Kullanıcı hedefi: `~/.claude/skills/mt-developer-dna/SKILL.md`.
- Kurulum: `--provider claude --scope project|global`.
- Etkinleştirme: Claude Code oturumunda `/mt-developer-dna`.
- Doğrulama: slash menüsünde skill'i görün ve gerçek oturumda çağırın.

Resmi skill belgesi: https://code.claude.com/docs/en/skills

## Gemini CLI

- Provider kurulumu: Gemini CLI kurulumundan sonra `gemini --version` çalışmalıdır.
- Proje skill hedefi: `.gemini/skills/mt-developer-dna/SKILL.md`.
- Kullanıcı skill hedefi: `~/.gemini/skills/mt-developer-dna/SKILL.md`.
- Slash komut hedefi: `.gemini/commands/mt-developer-dna.toml` (scope'a göre proje veya kullanıcı kökü).
- Kurulum: `--provider gemini --scope project|global`.
- Etkinleştirme: `/skills reload`, `/commands reload`, ardından `/mt-developer-dna`.
- Doğrulama: `/skills list` ve `/commands list`; sonrasında gerçek çağrı.

Resmi belgeler: https://geminicli.com/docs/cli/using-agent-skills/ ve
https://geminicli.com/docs/cli/custom-commands/

## Google Antigravity

- Discovery: PATH üzerinde `agy`, Windows kullanıcı kurulumundaki `agy.exe` veya doğrulanmış Antigravity IDE executable'ı.
- Proje hedefi: `.agents/skills/mt-developer-dna/SKILL.md`.
- IDE kullanıcı hedefi: `~/.gemini/config/skills/mt-developer-dna/SKILL.md`.
- Antigravity CLI kullanıcı hedefi: `~/.gemini/antigravity-cli/skills/mt-developer-dna/SKILL.md`.
- Standart dışı kurulum: tek provider seçerek `--provider-path <exe-veya-klasör>` verin.
- Etkinleştirme: `/mt-developer-dna`.
- Doğrulama: `doctor` sonrası gerçek IDE/CLI oturumunda çağırın.

Eski `.agent/workflows/` çıktısı üretilmez. Resmi geçiş belgesi:
https://www.antigravity.google/docs/migration/workflows-to-skills/

## Cursor

- Provider kurulumu: Cursor kurulumundan sonra PATH'te `cursor` bulunması discovery için kullanılır.
- Proje hedefi: `.cursor/skills/mt-developer-dna/SKILL.md`.
- Kullanıcı hedefi: `~/.cursor/skills/mt-developer-dna/SKILL.md`.
- Kurulum: `--provider cursor --scope project|global`.
- Etkinleştirme: Agent sohbetinde `/mt-developer-dna` veya skill seçimi.
- Doğrulama: Cursor Skills görünümünde skill'i ve gerçek çağrıyı kontrol edin.

Resmi belge: https://cursor.com/docs/skills

## Cline

Cline bağımsız CLI varsayılmaz. Discovery, VS Code/VS Code Insiders extension
depolarındaki `package.json` içinde `saoudrizwan.claude-dev` kimliğini doğrular;
`VSCODE_EXTENSIONS` ve `--provider-path` ile özel kök desteklenir.

- Proje hedefi: `.cline/skills/mt-developer-dna/SKILL.md`.
- Windows kullanıcı hedefi: `%USERPROFILE%\.cline\skills\mt-developer-dna\SKILL.md`.
- Kurulum: `--provider cline --scope project|global`.
- Etkinleştirme: Cline Skills sekmesinde enabled olduğunu doğrulayın, sonra `/mt-developer-dna`.
- Not: Paketin diskte bulunması, seçili VS Code profilinde extension'ın etkin olduğunu kanıtlamaz.

Resmi belge: https://github.com/cline/cline/blob/main/docs/customization/skills.mdx

## OpenCode

- Discovery: PATH dizinlerinde `opencode.exe`, npm `opencode.cmd` ve `opencode.ps1`.
- Proje hedefi: `.opencode/skills/mt-developer-dna/SKILL.md`.
- Kullanıcı hedefi: `~/.config/opencode/skills/mt-developer-dna/SKILL.md`.
- Kurulum: `--provider opencode --scope project|global`.
- Etkinleştirme: renderer'ın `metadata.opencode/slash: "true"` alanıyla `/mt-developer-dna`.
- Doğrulama: `opencode --version`, `doctor`, sonra gerçek TUI çağrısı.

PowerShell execution policy değiştirilmez. `.ps1` probe'u engellenirse güvenli
`.cmd` veya `.exe` runner'ı kullanılabilir. Resmi belge:
https://docs.opencode.ai/docs/skills/

## Genel doğrulama

```powershell
.\dna-cli.exe doctor --provider all --scope project --path C:\Projects\Example
```

Doctor'ın yapısal `PASS` sonucu canlı model kabul testi değildir. Çalıştırılmayan
interaktif doğrulamalar `NOT_RUN` olarak kalmalıdır.
