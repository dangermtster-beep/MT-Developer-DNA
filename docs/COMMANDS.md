# Komut rehberi

MT Developer DNA iki katmanda kullanılır:

1. **Skill komutları** — yapay zekâ kodlama aracının sohbetinde yazılır.
2. **`dna-cli` komutları** — terminalde çalışır; kurulum, hafıza ve kalite işlerini yapar.

Akış örnekleri için [USAGE.md](USAGE.md), kurulum için [INSTALL.md](INSTALL.md) dosyasına bakın.

## 1. Skill komutları (sohbet içinde)

| Provider | Çağrı biçimi |
|---|---|
| Claude Code, Gemini CLI, Antigravity, Cursor, Cline, OpenCode | `/mt-developer-dna` |
| Codex | `$mt-developer-dna` (Codex slash çağrısını desteklemez) |

| Komut | Ne yapar |
|---|---|
| `/mt-developer-dna plan <istek>` | Analiz ve geliştirme planı; dosya yazmaz |
| `/mt-developer-dna build <istek>` | Onaylı planı uygular |
| `/mt-developer-dna review` | Projeyi inceler, kanıtlı bulgular verir; salt-okunur |
| `/mt-developer-dna test` | Testleri çalıştırır |
| `/mt-developer-dna quality` | Test, format ve lint kontrolleri |
| `/mt-developer-dna performance` | Sınırlı tekrarlı ölçüm; ölçülemeyeni uydurmaz |
| `/mt-developer-dna security` | Gizli bilgi ve bağımlılık güvenlik taraması |
| `/mt-developer-dna release-check` | Yayın kapısı: test, kalite, build ve güvenlik |
| `/mt-developer-dna optimize --mode analyze` | Kod analizi (salt-okunur) |
| `/mt-developer-dna optimize --mode preview` | Düzeltme önizlemesi: diff, risk, rollback planı |
| `/mt-developer-dna optimize --mode apply` | Onaylı düzeltme |
| `/mt-developer-dna fix --finding-id <id>` | Tek bir bulguyu düzeltir |
| `/mt-developer-dna init` | `.mt-dna` proje hafızasını başlatır |
| `/mt-developer-dna resume` | Hafızadan kaldığın yeri özetler |
| `/mt-developer-dna update` | İlerleme ve karar kaydı ekler |
| `/mt-developer-dna handoff` | Başka bir provider'a geçiş özeti üretir |
| `/mt-developer-dna memory check` | Hafızayı denetler |
| `/mt-developer-dna memory repair` | Hafızayı önizlemeli onarır |

Komut yazmadan doğal dille de istenebilir; örneğin
`/mt-developer-dna ana ekranı planla` PLAN moduna geçer. Skill, seçili dili önce
projenin `.mt-dna/project.json` dosyasından okur; kayıt yoksa kullanıcıya sorar.

## 2. `dna-cli` komutları (terminal)

Her komutun tüm parametreleri için: `dna-cli <komut> --help`.

### Kurulum ve provider'lar

| Komut | Ne yapar |
|---|---|
| `dna-cli providers list` | Kurulu provider'ları, sürümlerini ve çağrı biçimlerini listeler |
| `dna-cli doctor --provider all --scope global` | Kurulumu, registry'yi ve dosya hash'lerini denetler |
| `dna-cli deploy plan --profile <profil> --provider <ad> --scope project\|global` | Dosya bazında dry-run planı |
| `dna-cli deploy install ... --yes` | Planı atomik olarak uygular |
| `dna-cli deploy sync --profile <profil> --provider all --scope global` | Kaynaktan yeniden üretir (dry-run) |
| `dna-cli deploy sync ... --apply --yes` | Senkronizasyonu uygular |
| `dna-cli deploy update ... --yes` | Provider'ı güncel paket sürümüne getirir |
| `dna-cli deploy status --scope global` | Kurulu dosyaların hash durumunu gösterir |
| `dna-cli deploy rollback --transaction <id> --yes` | Yalnız ilgili işlemdeki dosyaları geri alır |

