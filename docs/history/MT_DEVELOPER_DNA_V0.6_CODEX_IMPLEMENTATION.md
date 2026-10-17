# MT Developer DNA V0.6 — Intelligent Code Optimization & Controlled Auto-Fix

**Durum:** Codex uygulama şartnamesi; uygulanmış veya test edilmiş sürüm değildir.  
**Ürün:** Codex / Claude Code / Gemini CLI için `/mt-developer-dna` çağrısıyla kullanılan yazılım geliştirme skill/plugin sistemi. **Masaüstü GUI oluşturulmayacak.**

## 1. Amaç ve sınırlar

V0.5 kalite kapıları, testler, güvenlik ve performans raporlarını kullanarak **kanıta dayalı düzeltme önerileri** üret; kullanıcı tarafından izin verilen düzeltmeleri güvenle uygula; yeniden test et ve geri alma imkânı sağla. Yalnızca yazılım geliştirme alanı. Kullanıcının seçtiği programlama dili korunur. Önceki V0.1–V0.5 davranışları geriye uyumlu kalır.

## 2. Mevcut kodu keşfet (zorunlu ilk adım)

- Workspace içindeki `dna-core`, `dna-render`, `dna-cli`, mevcut provider adapter'ları, SKILL.md, `.mt-dna` şeması ve V0.5 rapor sözleşmelerini incele.
- Gerçekte var olmayan fonksiyonları veya tamamlanmamış özellikleri varmış gibi varsayma. Yeni API tasarlarken mevcut tipleri yeniden kullan.
- Önce `cargo test --workspace` ve uygun mevcut kalite kontrollerini çalıştır; başlangıç durumunu raporla.

## 3. Modlar ve güvenlik politikası

- **ANALYZE** (varsayılan): Salt-okunur inceleme; bulgu, kanıt, önerilen yama ve beklenen etki. Kod değiştirmez.
- **PREVIEW**: Uygulanacak dosyalar, unified diff, risk sınıfı, geri alma planı ve test planı sunar. Kod değiştirmez.
- **APPLY**: Kullanıcı açıkça izin verdiyse kapsamı sınırlandırılmış yamayı uygular. Önce temiz/izlenebilir durum ve geri alma noktası doğrulanır. Ardından testler ve kalite kapıları çalışır.
- **ROLLBACK**: Yalnızca bu plugin'in ilgili işlemde yaptığı değişiklikleri, önceden kaydedilen değişiklik kümesi üzerinden geri alır. Kullanıcının diğer değişikliklerine dokunmaz. Çakışmada durur ve raporlar.
- `review` / `plan` çağrıları kesinlikle dosya değiştirmez; `build` yalnızca geçerli izinlerle APPLY yapabilir.
- Güvenlik açığı düzeltmeleri de otomatik olarak yetkilendirilmiş sayılmaz.

## 4. Bulgu ve düzeltme veri modeli

Her bulgu için: `finding_id`, `source` (test/lint/static-analysis/security/performance), `severity`, `confidence`, `file`, `line_range`, `evidence`, `reproduction`, `proposed_fix`, `risk`, `verification_steps`, `status`.

Her düzeltme için: `fix_id`, `finding_ids`, `target_files`, `patch_digest`, `before_hashes`, `after_hashes`, `approval_record`, `applied_at`, `tests_run`, `gate_results`, `rollback_artifact`. Hassas veri ve token saklama.

Sadece gerçekten mevcut ve doğrulanmış ölçümleri sayısal olarak raporla. Statik sezgileri kesin performans kazancı olarak sunma.

## 5. Düzeltme öncelikleri

1. Derleme ve zorunlu test hataları.
2. Doğrulanmış güvenlik kusurları ve tehlikeli bağımlılıklar.
3. Tekrarlanabilir performans regresyonları.
4. Kod kalitesi ve bakım kolaylığı.
5. İsteğe bağlı stil iyileştirmeleri.

Birden fazla bulgu aynı kök nedenden geliyorsa tek çözüm altında grupla. Kullanıcının seçtiği dil, framework ve proje mimarisini sessizce değiştirme. Büyük çaplı yeniden yazım yapma.

## 6. Düşük RAM / CPU optimizasyon politikası

- Boşta CPU tüketimi, gereksiz polling, busy-wait, yüksek frekanslı timer, gereksiz redraw ve arka plan işlerini tespit et.
- Bellek tahsisleri, gereksiz kopyalar, sınırsız cache, büyük veri yüklemeleri, kaynak sızıntıları ve eşzamanlılık sorunlarını incele.
- İyileştirmeler için **önce-sonra** ölçümleri aynı cihaz/konfigürasyon ve karşılaştırılabilir iş yükünde tekrarla; medyan ve dağılımı raporla.
- Yalnızca performansı değil, doğruluğu, yanıt süresini, enerji tüketimini ve kullanıcı deneyimini de değerlendir. CPU yüzdesini keyfî sınırlandırmayı otomatik optimizasyon sayma.
- GPU optimizasyonu yalnızca uygulama GPU kullanıyorsa ve uygun ölçüm altyapısı mevcutsa yapılır.

