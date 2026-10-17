# Kullanım

Komutlar Türkçe çıktı üretir. Tüm örnekler release paketi kökünde PowerShell için
yazılmıştır; kaynak checkout'unda `.\dna-cli.exe` yerine
`.\target\release\dna-cli.exe` kullanılabilir. Tüm skill ve CLI komutlarının
toplu listesi [COMMANDS.md](COMMANDS.md) dosyasındadır.

## Profil ve provider keşfi

```powershell
.\dna-cli.exe profiles list
.\dna-cli.exe profiles validate --profile .\profiles\universal.json
.\dna-cli.exe providers list --json
.\dna-cli.exe doctor --provider all --scope project --path C:\Projects\Example
```

Universal Profile `ask_user` durumundadır; dil seçimi provider kurulumu sırasında
değişmez. Dil seçildikten sonra ilgili profil kullanılmalıdır.

## Kurulum ve senkronizasyon

```powershell
# Dry-run
.\dna-cli.exe deploy plan --profile .\profiles\universal.json `
  --provider opencode --scope project --path C:\Projects\Example

# Açık onaylı apply
.\dna-cli.exe deploy install --profile .\profiles\universal.json `
  --provider opencode --scope project --path C:\Projects\Example --yes

# Sync varsayılan dry-run
.\dna-cli.exe deploy sync --profile .\profiles\universal.json `
  --provider opencode --scope project --path C:\Projects\Example

# Sync apply iki bayrak ister
.\dna-cli.exe deploy sync --profile .\profiles\universal.json `
  --provider opencode --scope project --path C:\Projects\Example --apply --yes
```

Çakışmada varsayılan politika `abort`tur. `skip` veya
`backup-and-replace` ancak sonuçları bilinerek açıkça seçilmelidir.

## PLAN, BUILD ve REVIEW

```powershell
.\dna-cli.exe plan --project C:\Projects\Example `
  --request "Yeni özelliği planla" --language rust
.\dna-cli.exe build --project C:\Projects\Example
.\dna-cli.exe review --project C:\Projects\Example --language rust
```

`build` gerçek kaynak düzenlemez; provider ajanına izin ve doğrulama sözleşmesi
üretir. Onaylı sözleşme için `--approve` gerekir.

## Project Memory

```powershell
# Önizleme
.\dna-cli.exe init --profile .\tests\fixtures\profile-rust.json `
  --project C:\Projects\Example --provider codex

# Onaylı yazma
.\dna-cli.exe init --profile .\tests\fixtures\profile-rust.json `
  --project C:\Projects\Example --provider codex --confirm

.\dna-cli.exe resume --project C:\Projects\Example
.\dna-cli.exe memory check --project C:\Projects\Example
.\dna-cli.exe handoff --project C:\Projects\Example --provider claude
```

## Quality Gates ve performans

```powershell
.\dna-cli.exe test --project C:\Projects\Example --language rust
.\dna-cli.exe quality --project C:\Projects\Example --language rust
.\dna-cli.exe security --project C:\Projects\Example --language rust
.\dna-cli.exe performance --project C:\Projects\Example --language rust
.\dna-cli.exe release-check --project C:\Projects\Example --language rust
```

Araç kurulmaz ve format/lint sırasında otomatik düzeltme yapılmaz. Zorunlu eksik
kontrol release sonucunu `BLOCKED` yapar.

## Auto-Fix ve rollback

```powershell
.\dna-cli.exe optimize --project C:\Projects\Example --language rust
.\dna-cli.exe optimize --project C:\Projects\Example --language rust --mode preview
.\dna-cli.exe fix --project C:\Projects\Example --language rust `
  --finding-id FINDING_ID --confirm
.\dna-cli.exe optimize --project C:\Projects\Example --language rust `
  --mode rollback --fix-id FIX_ID --confirm
```

Her komutun geçerli seçenekleri için `--help` kullanın. Bu belge doğrulanan CLI
yüzeyini gösterir; provider'a özel canlı davranış için [PROVIDERS.md](PROVIDERS.md).