Çakışma politikası `--conflict abort|skip|backup-and-replace` ile seçilir; varsayılan `abort`tur.

### Proje hafızası (`.mt-dna`)

| Komut | Ne yapar |
|---|---|
| `dna-cli init --profile <profil> --project <klasör> --provider <ad>` | Hafıza önizlemesi; `--confirm` ile yazar |
| `dna-cli resume --project <klasör>` | Dil, son karar, açık işler ve sıradaki adım özeti |
| `dna-cli update --project <klasör> --provider <ad> --completed "..." --next "..."` | İlerleme kaydı; `--decision`, `--decision-reason` ile karar kaydı |
| `dna-cli handoff --project <klasör> --provider <ad>` | Hedef provider için aktarım özeti; `--confirm` ile kaydeder |
| `dna-cli memory check --project <klasör>` | Şema, hash, gizli bilgi ve güncellik denetimi |
| `dna-cli memory repair --project <klasör>` | Onarım planı; yalnız açık onayla yazar |

### Plan, inceleme ve kalite

| Komut | Ne yapar |
|---|---|
| `dna-cli plan --project <klasör> --request "<istek>"` | Deterministik geliştirme planı; `--save --confirm` ile hafızaya ekler |
| `dna-cli build --project <klasör> --approve` | Onaylı plan için BUILD sözleşmesi üretir |
| `dna-cli review --project <klasör>` | Kanıtlı yapısal bulgular |
| `dna-cli test --project <klasör>` | Dil adapter'ının test komutu |
| `dna-cli quality --project <klasör>` | Test, format ve lint |
| `dna-cli performance --project <klasör>` | Sınırlı tekrarlı ölçüm |
| `dna-cli security --project <klasör>` | Gizli bilgi taraması ve kurulu güvenlik aracı |
| `dna-cli release-check --project <klasör>` | Release gate sonucu |
| `dna-cli optimize --project <klasör> --mode analyze\|preview\|apply\|rollback` | Analiz, önizleme, onaylı Auto-Fix, rollback |
| `dna-cli fix --project <klasör> --finding-id <id> --confirm` | Tek bulguyu uygular |
| `dna-cli optimization-report --project <klasör>` | Kaydedilmiş optimizasyon raporu |

### Profil ve paket

| Komut | Ne yapar |
|---|---|
| `dna-cli profiles list` | Yerleşik profilleri listeler |
| `dna-cli profiles validate --profile <dosya>` | Profili doğrular |
| `dna-cli profiles show --profile <dosya>` | Profil içeriğini gösterir |
| `dna-cli validate --profile <dosya>` | JSON profilini doğrular |
| `dna-cli preview --profile <dosya> --provider <ad>` | Üretilecek dosyaları terminalde gösterir |
| `dna-cli export --profile <dosya> --output <klasör>` | Paketleri yeni klasöre yazar; var olanı ezmez |

## Örnek: yeni bir projeye başlama

```powershell
dna-cli init --profile .\profil.json --project C:\Projeler\Ornek --provider claude
dna-cli init --profile .\profil.json --project C:\Projeler\Ornek --provider claude --confirm
dna-cli resume --project C:\Projeler\Ornek
```

İlk komut yalnız önizleme gösterir; ikinci komut hafızayı yazar. Ardından provider
içinde `/mt-developer-dna plan ...` (Codex'te `$mt-developer-dna plan ...`) ile
devam edilir.

## Güvenlik kuralları

- Dosya yazan her komut önce önizleme gösterir; yazmak için `--confirm`, `--yes`
  veya `--apply --yes` gerekir.
- Kullanıcının elle değiştirdiği dosyalar sessizce ezilmez; `conflict` olarak raporlanır.
- Eksik araç `NOT_RUN`, ölçülemeyen metrik `UNAVAILABLE` olarak raporlanır; başarı sayılmaz.
- Daemon, watcher veya sürekli polling yoktur; komutlar yalnız çağrıldığında çalışır.
