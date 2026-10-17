# CODEX GÖREVİ — MT Developer DNA V0.1 (ARŞİV)

> Bu dosya ilk geliştirme aşamasının tarihsel şartnamesidir. Güncel ana şartname
> `MT_DEVELOPER_DNA_V0.7_CODEX_IMPLEMENTATION.md` dosyasıdır. V0.7, masaüstü UI
> planını kaldırır ve universal provider deployment/senkronizasyon sınırlarını
> tanımlar; çelişki halinde V0.7 geçerlidir.

## Amaç
Rust ile Windows odaklı, düşük kaynak tüketimli, kişisel yazılım standartları yönetim ve provider-paket üretim aracı geliştir. Projenin kapsamı yalnızca yazılım geliştirme standartlarıdır. Kullanıcı programlama dilini özgürce seçer.

## Teknoloji
- Rust stable; Cargo workspace
- İlk çalışan sürümde CLI zorunlu; Slint masaüstü UI aynı çekirdeği kullanan ayrı crate olarak eklensin.
- `serde`, `serde_json`, `clap`, `thiserror` gibi yerleşik ekosistem paketlerini gerekçeli kullan.
- Profilleri JSON dosyasıyla sakla; V0.1 için veritabanı gereksiz.
- İnternet ve bulut API bağımlılığı olmasın.

## Workspace
- `crates/dna-core`: tipli profil, doğrulama, kurallar, sürümleme
- `crates/dna-render`: ortak kurallardan provider çıktısı oluşturma
- `crates/dna-cli`: `validate`, `preview`, `export`, `install` komutları
- `crates/dna-ui`: Slint ile Türkçe profil düzenleme, önizleme, dışa aktarma (CLI tamamlandıktan sonra)
- `tests/fixtures`: altın dosya / snapshot testleri

## Çıktılar
- Codex CLI proje `AGENTS.md` ve uygun skill paketleri
- Claude Code proje `CLAUDE.md` ve uygun `.claude/skills/` yapısı
- Gemini CLI proje `GEMINI.md` ve sürümün desteklediği skill yapısı
- Ortak kurallar tek kaynakta tutulacak; sağlayıcı çıktılarına gerekli içerik doğrudan yazılacak (hedef klasörde olmayan göreli dosyalara bağımlı kalmayacak).
- Provider desteği ve dosya konumları için kurulu CLI sürümlerini doğrula; belirsiz desteklerde dosyayı zorla kurma, yalnızca export et.

## Profil
- Dil: Rust, C#, Python, C++, Go, Java, TypeScript veya Auto
- UI: Auto veya dile uyumlu seçenekler
- DB: Auto, None, SQLite, SQL Server, PostgreSQL
- Düşük boşta CPU, düşük RAM, GPU kullanımı, i18n, tema, test, loglama
- Performans hedefleri garanti değil; benchmark hedefleri olarak etiketlenmeli.
- Uyumsuz dil/UI kombinasyonunu sessiz değiştirme: açık hata ve alternatif sun.

## Güvenlik
- `preview` varsayılan davranış; `install` açık kullanıcı onayı ister.
- Mevcut AGENTS.md/CLAUDE.md/GEMINI.md dosyalarını asla sessizce ezme.
- Kurulumda diff göster, yedek al, atomik yaz; rollback seçeneği sağla.
- Path traversal ve symlink hedeflerini doğrula; yalnızca seçilen proje köküne yaz.
- Kişisel bilgileri, API anahtarlarını ve gizli dosyaları çıktılara gömme.

## Kabul kriterleri
1. JSON profil yüklenir ve doğrulanır.
2. Seçilen dil tüm provider çıktılarında aynen korunur.
3. Üç sağlayıcı için önizleme üretilebilir.
4. Çıktıların ortak kuralları tutarlı ve deterministiktir.
5. Kullanıcı onayı olmadan dosya değiştirilmez.
6. İkinci kurulumda idempotent davranış ve diff vardır.
7. Birim testleri ve golden-file testleri geçer.
8. UI boşta iken busy-loop veya sürekli render bulunmaz.
9. Windows Release paketi ve Türkçe README hazırlanır.

## Geliştirme sırası
1. Core + profil şeması + validasyon.
2. Provider renderer + golden tests.
3. CLI preview/export/install + güvenlik testleri.
4. Slint GUI ve ayarlar.
5. Windows release, benchmark ve kullanım dokümantasyonu.

Önce mimariyi ve görev listesini sun, ardından 1. aşamayı gerçek kod ve testlerle uygula. Sonraki aşamalara testleri geçtikten sonra ilerle. Mevcut örnek dosyaları üretim tamamlanmış gibi sunma.
