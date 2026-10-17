# V0.5 test, kalite, güvenlik ve performans sözleşmesi

## Komutlar

Provider çağrısına `test`, `quality`, `performance`, `security` veya
`release-check` niyeti eklenir. Provider slash komut desteklemiyorsa mevcut skill
çağrısının argümanı olarak aktarılır. Provider'ın yerleşik özelliği varmış gibi
gösterme.

Komutları seçili dil ve proje manifestinden keşfet. Program ve argümanları ayrı
tut; kullanıcı girdisini shell olarak çalıştırma. Çalışma dizinini proje köküne
sabitle, timeout ve iptal uygula, stdout/stderr belleğini sınırla. Eksik aracı
kurma; `NOT_RUN` kaydet.

## Gate durumları

- Zorunlu `FAIL`: release `FAIL`.
- Zorunlu `NOT_RUN` veya `UNAVAILABLE`: release `BLOCKED`.
- Yalnızca opsiyonel hata/uyarı: `WARN`.
- Tüm zorunlu kontroller kanıtlı `PASS` ve uyarı yok: `PASS`.

Her PASS gerçek çıkış kodu ve kısa kanıt taşır. Çalıştırılmayan kontrolü veya
ölçülmeyen metriği sıfırla doldurma.

## Performans

İş yükünü, OS/mimari/CPU/RAM ortamını, araç sürümünü ve ölçüm yöntemini kaydet.
Varsayılan en az üç tekrarın medyan süresini ve destekleniyorsa peak working set
ile toplam CPU time'ı raporla. Boşta CPU veya GPU ölçülmediyse `UNAVAILABLE` yaz.
Arka planda sürekli örnekleme başlatma.

Baseline yalnızca aynı komut, iş yükü, makine ve derleme koşullarında
karşılaştırılır. Koşullar farklıysa yüzde regresyon üretme.

## Güvenlik ve hafıza

Yerel taramada yalnızca sınırlı kaynak kümesini oku; `.env`, provider hafızası,
build çıktıları ve büyük dosyaları dışla. Bulguya sır değerini kopyalama. Ekosistem
scanner'ı yoksa `NOT_RUN`; temiz sonuç güvenlik garantisi değildir.

JSON ve Markdown raporlarını `.mt-dna/reports/<kategori>/`, baseline'ı
`.mt-dna/baselines/performance.json` altında mevcut manifest/hash, diff, yedek ve
atomik yazma hattıyla yönet. Açık onay olmadan yazma veya eski rapor silme.
