# MT Developer DNA V0.3 — Project Memory & Provider Handoff

**Durum:** Codex için uygulama şartnamesi  
**Ön koşul:** V0.2 mevcut ve çalışıyor; gerçek kod incelenmeden tamamlanmış özellik varsayılmayacak.  
**Ürün:** `/mt-developer-dna` ile çağrılan AI yazılım geliştirme skill/plugin sistemi. **Masaüstü GUI oluşturulmayacak.**

## 1. Amaç

Codex, Claude Code ve Gemini CLI arasında taşınabilen, proje bazlı, **dosya temelli ve denetlenebilir** geliştirme hafızası oluştur. Aynı projenin seçilmiş programlama dili, mimari kararları, kalite standartları, ilerlemesi ve doğrulanmış bulguları sağlayıcı değişse de korunabilsin. Sağlayıcıların sohbet geçmişlerinin ortak olduğu iddia edilmesin.

## 2. Değişmez kurallar

1. Kullanıcı seçtiği programlama dilini belirler; açık onay olmadan değiştirme.
2. Mevcut `dna-core`, `dna-render`, `dna-cli` ve V0.2 skill/provider adaptörlerini koru; önce API ve dosya şemalarını incele.
3. `/.mt-dna` proje kökünde bulunur; depo dışına taşan yolları reddet. Kullanıcı onayı olmadan mevcut dosyaları silme/ezme.
4. Gizli bilgiler, API anahtarları, tokenlar, kişisel veriler ve `.env` içerikleri hafızaya yazılmaz. Yazmadan önce redaksiyon ve güvenli alan doğrulaması yap.
5. Hafızadaki bilgiler *kanıt*, *karar*, *varsayım* ve *öneri* olarak ayrılır. Doğrulanmamış bilgiyi kesin gerçek diye sunma.
6. Kaynak kullanımı: boşta sürekli polling yok; ihtiyaç halinde dosya okuma/yazma, atomik kayıt, sınırlı I/O.
7. Kullanıcının başka bir proje üzerindeki verilerini projeler arasında otomatik birleştirme.

## 3. Klasör şeması

```text
<project-root>/.mt-dna/
├── project.json             # Proje kimliği, seçili dil, platform, şema sürümü
├── architecture.md          # Mevcut mimari ve modüller (kaynak/kanıt referanslarıyla)
├── decisions.md             # Tarihli ADR-benzeri karar kayıtları
├── standards.json           # Etkin kalite, güvenlik ve performans kuralları
├── progress.md              # Yapıldı / sürüyor / sıradaki / engeller
├── handoff.md               # Provider değişiminde kısa bağlam aktarımı
├── manifest.json            # Yönetilen dosyalar, sürümler, hash'ler
└── reports/
    └── YYYY-MM-DD-review.md
```

`project.json` örneği (alanlar mevcut V0.2 şemasıyla uyumlu olacak biçimde uyarlanabilir):

```json
{
  "schema_version": "0.3.0",
  "project_id": "example-project",
  "name": "Example Project",
  "language": "rust",
  "project_type": "cli",
  "platforms": ["windows"],
  "created_at": "2026-10-16T00:00:00Z",
  "updated_at": "2026-10-16T00:00:00Z",
  "last_provider": "codex"
}
```

**Not:** Zaman damgaları çalışma anında UTC ISO 8601 olarak üretilecek; örnek tarih sabit kodlanmayacak. `project_id` kullanıcıya ait kimlik bilgisi değil, yerel proje tanımlayıcısıdır.

## 4. Hafıza iş akışları

### A. Initialize
- `/mt-developer-dna init` veya doğal dilde proje hafızası başlatma isteğini tanı.
- Proje kökünü güvenli şekilde belirle; mevcut `.mt-dna` varsa silme veya otomatik sıfırlama yapma.
- Var olan kod tabanını yalnızca erişilebilen dosyalar üzerinden incele; seçili dil açık değilse sor.
- Oluşturulacak dosyaları önizle, kullanıcı onayı gereken yazma işlemlerinde onay al.
- Şemayı doğrula ve kayıtları atomik yaz (geçici dosya + rename).

### B. Update
- Kod veya yapılandırma değiştiğinde yalnızca ilgili hafıza dosyalarını güncelle.
- Her karar kaydına tarih, bağlam, seçenekler, karar, gerekçe ve etkilenen dosyaları ekle.
- `progress.md` durumunu test sonuçlarıyla uyumlu tut; çalıştırılmayan testi geçti olarak işaretleme.
- Çakışma varsa sessizce üzerine yazma; diff göster ve onay iste.

### C. Resume
- `/mt-developer-dna resume` çağrısında `.mt-dna` içeriğini doğrula.
- Kullanıcıya kısa özet sun: dil, mevcut mimari, son karar, biten işler, açık riskler ve sonraki adım.
- Hafızayı mevcut kaynak koduyla karşılaştır; eski veya tutarsız bilgileri işaretle.