## 7. Dil profilleri

Rust: cargo test, clippy, fmt; C#: dotnet test ve uygun analyzers; Python: pytest, ruff, varsa tip denetimi; C++: proje derleme/test sistemi, uygun sanitizer'lar; Go: go test, go vet; Java: Maven/Gradle testleri; TypeScript: typecheck, lint, test. **Komutlar projede mevcutsa** kullan; araç eksikse `NOT_RUN` bildir. Otomatik bağımlılık kurma veya internete çıkma için izin gereksinimlerini uygula.

## 8. CLI / skill çağrıları

Hedef kullanıcı deneyimi (provider'ın desteklediği komut mekanizmasına uyarlanır):

```text
/mt-developer-dna optimize
/mt-developer-dna optimize --mode analyze
/mt-developer-dna optimize --mode preview
/mt-developer-dna optimize --mode apply
/mt-developer-dna optimize --mode rollback --fix-id <id>
/mt-developer-dna fix --finding-id <id>
/mt-developer-dna optimization-report
```

Slash sözdiziminin her sağlayıcıda aynı şekilde çalışacağını varsayma. Provider destek matrisinde gerçek tetikleme biçimini belirt. Komutların parametre ayrıştırılması güvenli olmalı; keyfî shell komutu çalıştırma.

## 9. Mimari entegrasyon

- `dna-core`: bulgu/düzeltme tipleri, risk sınıflandırması, izin ve durum makinesi.
- `dna-cli`: analiz/önizleme/uygulama/geri alma giriş noktaları.
- `dna-render`: Codex, Claude Code, Gemini CLI için eşdeğer skill/komut dosyaları.
- V0.3 proje hafızası: `.mt-dna/reports/optimization/` altında JSON + Markdown raporları ve işlem kaydı; mevcut şema/versiyonlama kurallarını koru.
- V0.5 quality gate: uygulama öncesi ve sonrası zorunlu kontroller; FAIL/NOT_RUN ayrımı. Zorunlu gate başarısızsa başarılı teslim raporu verme.
- Git varsa mevcut branch ve çalışma ağacını denetle; kullanıcı değişikliklerini asla `reset --hard`, `clean -fd` veya toplu restore ile silme. Git yoksa kapsamlı, hash doğrulamalı dosya yedekleri kullan.

## 10. Atomiklik ve hata durumları

- Önce dosya listesi ve hash'leri kaydet; izinli dosyaları güvenli geçici konuma yedekle.
- Yama öncesi hash doğrulaması yap; uyuşmazlıkta işlemi durdur.
- Kısmi uygulama veya test hatasında kullanıcıya açık durum raporu sun; otomatik geri alma yalnızca kullanıcı değişikliklerine dokunmayacağı kesin ise yapılabilir.
- İşlem günlüğünü yaz; yarım kalmış işlemler için güvenli kurtarma mekanizması tasarla.
- Symlink/path traversal ve workspace dışına çıkışları engelle.

## 11. Testler ve kabul kriterleri

- ANALYZE/PREVIEW modlarında proje dosyalarının değişmediğini test et.
- Onaysız APPLY'nin reddedildiğini test et.
- Hash uyuşmazlığı ve kullanıcı değişikliği durumunda güvenli duruşu test et.
- Bir başarılı küçük yama + yeniden test + kayıt + geri alma uçtan uca senaryosu oluştur.
- Test başarısızlığı, eksik araç, izin reddi, kısmi yazma, çakışma ve bozuk rapor senaryolarını kapsa.
- Önceki V0.1–V0.5 regresyon testleri geçmeli.
- Codex, Claude Code ve Gemini CLI paketlerini şema/klasör açısından doğrula; gerçek çağırma testini yapamadıysan açıkça belirt.
- Release derlemesini doğrula: `cargo build --workspace --release`.

## 12. Teslimatlar

1. Mevcut workspace'e entegre çalışan V0.6 kaynak kodu.
2. Güncellenmiş `SKILL.md` ve provider paketleri.
3. Dil bağımsız optimizasyon iş akışı, risk ve izin politikaları.
4. Örnek optimize raporu (`.md` ve `.json`), gerçek test çıktıları.
5. `CHANGELOG` ve sürüm geçiş notları.
6. Test, derleme ve provider doğrulama sonuçları; çalıştırılmayan kontroller açıkça listelensin.

## 13. Codex'e doğrudan görev

> MT Developer DNA V0.6'yı bu şartnameye göre uygula. Önce V0.1–V0.5 kodunu ve gerçek arayüzleri incele; masaüstü UI oluşturma. Önce ANALYZE ve PREVIEW modlarını tamamla, test et. Ardından güvenli, açık izinli APPLY ve kapsamlı ROLLBACK geliştir. V0.3 hafıza ve V0.5 kalite kapılarıyla entegre et. Mevcut kodu gereksiz yeniden yazma; kullanıcı değişikliklerini koru. Tüm testleri ve release derlemesini çalıştır; sonuçları ve eksikleri dürüstçe raporla.
