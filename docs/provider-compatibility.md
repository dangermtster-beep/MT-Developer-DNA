# V0.8 Provider Uyumluluğu

Bu tablo dosya üretim sözleşmesini gösterir. `Destekleniyor`, canlı kurulum veya
interaktif kabul testinin bu makinede geçtiği anlamına gelmez; Doctor bunları
ayrı `PASS`, `NOT_RUN` ve `UNSUPPORTED` durumlarıyla raporlar.

| Provider | Project skill | Global skill | Çağrı | Durum |
|---|---|---|---|---|
| Codex | `.agents/skills/mt-developer-dna/SKILL.md` | aynı | `$mt-developer-dna` | Skill destekli, slash yok |
| Claude Code | `.claude/skills/mt-developer-dna/SKILL.md` | aynı | `/mt-developer-dna` | Destekleniyor |
| Gemini CLI | `.gemini/skills/mt-developer-dna/SKILL.md` | aynı | `/mt-developer-dna` | Komut dosyasıyla; canlı doğrulama gerekli |
| Google Antigravity | `.agents/skills/mt-developer-dna/SKILL.md` | IDE: `.gemini/config/skills/mt-developer-dna/SKILL.md`; CLI: `.gemini/antigravity-cli/skills/mt-developer-dna/SKILL.md` | `/mt-developer-dna` | `agy` CLI veya doğrulanmış IDE kurulumu algılanır; eski workflow üretilmez |
| Cursor | `.cursor/skills/mt-developer-dna/SKILL.md` | aynı | `/mt-developer-dna` | Destekleniyor |
| Cline | `.cline/skills/mt-developer-dna/SKILL.md` | `%USERPROFILE%\.cline\skills\mt-developer-dna\SKILL.md` | `/mt-developer-dna` | `saoudrizwan.claude-dev` VS Code extension manifestiyle doğrulanır; aktif profil durumu canlı IDE doğrulaması gerektirir |
| OpenCode | `.opencode/skills/mt-developer-dna/SKILL.md` | `.config/opencode/skills/mt-developer-dna/SKILL.md` | `/mt-developer-dna` | Windows'ta `opencode.exe`, npm `opencode.cmd` ve `opencode.ps1`; `opencode/slash` metadata ile |

Antigravity `.agent/workflows/mt-developer-dna.md` çıktısı bilinçli olarak yoktur.
Resmî workflow belgeleri mevcut sistemin 19 Ekim 2026'da çalışmayı durduracağını
belirtmektedir; kalıcı V0.8 adapter'ı Agent Skill mekanizmasını kullanır.

## Resmî kanıt kaynakları

- Codex: https://developers.openai.com/blog/skills-agents-sdk
- Claude Code: https://code.claude.com/docs/en/skills
- Gemini CLI: https://geminicli.com/docs/cli/using-agent-skills/
- Antigravity skills: https://antigravity.google/docs/skills?app=antigravity-ide
- Antigravity workflows: https://www.antigravity.google/docs/ide/workflows/
- Cursor: https://cursor.com/docs/skills
- Cline: https://github.com/cline/cline/blob/main/docs/customization/skills.mdx
- OpenCode: https://docs.opencode.ai/docs/skills/

Belgeler 17 Ekim 2026 tarihinde kontrol edildi. Yerel executable probe ve canlı
provider kabul testi ayrı kanıt sınıflarıdır.

Windows discovery sırasıyla PATH üzerindeki `agy`,
`%USERPROFILE%\AppData\Local\agy\bin\agy.exe` ve
`%USERPROFILE%\AppData\Local\Programs\Antigravity IDE\Antigravity IDE.exe`
yollarını doğrular. Farklı bir kurulum için yalnız tek provider seçiliyken
`--provider-path <executable-veya-klasör>` kullanılabilir.

Cline discovery, klasör adına tek başına güvenmez. Standart
`%USERPROFILE%\.vscode\extensions`, VS Code Insiders
`%USERPROFILE%\.vscode-insiders\extensions`, `VSCODE_EXTENSIONS` özel dizini
ve kullanıcı tarafından seçilen `--provider-path` köklerinde `package.json`
manifestinin `saoudrizwan.claude-dev` kimliğini doğrular. VS Code profilleri aynı
fiziksel paket deposunu kullanabilse de extension'ın seçili profilde etkin olması
yalnız paket varlığından çıkarılmaz.

OpenCode Windows discovery PATH üzerindeki `opencode.exe`, `opencode.cmd` ve
`opencode.ps1` dosyalarını ayrı ayrı doğrular. Native executable doğrudan,
`.cmd` yalnız doğrulanmış dosya adı ve shell metakarakter kontrolünden sonra
`cmd.exe /D /S /C` ile çalıştırılır. `.ps1`, execution policy değiştirilmeden
`powershell.exe -NoProfile -NonInteractive -File` ile denenir. Policy engellerse
kurulum yine algılanır, fakat bu shim'in sürümü doğrulanmış sayılmaz.