### D. Handoff
- `/mt-developer-dna handoff` ile `handoff.md` üret.
- En fazla gerekli bilgileri taşı: proje hedefi, dil/stack, kararlar, tamamlanan işler, açık işler, doğrulanan testler, riskler, ilk önerilen komut.
- Provider'a özgü gizli veya geçici bağlamı dışarı aktarma.
- Codex / Claude Code / Gemini CLI adaptörleri aynı `.mt-dna` içeriğini okuyabilsin; gerçek komut/skill çağırma biçimini her provider'ın mevcut desteğine göre oluştur ve doğrula.

### E. Audit / Repair
- `/mt-developer-dna memory check`: JSON şeması, eksik dosyalar, bozuk linkler, şüpheli sırlar, uyumsuz dil bilgisi, hash tutarsızlığı ve eski kayıtları raporla.
- `/mt-developer-dna memory repair`: önce değişiklik önizlemesi ve yedek; onay sonrası onarım.

## 5. Rust uygulama tasarımı

- Mevcut workspace'i incele; gerekiyorsa `dna-core` içinde `memory` modülü veya ayrı `dna-memory` crate'i aç. Yeni crate zorunlu değil; mevcut mimariye göre karar ver.
- Ayrı sorumluluklar: `MemoryStore`, `SchemaValidator`, `SecretRedactor`, `ProjectScanner`, `HandoffRenderer`, `ConflictDetector`.
- Dosya yolları için canonicalization / symlink escape kontrolleri uygula; var olmayan dosyalarda parent doğrulaması yap.
- JSON serde ile sürümlü şema; Markdown şablonları provider'dan bağımsız olsun.
- Çakışan eşzamanlı yazmalarda veri kaybını önle (kilit veya optimistic concurrency + hash).
- Şema yükseltmelerinde geriye dönük uyumluluk, dry-run migration ve yedekleme sun.
- Gizli veri taraması tek başına güvenlik garantisi değildir; allowlist alan yaklaşımı kullan.

## 6. CLI / skill komutları

| Komut | Beklenen davranış |
|---|---|
| `/mt-developer-dna init` | Proje hafızasını başlat |
| `/mt-developer-dna resume` | Hafızadan güvenilir bağlam özeti çıkar |
| `/mt-developer-dna update` | Proje değişikliklerini hafızaya işle |
| `/mt-developer-dna handoff` | Provider bağımsız aktarım özeti hazırla |
| `/mt-developer-dna memory check` | Bütünlük ve gizli veri denetimi |
| `/mt-developer-dna memory repair` | Onaylı onarım / migration |

**Uyumluluk notu:** Bunlar hedef kullanıcı ifadeleridir. Her provider'ın slash komut sözdizimi farklı olabilir. Adaptör desteklemiyorsa uygun skill çağrısı veya doğal dil tetikleyicisi kullan; desteklenmeyen özelliği çalışıyor diye raporlama.

## 7. Kabul testleri

1. Boş Rust projesinde `init` dosyaları doğru oluşturur; ikinci kez çağrıldığında veri kaybı olmaz.
2. Kullanıcının seçtiği dil `update`, `resume` ve `handoff` sonrasında değişmez.
3. `resume` doğru son karar ve açık işleri gösterir; kaynakla çelişen bilgileri işaretler.
4. Codex'te oluşturulan hafıza, Claude/Gemini adaptörleri tarafından şema kaybı olmadan okunabilir (desteklenen ortamlarda entegrasyon testi).
5. `.env` içindeki sahte test anahtarı hafıza dosyalarına yazılmaz.
6. Symlink/path traversal ile proje dışına yazma girişimi engellenir.
7. Eşzamanlı iki güncelleme sessiz veri kaybına yol açmaz.
8. Bozuk JSON ve eski şema sürümleri anlaşılır hata veya güvenli migration önerisi üretir.
9. Hiçbir test yapılmadığında raporda 'test edilmedi' yazılır.
10. Plugin boşta sürekli CPU kullanan watcher veya GUI başlatmaz.

## 8. Codex uygulama sırası

1. V0.2 kodunu, mevcut skill dosyalarını, provider adaptörlerini ve testleri incele; gerçek durum raporu yaz.
2. Bellek şemasını ve mevcut modüllerle entegrasyon planını çıkar.
3. `init`, `resume`, `update` için çekirdek uygulama ve birim testlerini tamamla.
4. `handoff`, `check`, `repair` akışlarını ve güvenlik kontrollerini ekle.
5. Provider adaptörlerini güncelle; yalnızca gerçek desteklenen çağırma biçimlerini kullan.
6. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace --release` çalıştır; ortam nedeniyle çalışmayanları açıkça belirt.
7. Değişen dosyaları, test çıktısını, kalan riskleri ve V0.4 önerilerini raporla.

## 9. Codex'e doğrudan verilecek talimat

> Mevcut MT Developer DNA V0.2 Rust workspace'ini incele. Bu belgedeki **V0.3 Project Memory & Provider Handoff** özelliklerini, mevcut kodu ve `/mt-developer-dna` skill mimarisini bozmadan uygula. Masaüstü arayüzü geliştirme. Önce durum tespiti yap, sonra dosya temelli proje hafızasını, güvenli güncellemeyi, provider aktarımını ve testleri tamamla. Kullanıcının seçtiği programlama dilini koru. Yapamadığın veya doğrulayamadığın işlemleri tamamlandı diye bildirme.
