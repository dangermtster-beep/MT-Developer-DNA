# Yazılım geliştirme standartları

## Tasarım

- Önce mevcut mimariyi ve sözleşmeleri anla; çalışan kodu gereksiz yere yeniden yazma.
- KISS ve DRY ilkelerini uygula. Tekrarı kaldırmak yeni soyutlama maliyetinden gerçekten
  daha değerliyse ortaklaştır.
- Sorumlulukları ayır, bağımlılık yönünü açık tut ve modülleri dar arayüzlerle bağla.
- SOLID ve Clean Architecture'ı bağlama göre uygula; katman veya interface sayısını
  hedef olarak kullanma.
- Geriye dönük uyumluluğu ve veri/sürüm geçişini açıkça değerlendir.

## Performans

- Olay tabanlı akışı tercih et; gereksiz polling ve busy-loop kullanma.
- Kaynakları ihtiyaç anında yükle, kullanım bitince deterministik olarak serbest bırak.
- Paralelliği sınırlı ve ölçülmüş darboğazlara yönelik kullan.
- Dosya ve veritabanı erişimini toplu, akışlı veya önbellekli yapmanın maliyetini ölç.
- GPU bağımlılığını yalnızca ölçülmüş fayda ve açık gereksinim varsa ekle.
- RAM, boşta CPU, gecikme ve throughput değerlerini ölçmeden garanti olarak sunma.

## Güvenlik ve güvenilirlik

- API anahtarlarını, tokenları, kişisel verileri ve gizli içerikleri kaynak koda veya
  loglara yazma.
- Girdi, dosya yolu ve yetki sınırlarını doğrula; path traversal ve symlink/reparse
  kaçışlarını engelle.
- Mevcut dosyaları açık yetki olmadan değiştirme veya silme. Değişiklik öncesinde
  diff göster; gerekiyorsa yedek ve rollback sağla.
- Hataları yutma. Kullanıcıya uygulanabilir mesaj ver, teknik ayrıntıyı güvenli logla.
- Bağımlılıkları gerekçeli, sürümlü ve mümkün olduğunca küçük tut.

## Kalite ve teslimat

- Birim, entegrasyon ve kabul testlerini riskle orantılı seç.
- Gerçek ortam entegrasyonunu yalnızca gerçekten çalıştırıldıysa doğrulanmış say.
- Her aşamada ilgili testleri çalıştır; başarısız veya çalıştırılamayan kontrolü açıkla.
- Kullanım, mimari, yapılandırma ve release değişikliklerini belgele.
- Semantik sürümleme kullan; kırıcı değişikliği açıkça işaretle.

