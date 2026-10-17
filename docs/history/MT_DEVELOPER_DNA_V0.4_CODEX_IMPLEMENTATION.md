# MT Developer DNA V0.4 — Intelligent Project Development Engine

**Durum:** Codex uygulama şartnamesi  
**Kapsam:** Mevcut V0.1–V0.3 Rust workspace ve provider skill paketleri üzerine ek geliştirme  
**Temel çağrı:** `/mt-developer-dna` (yalnızca sağlayıcının desteklediği çağrı biçiminde)

## 1. Amaç ve sınırlar

MT Developer DNA bağımsız bir masaüstü uygulaması değildir. Codex, Claude Code ve Gemini CLI gibi kodlama ajanları için yazılım geliştirme iş akışını yöneten bir skill/plugin paketidir. V0.4; proje analizi, gereksinim çıkarımı, mimari önerisi, planlama ve kontrollü uygulama modlarını ekler. **Slint, Tauri, React veya masaüstü yönetim arayüzü oluşturma.**

V0.1–V0.3 bileşenlerini silme veya yeniden yazma. Önce mevcut gerçek kodu, CLI komutlarını, şemaları, testleri ve V0.3 `.mt-dna` hafıza biçimini incele. Aşağıdaki dizinler örnektir; mevcut repo düzenine uyarlanmalıdır. Geriye dönük uyumluluk önceliklidir.

## 2. Kullanıcı deneyimi ve komutlar

Örnek istekler:

```text
/mt-developer-dna Rust ile hafif bir dosya yöneticisi geliştir.
/mt-developer-dna plan C# WPF ile beton takip uygulaması.
/mt-developer-dna build
/mt-developer-dna review
/mt-developer-dna Bu projeyi analiz et ve eksiklerini raporla.
```

`plan`, `build`, `review` alt komutları **hedef davranış** olarak tanımlanır. Her provider'da slash argümanlarının aynı şekilde işlendiği varsayılmamalıdır. Gerçek desteklenen çağırma biçimi adaptör belgelerinde belirtilmeli ve test edilmelidir.

İstek açık bir mod içermiyorsa: yalnızca analiz isteniyorsa REVIEW; yeni proje tasarımı isteniyorsa PLAN; açıkça uygulama isteniyorsa, mevcut izinler ve onay kuralları içinde BUILD seçilebilir. Belirsiz ve yüksek etkili işlemlerde kullanıcıya sor. Kullanıcının seçtiği programlama dilini izinsiz değiştirme. Dil belirtilmediyse gerekçeli seçenekler sun veya bağlam yeterliyse öneriyi teyit et.

## 3. Mod sözleşmeleri

### PLAN — salt okunur planlama
- Proje varlığını, yapılandırmasını, kaynak dosyalarını ve V0.3 proje hafızasını oku.
- Gereksinimleri işlevsel / işlevsel olmayan / kabul kriterleri şeklinde ayır.
- Teknoloji seçeneklerini, mimari kararları, riskleri, test planını ve aşamalı işleri üret.
- **Proje kaynak kodunu değiştirme.** Hafıza veya plan dosyası yazılması da kullanıcı iznine / mevcut repo politikasına tabi olsun.
- Çıktı: `DevelopmentPlan` + okunabilir Markdown raporu.

### BUILD — kontrollü uygulama
- Mevcut veya onaylı planı yükle; yoksa kısa plan üret.
- Dosya değişikliklerini kapsamla; riskli/yıkıcı adımlardan önce açık onay al.
- Küçük, doğrulanabilir adımlarla kodla; uygun test, lint ve derleme komutlarını çalıştır.
- Her adım sonunda gerçekleşen değişiklikleri ve sonuçları kaydet.
- Test başarısızlığını başarı gibi raporlama; çalıştırılamayan testleri ayrı göster.
- İptal / hata durumunda kısmi değişiklikleri açıkça bildir.

### REVIEW — inceleme
- Varsayılan olarak salt okunur; kullanıcı açıkça düzeltme istemedikçe kod değiştirme.
- Hataları önem derecesi, dosya/konum, kanıt, etki ve önerilen çözümle raporla.
- Performans için ölçülmemiş RAM/CPU değerleri uydurma; önerileri varsayım olarak etiketle.
- Çıktı: `ReviewReport` + Markdown özet.

