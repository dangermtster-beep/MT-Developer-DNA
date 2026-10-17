---
name: mt-developer-dna
description: Plan, implement, test, secure, document, and prepare Rust software projects for release while preserving the user's language choice and existing architecture.
---

# MT Developer DNA

Bu skill Claude Code için üretilmiştir. Çağırma biçimi: `/mt-developer-dna`.

## İş akışı

1. Yeni projede gereksinimleri; mevcut projede önce kodu, bağımlılıkları ve testleri incele.
2. Kritik eksikleri belirle. Dil belirtilmemişse uygun alternatifler sun; seçimi kullanıcıya bırak.
3. Orantılı mimari ve geliştirme planı oluştur; gerekli onayları almadan kapsamı genişletme.
4. Çalışan kodu gereksiz yere yeniden yazmadan en küçük bütünlüklü değişikliği uygula.
5. İlgili testleri, güvenlik kontrollerini ve release derlemesini çalıştır.
6. Sonucu; değişiklikleri, gerçek doğrulama kanıtını ve sınırlamalarıyla raporla.

## PLAN, BUILD ve REVIEW modları

- `PLAN`: projeyi sınırlı ve ignore-aware tara; gereksinim, mimari, görev bağımlılıkları ve doğrulama komutları üret. Varsayılan olarak kaynak veya hafıza yazma.
- `BUILD`: yalnızca kullanıcı uygulamayı istediğinde veya onayladığında devreye gir. Onaylı plan kapsamındaki en küçük değişikliği provider izin modeliyle uygula; Rust çekirdeğinin kendi başına kaynak yazdığını ya da komut çalıştırdığını iddia etme.
- `REVIEW`: varsayılan olarak salt-okunurdur. Her bulguda önem, dosya/konum, kanıt, etki ve düzeltme önerisi ver; kısmi taramayı ve çalıştırılmayan testleri açıkça belirt.
- Niyet belirsizse güvenli REVIEW modunda kal ve uygulama öncesinde doğrulama iste.
- Proje hafızasındaki dil ve onaylı plan provider değişse de korunur. Plan/review kaydı için diff göster ve açık onay al.
- Bu provider için çağrılar: `/mt-developer-dna plan`, `/mt-developer-dna build`, `/mt-developer-dna review`.

## V0.5 kalite ve ölçüm sistemi

- `test`, `quality`, `performance`, `security` ve `release-check` kontrollerini talep üzerine çalıştır; daemon, watcher veya sürekli polling başlatma.
- Komutları shell metni olarak birleştirme. Dil adapter'ının program/argüman listesini, proje çalışma dizinini, timeout'u ve sınırlı çıktıyı kullan.
- Eksik araç veya çalıştırılamayan zorunlu kontrol `NOT_RUN`; desteklenmeyen metrik `UNAVAILABLE`; zorunlu `NOT_RUN/UNAVAILABLE` release sonucunu `BLOCKED` yapar.
- Format/lint sırasında auto-fix çalıştırma. Kaynak değişikliği yalnızca BUILD izniyle ayrı bir adımdır.
- Performansı warm-up/tekrar, ortam ve ölçüm yöntemiyle raporla; karşılaştırılamayan baseline için regresyon yüzdesi üretme. Temiz güvenlik taramasını garanti sayma.
- Rapor veya baseline hafızaya yazılacaksa `.mt-dna` diff'ini göster ve açık onay al; eski raporları varsayılan olarak silme.
- Bu provider için hedef çağrılar: `/mt-developer-dna test`, `/mt-developer-dna quality`, `/mt-developer-dna performance`, `/mt-developer-dna security`, `/mt-developer-dna release-check`.

## V0.6 kontrollü optimizasyon ve Auto-Fix

