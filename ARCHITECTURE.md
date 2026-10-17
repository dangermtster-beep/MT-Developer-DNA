# MT Developer DNA V0.8 Mimarisi

## Ürün sınırı

MT Developer DNA bir masaüstü uygulaması değil, farklı AI kodlama ajanlarına aynı
yazılım geliştirme çalışma biçimini taşıyan skill/plugin paketidir. UI framework veya
sürekli çalışan servis içermez; normal kullanımda boşta CPU/RAM tüketimi yoktur.

## Bağımlılık yönü

```text
dna-cli -> dna-render -> dna-core
       `-> dna-memory -> dna-core
       `-> dna-deploy -> dna-render
```

- `dna-core`: Geriye uyumlu V0.1 proje profili, genişletilebilir dil profili,
  skill doğrulaması ve ortak kurallar.
- `dna-render`: Mevcut `render_package(profile, provider)` API'sini koruyan adapter
  katmanı. Provider'a özgü yollar ve çağırma biçimleri burada sınırlandırılır.
- `dna-memory`: Sürümlü `.mt-dna` şeması, allowlist alanları, redaksiyon, sınırlı
  proje taraması, optimistic concurrency, atomik kayıt, handoff ve audit/repair.
- `dna-cli`: Salt-okunur provider/preview/plan işlemleri ile açık onaylı dosya
  mutasyonlarını ayırır.
- `dna-deploy`: Provider keşfi, scope çözümleme, deterministik SHA-256 manifest,
  registry, transaction lock, yedek, atomik apply, sync, status, doctor ve rollback.

## Deployment akışı

```text
detect -> render -> dry-run(create/update/unchanged/conflict)
       -> --yes -> lock -> backup -> atomic apply -> registry -> validate
       -> rollback: after-hash + backup-hash -> restore/remove owned files
```

Global ve project scope ayrı kayıtlardır. `provider all` yalnız algılanan provider'ları
seçer. Kayıtsız veya registry hash'inden farklı dosya user-modified sayılır ve
varsayılan olarak üzerine yazılmaz.

## Project Memory akışı

```text
kaynak taraması + profil -> plan/diff -> açık onay -> yazma kilidi
                        -> stale-plan/hash kontrolü -> yedek -> atomik kayıt
                        -> manifest en son yazılır
```

`.mt-dna` provider bağımsızdır; provider paketlerine kopyalanmaz. Adaptörler aynı
klasörü okumayı ve gerçek çağırma biçimleriyle `init`, `resume`, `update`, `handoff`,
`memory check` ve `memory repair` niyetlerini yönlendirir. Sohbet geçmişleri ortak
kabul edilmez. Tarama ihtiyaç anında ve sınırlıdır; watcher/polling veya GUI yoktur.

`MemoryStore` dosya yaşam döngüsünü, `SchemaValidator` sürümlü JSON kurallarını,
`SecretRedactor` yazma öncesi redaksiyonu, `ProjectScanner` kaynak parmak izini,
`HandoffRenderer` aktarım özetini ve `ConflictDetector` nesil/hash kontrolünü taşır.

## Skill paketi

Canonical `mt-developer-dna/SKILL.md` kısa yönlendirme ve karar kurallarını taşır.
Ayrıntılı standartlar ve dil profilleri yalnızca gerektiğinde okunmak üzere
`references/` altındadır. Bu yapı gereksiz bağlam ve bellek tüketimini azaltır.

## Provider adapter sözleşmesi

Her adapter ana talimat dosyasını, skill yolunu, açık çağırma biçimini, ek komut
dosyalarını ve doğrulama uyarılarını tanımlar. Ortak profil ve standart metni tek
renderer'dan gelir; provider farkları adapter sınırında kalır.

Codex doğrudan slash komut varsayımı yapmaz ve `$mt-developer-dna` kullanır. Claude
Code skill klasöründen `/mt-developer-dna` sağlar. Gemini CLI hem skill'i hem de
`/mt-developer-dna` custom command dosyasını alır.

## Güvenli kurulum akışı

```text
profil + provider -> render -> plan/diff -> açık onay -> yedek manifesti
                  -> atomik yazma -> idempotency kontrolü -> isteğe bağlı rollback
```

- Proje kökü ve her hedef bileşeni path traversal ile symlink/reparse açısından
  doğrulanır.
- Onaysız `install` yalnızca plan/diff üretir.
- Değişen eski içerik yedeklenir; yeni dosyalar manifestte kaydedilir.
- Windows'ta `MoveFileExW` ile replace + write-through uygulanır.
- İkinci aynı kurulum `Unchanged` olur ve yeni yedek üretmez.
- Rollback yalnızca `--confirm` ile eski dosyaları atomik geri yükler ve kurulumun
  oluşturduğu yeni dosyaları kaldırır; yedek kaydı korunur.

## Genişleme

Yeni dil, `LanguageProfile` JSON şemasına uygun bir profil eklenerek tanımlanabilir.
Yeni provider, adapter sözleşmesini uygulayarak eklenir. Var olan public `Profile`,
`Provider`, `ProviderPackage` ve `render_package` sözleşmeleri korunmuştur.

## Kontrollü optimizasyon akışı

```text
bounded ANALYZE -> PREVIEW (diff + risk + hash) -> açık onay
                -> pre-quality gate -> rollback artifact PREPARED
                -> atomik APPLY -> post-quality gate -> işlem raporu
                -> isteğe bağlı ROLLBACK (after-hash + backup-hash doğrulaması)
```

`dna-core` bulgu ve işlem sözleşmesini taşır; `dna-cli::optimization` dosya sistemi
sınırını uygular. ANALYZE/PREVIEW yazmaz. APPLY yalnız düşük riskli ve deterministik
dönüşümlerde, `.mt-dna` hafızası mevcutken ve hedef preview sonrasında değişmemişken
çalışır. Rollback yalnız ilgili işlem kaydındaki dosyaya dokunur; sonraki kullanıcı
değişikliğinde durur. Büyük, UTF-8 olmayan veya şüpheli sır içeren dosyalar otomatik
yedeklenmez/düzeltilmez.

Performans sezgileri ölçüm değildir. Auto-Fix kapsamına alınmayan polling ve gereksiz
kopyalama adayları, aynı cihaz/konfigürasyon/iş yükündeki önce-sonra ölçümleri
sağlanana kadar yalnız öneri olarak kalır.
