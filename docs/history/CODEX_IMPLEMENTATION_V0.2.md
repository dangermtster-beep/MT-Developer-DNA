# MT Developer DNA — V0.2 ana teknik şartname

## Amaç

MT Developer DNA; Codex, Claude Code ve Gemini CLI için yazılım geliştirme
standartlarını yöneten evrensel bir skill/plugin sistemidir. Bağımsız masaüstü
uygulaması değildir ve UI framework geliştirmesi kapsam dışıdır.

## Korunacak mimari

- `dna-core`
- `dna-render`
- `dna-cli`

Çalışan kod gereksiz yere yeniden yazılmaz; mevcut public API sözleşmeleri ve V0.1
profil uyumluluğu korunur.

## Skill ve dil profilleri

- Canonical giriş `mt-developer-dna/SKILL.md` dosyasıdır.
- Claude Code ve Gemini CLI hedef çağrısı `/mt-developer-dna`; Codex eşdeğer çağrısı
  `$mt-developer-dna` biçimidir.
- Rust, C#, Python, C++, Go, Java ve TypeScript başlangıç profilleri sağlanır.
- Dil belirtilmemişse alternatifler sunulur; açık seçim onaysız değiştirilmez.
- Yeni diller doğrulanabilir veri profilleriyle eklenebilir.

## Çalışma biçimi

Skill analiz, kritik bilgi toplama, dil ve proje türü belirleme, orantılı mimari,
geliştirme planı, dar kapsamlı uygulama, performans/güvenlik kontrolleri, test,
dokümantasyon ve release hazırlığını yönetir. Mevcut projede önce gerçek kod,
teknoloji ve test yapısı incelenir.

## Standartlar

SOLID bağlama göre; DRY, KISS, Separation of Concerns, modülerlik, güvenli hata
yönetimi, bağımlılık yönetimi, test edilebilirlik, sürümleme ve dokümantasyon
uygulanır. Gereksiz soyutlama ve aşırı mühendislik yapılmaz.

Performans politikası düşük RAM/boşta CPU, olay tabanlı mimari, lazy loading,
gereksiz polling/GPU kullanımından kaçınma, kontrollü paralellik ve verimli I/O
üzerine kuruludur. Ölçülmeyen değerler garanti olarak sunulmaz.

## Güvenlik ve kurulum

- Onaysız kurulum dosya değiştirmez.
- Kurulum öncesi diff, açık onay, yedek, atomik yazma ve rollback sağlanır.
- Path traversal ve symlink/reparse hedefleri reddedilir.
- API anahtarı, gizli bilgi ve kişisel veri kaynak veya loglara yazılmaz.
- Provider'ın gerçek izin ve çağırma modeli kullanılır; desteklenmeyen özellik iddia
  edilmez.

## Kabul

- Skill ve yedi dil profili doğrulanır.
- Üç provider paketi deterministik üretilir.
- Dosya yolları güvenlidir.
- Install idempotenttir; mevcut dosya yedeklenir ve rollback çalışır.
- V0.1 profil fixture'ı geriye uyumlu kalır.
- Rust unit/golden/integration testleri, Clippy ve Release build geçer.
- Provider canlı entegrasyon durumu doğrulanmış, engelli veya doğrulanmamış olarak
  açıkça raporlanır.
