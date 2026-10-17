# Provider uyumluluğu

Bu matris v0.8.1 renderer ve deployment sözleşmesini gösterir. “Destekleniyor”
ifadesi canlı model oturumunun bu makinede kabul testinden geçtiği anlamına gelmez.

| Provider | Project skill | Global skill | Çağrı | Discovery / sınır |
|---|---|---|---|---|
| Codex | `.agents/skills/mt-developer-dna/SKILL.md` | `~/.agents/skills/...` | `$mt-developer-dna` | `codex`; slash destekleniyor denmez |
| Claude Code | `.claude/skills/mt-developer-dna/SKILL.md` | `~/.claude/skills/...` | `/mt-developer-dna` | `claude` CLI |
| Gemini CLI | `.gemini/skills/.../SKILL.md` + command TOML | `~/.gemini/skills/...` + command TOML | `/mt-developer-dna` | `gemini`; `/skills list` ve `/commands list` ile doğrula |
| Google Antigravity | `.agents/skills/.../SKILL.md` | IDE `~/.gemini/config/skills/...`; CLI `~/.gemini/antigravity-cli/skills/...` | `/mt-developer-dna` | `agy`, doğrulanmış IDE veya `--provider-path`; eski workflow yok |
| Cursor | `.cursor/skills/.../SKILL.md` | `~/.cursor/skills/...` | `/mt-developer-dna` | `cursor`; canlı IDE testi ayrıca gerekir |
| Cline | `.cline/skills/.../SKILL.md` | `~/.cline/skills/...` | `/mt-developer-dna` | VS Code extension `saoudrizwan.claude-dev`; profil enabled durumu ayrıca gerekir |
| OpenCode | `.opencode/skills/.../SKILL.md` | `~/.config/opencode/skills/...` | `/mt-developer-dna` | `.exe`, `.cmd`, `.ps1`; slash metadata ile |

## Kanıt seviyeleri

- **Discovery:** executable veya doğrulanmış extension/uygulama manifesti bulundu.
- **Render:** güvenli göreli yollarla beklenen paket üretildi.
- **Deployment:** dry-run/apply/registry/hash davranışı doğrulandı.
- **Live acceptance:** skill gerçek provider oturumunda çağrıldı.

İlk üç seviye dördüncü seviye gibi raporlanmaz. `doctor` bu ayrımı `PASS`, `WARN`,
`FAIL`, `NOT_RUN` ve `UNSUPPORTED` durumlarıyla korur.

## Güvenli deployment sözleşmesi

`deploy plan` salt-okunurdur. `deploy install` için `--yes`; uygulanan sync için
`--apply --yes` gerekir. Global ve project scope karıştırılmaz. Kayıtsız veya
hash'i değişmiş hedef `conflict` olur. Provider ayarları, anahtarları ve token'ları
yedeğe kopyalanmaz.

## Resmi kaynaklar

- Codex: https://developers.openai.com/codex/skills
- Claude Code: https://code.claude.com/docs/en/skills
- Gemini CLI: https://geminicli.com/docs/cli/using-agent-skills/
- Google Antigravity: https://www.antigravity.google/docs/migration/workflows-to-skills/
- Cursor: https://cursor.com/docs/skills
- Cline: https://github.com/cline/cline/blob/main/docs/customization/skills.mdx
- OpenCode: https://docs.opencode.ai/docs/skills/

Yollar ve çağrılar 17 Ekim 2026 tarihinde resmi kaynaklar ve mevcut renderer ile
karşılaştırılmıştır. Provider sürümleri değişebileceğinden her release'te yeniden
doğrulanmalıdır.