- `optimize --mode analyze` varsayılan ve salt-okunurdur. `preview` yalnız dosya listesi, unified diff, risk, test ve rollback planı üretir.
- `apply` yalnız açık kullanıcı onayı, düşük riskli doğrulanabilir yama, uygulama öncesi başarılı zorunlu kalite kapısı ve değişmemiş hedef hash'i ile çalışır. Güvenlik bulgusu kendiliğinden yetki vermez.
- `rollback --fix-id` yalnız ilgili V0.6 işlem kaydındaki dosyaları geri alır. Sonraki kullanıcı değişikliği veya hash uyuşmazlığında dur; toplu restore, `reset --hard` veya `clean` kullanma.
- Statik performans sezgisini ölçülmüş kazanç gibi sunma. Önce/sonra karşılaştırması aynı ortam ve iş yükünde warm-up ve tekrarlarla yapılmalıdır.
- İşlem raporunu `.mt-dna/reports/optimization/`, geri alma artifact'ini `.mt-dna/transactions/` altında tut; token veya hassas içerik rapora kopyalama.
- Bu provider için hedef çağrılar: `/mt-developer-dna optimize --mode analyze`, `/mt-developer-dna optimize --mode preview`, `/mt-developer-dna optimize --mode apply`, `/mt-developer-dna optimize --mode rollback --fix-id <id>`, `/mt-developer-dna fix --finding-id <id>` ve `/mt-developer-dna optimization-report`.

## V0.7 Universal Plugin Deployment ve Synchronization

- `deploy plan` ve `deploy sync` varsayılan olarak salt-okunur dry-run üretir. Install, update, sync apply ve rollback yalnız kullanıcının CLI'de verdiği açık `--yes` onayıyla dosya değiştirir.
- Registry dışı veya kayıtlı SHA-256 değerinden farklı dosyayı `user-modified/conflict` say. Sessiz üzerine yazma veya otomatik merge yapma; abort, skip ya da açık backup-and-replace politikası kullan.
- `provider all` yalnız executable keşfinde gerçekten bulunan provider'lara dağıtım yapar. Global ve project scope'u karıştırma; dil profilini kurulum sırasında değiştirme.
- Doctor sonuçlarında PASS, WARN, FAIL, NOT_RUN ve UNSUPPORTED ayrımını koru. Yapısal dosya doğrulamasını canlı interaktif çağrı olarak sunma.
- Provider çağrısı: `/mt-developer-dna`. `doctor`, `install` ve `sync` isteklerinde skill çağrısı tek başına mutasyon yetkisi değildir; Rust CLI planı ve açık onay gerekir.
- Codex doğrudan `/mt-developer-dna` sunmaz; `$mt-developer-dna` eşdeğeridir. Claude Code skill'i slash çağrısı sağlar. Gemini slash çağrısı `.gemini/commands/mt-developer-dna.toml` dosyasına dayanır ve `/commands list` ile doğrulanır.

## Zorunlu çalışma biçimi

- Kullanıcının seçtiği programlama dilini aynen koru: Rust.
- Kullanıcı arayüzü ve kullanıcıya dönük mesajlar Türkçe olsun.
- RAM verimliliğini ve düşük boşta CPU kullanımını önceliklendir.
- Performans değerlerini garanti değil, ölçülebilir benchmark hedefi olarak belirt.
- Her aşamada ilgili testleri çalıştır ve gerçek sonucu bildir.
- GPU bağımlılığı ekleme.

## Proje hafızası

- Ortak ve denetlenebilir bağlam yalnızca proje kökündeki `.mt-dna/` dosyalarından gelir; provider sohbet geçmişlerinin ortak olduğunu varsayma.
- `init`, `update`, `handoff` ve `memory repair` yazmalarından önce diff/önizleme göster ve açık onay al.
- `resume` ve `memory check` sırasında şema, hash, seçili dil, gizli bilgi ve kaynak güncelliği sorunlarını kesin gerçeklerden ayır.
- `.env`, anahtar, token, kişisel veri veya provider'a özgü geçici bağlamı hafızaya aktarma.
- Bu provider için hedef çağrılar: `/mt-developer-dna init`, `/mt-developer-dna resume`, `/mt-developer-dna update`, `/mt-developer-dna handoff`, `/mt-developer-dna memory check`, `/mt-developer-dna memory repair`.

## Ortak yazılım standartları

### Mimari

- Dar kapsamlı ve geri alınabilir değişiklik yap.
- I/O sınırlarını çekirdek iş mantığından ayır.

### Kalite

- Her aşamada ilgili otomatik testleri çalıştır.
- Derleyici uyarılarını açıkça değerlendir.

### Güvenlik

- Mevcut dosyayı açık onay olmadan değiştirme.
- Gizli bilgileri kaynak koda veya loglara yazma.

### Performans benchmark hedefleri

- Boşta CPU kullanımını benchmark ile ölç.
- Bellek hedeflerini ölçüm hedefi olarak raporla, garanti verme.

