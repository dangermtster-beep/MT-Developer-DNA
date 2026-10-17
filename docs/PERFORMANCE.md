# Performans politikası

Öncelikler düşük RAM tüketimi, kontrollü CPU kullanımı, gereksiz GPU kullanımından
kaçınma ve ölçülebilir sonuçlardır. Kesin kazanım ölçüm yapılmadan iddia edilmez.

## Tasarım ilkeleri

- Daemon, watcher ve sürekli polling yoktur; komutlar talep üzerine çalışır.
- Proje taraması dosya/adet/boyut sınırları ve ignore kuralları kullanır.
- Dosyalar ihtiyaç halinde okunur; provider paketleri deterministik üretilir.
- Alt süreç stdout/stderr verisi limitli toplanır ve timeout/cancellation uygulanır.
- Kontrollü paralellik kullanılır; sınırsız worker veya görev kuyruğu yoktur.
- GPU bağımlılığı yoktur.

## Ölçüm

```powershell
.\dna-cli.exe performance --project C:\Projects\Example --language rust
```

Windows sampler wall-clock, process CPU time ve peak working set ölçebilir. Test
iş yükü warm-up ve sınırlı tekrarlarla çalışır; medyan raporlanır. Idle CPU veya
GPU ölçülmüyorsa sonuç `UNAVAILABLE` olmalıdır. Farklı cihaz, araç sürümü, profil
ve iş yükleri arasında regresyon yüzdesi üretilmemelidir.

Baseline ancak kullanıcı onayıyla `.mt-dna/baselines/performance.json` altında
kaydedilir. Sonuç raporunda işletim sistemi, araç zinciri, komut, tekrar sayısı,
ölçüm yöntemi ve ölçülemeyen metrikler bulunmalıdır.

Release profili `thin` LTO, tek codegen unit ve symbol stripping kullanır. Bu
ayarların etkisi paket boyutu veya runtime benchmark'ı yapılmadan nicel kazanç
olarak sunulmaz.