## 4. Önerilen modüller

Mevcut `dna-core`, `dna-render`, `dna-cli` modüllerine uyumlu, küçük bileşenler ekle. Gerekmedikçe yeni crate açma.

- **ProjectAnalyzer:** Dil, build sistemi, bağımlılıklar, testler, kaynak yapısı, giriş noktaları ve mevcut `.mt-dna` hafızasını tespit eder.
- **RequirementEngine:** Kullanıcı isteğini gereksinimler, kısıtlar ve açık sorulara dönüştürür.
- **ArchitectureAdvisor:** Seçilen dil ve proje türüne uygun en basit sürdürülebilir mimariyi önerir.
- **DevelopmentPlanner:** İş paketleri, bağımlılıklar, kabul kriterleri ve doğrulama komutlarını oluşturur.
- **ModeRouter:** PLAN/BUILD/REVIEW niyetini belirler; belirsizlikte güvenli varsayılan uygular.
- **ReportRenderer:** Yapılandırılmış sonuçları Markdown'a dönüştürür.
- **MemoryBridge:** V0.3 proje hafızasını okur ve yalnızca izinli, doğrulanmış güncellemeleri yazar.

**Önemli ayrım:** Rust çekirdeği deterministik analiz, şema doğrulama, plan/rapor saklama ve sağlayıcı paket üretiminden sorumludur. Kaynak kod yazma, mimari muhakeme ve test çalıştırma gibi ajan yetenekleri, ilgili provider'ın izin verilen araçlarıyla gerçekleştirilir. Rust çekirdeğine var olmayan bir LLM veya terminal yeteneği atfetme.

## 5. Örnek veri sözleşmeleri

Mevcut şemalar varsa bunları kırmadan genişlet; aşağıdakiler kavramsal örnektir.

```json
{
  "schema_version": "0.4",
  "project_id": "example-project",
  "mode": "PLAN",
  "language": "rust",
  "project_type": "desktop",
  "requirements": [
    {"id": "REQ-001", "text": "Düşük boşta CPU kullanımı", "priority": "high", "acceptance": "Boşta CPU ölçümü raporlanır"}
  ],
  "constraints": ["Kullanıcının seçtiği dil korunur"],
  "open_questions": [],
  "tasks": [
    {"id": "TASK-001", "title": "Mevcut projeyi analiz et", "depends_on": [], "status": "pending"}
  ]
}
```

Kimlikler kararlı olmalı; serileştirme deterministik olmalı. Bilinmeyen `schema_version` değerleri güvenli hata üretmeli. Kullanıcı girdisi JSON/Markdown'a yazılırken uygun biçimde işlenmeli.

## 6. V0.3 proje hafızası entegrasyonu

Var olan `.mt-dna` düzenini ve sürümleme sözleşmesini koru. V0.4 için gerekli alanlar eklenirse uyumlu göç / okuma mantığı ve testler oluştur. Plan, karar, ilerleme ve inceleme kayıtları birbirinden ayrılmalı. Provider değişiminde aşağıdaki bilgiler aktarılabilmeli:

- Seçilen programlama dili ve teknoloji kısıtları
- Mimari kararlar ve gerekçeleri
- Onaylanan geliştirme planı ve görev durumları
- Son test sonuçları, bilinen sorunlar ve sonraki adımlar
- Kaynak kodun kendisi yerine ilgili dosya yolları / commit bilgileri

Gizli anahtar, token, parola, özel ortam değişkeni ve gereksiz kişisel verileri hafızaya yazma. Handoff raporunda kanıtlanmamış tamamlanma iddiaları üretme.

## 7. Kalite ve performans ilkeleri

- Olay odaklı çalışma; boşta sürekli polling yapma.
- Büyük repolarda sınırlı tarama, ignore kurallarına uyum, gerektiğinde artımlı analiz.
- `target`, `.git`, `node_modules`, sanal ortamlar ve derleme çıktıları varsayılan olarak tarama dışında; kullanıcı isteğiyle değiştirilebilir.
- Dosya boyutu / sayısı sınırları ve zaman aşımı tanımla; kısmi taramayı raporla.
- Belleğe tüm repo içeriğini yüklemek yerine akış veya kademeli işleme kullan.
- Performans iddialarını ölçüme bağla; düşük CPU kullanımı için işi anlamsızca yavaşlatma.
- Dosya yollarında traversal/symlink risklerini değerlendir; kapsam dışına çıkma.
- Dış komutları güvenilir argümanlarla çalıştır; shell injection'dan kaçın.
- Provider izin mekanizmalarını aşma; yazma ve yıkıcı işlemlerde açık onay sınırlarını koru.

