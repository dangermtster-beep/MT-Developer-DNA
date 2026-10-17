# MT Developer DNA

**MT Developer DNA**, desteklenen AI kodlama ortamlarında aynı yazılım geliştirme
standartlarını uygulayan hafif bir Rust CLI ve taşınabilir Agent Skill sistemidir.
Masaüstü uygulaması, daemon, watcher veya sürekli polling içermez.

**MT SOFTWARE tarafından geliştirilmiştir.** Sürüm: **v0.8.1** — Lisans: **MIT**.

## Temel özellikler

- Rust, C#, Python, C++, Go, Java ve TypeScript dil profilleri.
- Dil seçmeyen, kullanıcıya soran Universal Language Profile.
- Salt-okunur varsayılanlı PLAN ve REVIEW; açık onay sınırına sahip BUILD.
- Hash, redaksiyon, sınırlı tarama ve handoff destekli `.mt-dna` Project Memory.
- Talep üzerine test, format, lint, build, güvenlik ve performans kontrolleri.
- Kontrollü Auto-Fix önizleme/uygulama ve işlem kapsamlı güvenli rollback.
- Dry-run öncelikli provider kurulum, senkronizasyon, yedekleme ve Doctor sistemi.
- Codex, Claude Code, Gemini CLI, Google Antigravity, Cursor, Cline ve OpenCode adapter'ları.

Bir provider'ın algılanması, canlı model oturumunda skill'in çalıştığını tek başına
kanıtlamaz. Kanıt seviyeleri [docs/PROVIDERS.md](docs/PROVIDERS.md) dosyasındadır.

## Sistem gereksinimleri

- Rust `1.81+` ve Cargo.
- Hazır Windows paketi için 64-bit Windows ve MSVC uyumlu çalışma ortamı.
- Kurulum veya canlı kullanım için desteklenen provider'lardan en az biri.

```powershell
git clone https://github.com/dangermtster-beep/MT-Developer-DNA.git
cd MT-Developer-DNA
cargo build --workspace --release
cargo test --workspace
```

## Hızlı kullanım

```powershell
# Universal Profile doğrulaması
.\target\release\dna-cli.exe profiles validate --profile .\profiles\universal.json

# Provider discovery
.\target\release\dna-cli.exe providers list

# Dosya yazmayan kurulum planı
.\target\release\dna-cli.exe deploy plan `
  --profile .\profiles\universal.json `
  --provider codex --scope project --path C:\Projeler\Ornek

# Plan incelendikten sonra açık onaylı kurulum
.\target\release\dna-cli.exe deploy install `
  --profile .\profiles\universal.json `
  --provider codex --scope project --path C:\Projeler\Ornek --yes
```

Codex çağrısı `$mt-developer-dna`, slash destekli adapter çağrısı
`/mt-developer-dna` biçimindedir. Gerçek kabul için `doctor` ve provider içi
doğrulama adımları uygulanmalıdır.

## Bilgisayarda tüm provider'lar için kurulum

CLI'yi PATH'e kurun, ardından algılanan tüm provider'lara global skill'i yükleyin:

```powershell
cargo install --path crates/dna-cli --locked
dna-cli deploy sync --profile .\profiles\universal.json --provider all --scope global
dna-cli deploy sync --profile .\profiles\universal.json --provider all --scope global --apply --yes
dna-cli deploy status --scope global
```

- Global kurulum projeden bağımsızdır: profildeki dil, arayüz ve veritabanı
  global skill'e yazılmaz. Skill dili önce projenin `.mt-dna/project.json`
  dosyasından okur, kayıt yoksa kullanıcıya sorar.
- Her skill, ayrıntılı sözleşmeleri içeren `references/` klasörüyle birlikte kurulur.
- npm ile kurulan Codex ve OpenCode (`.cmd`/`.ps1` shim) otomatik algılanır.

## PLAN / BUILD / REVIEW

- **PLAN:** Sınırlı analiz ve geliştirme planı; varsayılan olarak kaynak yazmaz.
- **BUILD:** Onaylı uygulama sözleşmesi üretir; gerçek düzenlemeyi provider ajanı kendi izin modeliyle yapar.
- **REVIEW:** Kanıt, önem, konum, etki ve çözüm önerisi üretir; varsayılan salt-okunurdur.

## Project Memory

Ortak ve denetlenebilir hafıza yalnız hedef projenin `.mt-dna/` klasöründedir.
Yazmalar önce diff/önizleme üretir ve onay ister. Provider sohbet geçmişleri ortak
hafıza sayılmaz; `.env`, anahtar ve token içerikleri hafızaya alınmaz.

## Quality Gates ve performans

Kontroller yalnız çağrıldığında çalışır. Eksik zorunlu araç `NOT_RUN`, ölçülemeyen
metrik `UNAVAILABLE` olarak raporlanır. RAM/CPU kazanımı ölçülmeden kesin performans
iddiası yapılmaz. GPU bağımlılığı eklenmemiştir.

## Auto-Fix, rollback ve senkronizasyon

Auto-Fix yalnız düşük riskli, önizlenmiş ve hash'i değişmemiş hedeflere uygulanır.
Rollback yalnız ilgili transaction dosyalarına dokunur. `deploy sync` dry-run'dır;
uygulama için `--apply --yes` gerekir. Kullanıcı tarafından değiştirilmiş provider
dosyaları sessizce ezilmez.

## Güncelleme

SemVer kullanılır. Sürüm kontrolü GitHub Releases üzerinden elle yapılır; kullanıcı
onayı olmadan otomatik indirme veya kurulum yapılmaz. Ayrıntılar:
[docs/UPDATE.md](docs/UPDATE.md).

## Belgeler

[Kurulum](docs/INSTALL.md) · [Kullanım](docs/USAGE.md) ·
[Komut rehberi](docs/COMMANDS.md) ·
[Provider'lar](docs/PROVIDERS.md) · [Mimari](docs/ARCHITECTURE.md) ·
[Performans](docs/PERFORMANCE.md) · [Sorun giderme](docs/TROUBLESHOOTING.md)

MIT License — [LICENSE](LICENSE).

**MT SOFTWARE tarafından geliştirilmiştir.**
