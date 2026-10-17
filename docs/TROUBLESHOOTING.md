# Sorun giderme

## Provider kurulu değil

```powershell
.\dna-cli.exe providers list --json
.\dna-cli.exe doctor --provider PROVIDER --scope project --path C:\Projects\Example
```

PATH değiştiyse yeni terminal açın. Standart dışı Antigravity/Cline kurulumu için
tek provider seçiliyken `--provider-path` kullanın. Cline'da extension paketinin
bulunması seçili VS Code profilinde enabled olduğunu kanıtlamaz.

## OpenCode npm kurulumu algılanmıyor

```powershell
Get-Command opencode -All
opencode --version
```

Discovery PATH içinde `opencode.exe`, `opencode.cmd` ve `opencode.ps1` arar.
Execution policy değiştirilmez; `.ps1` engellenirse `.cmd` runner'ı kullanılabilir.

## Conflict nedeniyle kurulum durdu

Hedef dosya registry dışında veya son kurulumdan sonra değişmiştir. İçeriği inceleyin.
Varsayılan `abort` en güvenli politikadır. `--conflict skip` hedefi atlar;
`backup-and-replace` yalnız açıkça kabul edildiğinde kullanılmalıdır. Şüpheli sır
içeren dosya yedeklenmez.

## Hafıza hash veya stale-plan hatası

```powershell
.\dna-cli.exe memory check --project C:\Projects\Example
.\dna-cli.exe memory repair --project C:\Projects\Example
```

Önizlemeyi inceleyin; onay bayrağı olmadan onarım yazmaz. Başka bir süreç veya
kullanıcı değişikliği varsa yeni plan üretin.

## Quality check NOT_RUN veya UNAVAILABLE

`NOT_RUN`, ilgili tool'un bulunmadığı veya komutun çalıştırılamadığı anlamına gelir.
`UNAVAILABLE`, metrik/platform desteği olmadığını gösterir. Bunlar PASS değildir.
Gerekli tool'u kendiniz kurun, PATH'i doğrulayın ve kontrolü yeniden çalıştırın.

## Cargo hard-link uyarısı

Bazı Windows dosya sistemlerinde incremental cache hard-link kuramaz; Cargo dosya
kopyalayarak devam edebilir. Exit code 0 ise bu tek başına build/test hatası değildir.
Sorun sürerse checkout ve `target` dizinini hard-link destekleyen aynı volume üzerinde
tutun.

## Canlı slash çağrısı görünmüyor

Önce dosyanın doğru scope'a kurulduğunu ve `doctor` sonucunu kontrol edin. Gemini'de
`/skills reload` ve `/commands reload`; Cline/Cursor/Antigravity'de skill görünümü;
OpenCode'da slash metadata; Claude'da slash listesi doğrulanmalıdır. Codex çağrısı
slash değil `$mt-developer-dna` biçimindedir.