## 8. Test planı ve kabul kriterleri

**Birim testleri**
1. PLAN/BUILD/REVIEW yönlendirmesi ve belirsiz niyet.
2. Dil seçiminin korunması; uyumsuz teknoloji için uyarı.
3. Gereksinim / plan şema doğrulaması ve deterministik çıktı.
4. Eksik/bozuk/eski V0.3 hafızasının güvenli ele alınması.
5. Ignore kuralları, dosya sınırları ve yol güvenliği.

**Entegrasyon testleri**
6. Boş klasörde yeni proje PLAN raporu.
7. Örnek Rust ve C# projelerinde salt okunur REVIEW.
8. BUILD modunda onay/izin sınırları ve başarısız test raporu.
9. V0.3 hafızasından yükleme ve provider handoff bütünlüğü.
10. Codex, Claude Code, Gemini CLI paket üretimi; sağlayıcıların gerçekten desteklediği çağrı yöntemiyle smoke test.

**Kabul koşulları**
- V0.1–V0.3 testleri regresyonsuz geçer.
- PLAN ve REVIEW varsayılan olarak kaynak kodu değiştirmez.
- BUILD için izinsiz yıkıcı işlem yapılmaz.
- Dil seçimi sessizce değiştirilmez.
- `.mt-dna` eski proje hafızası okunabilir kalır.
- Test/derleme sonuçları gerçek komut çıktılarıyla raporlanır.
- Yeni masaüstü arayüzü eklenmez.

Uygun olduğunda:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Komutlar başarısız olursa nedenini ve çözüm durumunu açıkça yaz; ortamda olmayan araçları çalışmış gibi gösterme.

## 9. Uygulama sırası

**Aşama A — Keşif:** Repo ve V0.3 sözleşmelerini incele; değişiklik etki analizi ve mevcut test baz çizgisini çıkar.  
**Aşama B — Sözleşmeler:** Mod yönlendirme, proje analizi, gereksinim ve plan veri modellerini oluştur.  
**Aşama C — İş akışı:** PLAN, BUILD ve REVIEW davranışlarını provider skill talimatlarına entegre et.  
**Aşama D — Hafıza:** V0.3 köprüsü, güvenli kayıt ve handoff raporlarını ekle.  
**Aşama E — Doğrulama:** Birim, entegrasyon, regresyon ve gerçek provider smoke testlerini tamamla.

## 10. Codex'e doğrudan uygulanacak görev

> Mevcut MT Developer DNA deposunda V0.4 Intelligent Project Development Engine'i geliştir. Önce V0.1–V0.3 kodlarını ve proje hafızası şemasını incele; bunları bozmadan PLAN, BUILD, REVIEW modlarını, proje analizi, gereksinim çıkarımı, mimari planlama ve V0.3 MemoryBridge entegrasyonunu ekle. Plugin/skill mimarisini koru; masaüstü UI oluşturma. Kullanıcının seçtiği programlama dilini değiştirme. Yazma/yıkıcı işlemlerde izinleri uygula. Provider çağrı desteğini varsayma, doğrula. Kod, test, belgeler ve gerçek test sonuçlarıyla teslim et. Önce kısa uygulama planı çıkar, sonra aşama aşama uygula.

## 11. Teslimat raporu formatı

```text
MT Developer DNA V0.4 — Implementation Report
- İncelenen mevcut modüller:
- Eklenen/değiştirilen dosyalar:
- PLAN/BUILD/REVIEW durumu:
- V0.3 hafıza uyumluluğu:
- Provider çağrı doğrulaması (Codex/Claude/Gemini):
- cargo fmt / clippy / test / build sonuçları:
- Başarısız veya çalıştırılamayan testler:
- Bilinen sınırlamalar:
- V0.5 için önerilen işler:
```
