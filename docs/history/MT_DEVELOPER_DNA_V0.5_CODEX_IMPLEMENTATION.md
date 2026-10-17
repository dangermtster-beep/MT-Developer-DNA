# MT Developer DNA — V0.5
## Automated Testing, Quality Gates & Performance Engineering

**Durum:** Codex uygulama şartnamesi (henüz uygulanmış kod değildir)  
**Kapsam:** `/mt-developer-dna` ile çağrılan çok sağlayıcılı yazılım geliştirme skill/plugin sistemi.  
**Hedef:** V0.1–V0.4'ü bozmadan ölçülebilir test, kalite, güvenlik ve performans kontrolleri eklemek.

## 1. Değişmez ilkeler

- Bu bir masaüstü uygulaması değildir. Slint, Tauri, React veya yeni GUI geliştirme.
- Mevcut `dna-core`, `dna-render`, `dna-cli` ve V0.3/V0.4 modüllerini **önce keşfet**; adlarını veya API'lerini varsayarak yeniden yazma.
- Kullanıcı programlama dilini seçer; otomatik değiştirme. Rust, C#, Python, C++, Go, Java, TypeScript için genişletilebilir adapter tasarla.
- `PLAN` salt-okunur, `REVIEW` salt-okunur, `BUILD` izin verilmiş değişiklikleri yapabilir. Kalite kontrolü otomatik kod düzeltme yetkisi vermez.
- Ölçülmeyen metrik için sayı uydurma; `NOT_RUN` veya `UNAVAILABLE` kaydet.
- Kullanıcının açık onayı olmadan dış servislere kaynak kod, gizli bilgi veya rapor gönderme.
- Varsayılan olarak düşük boşta CPU/RAM: sürekli polling, daemon, gereksiz paralellik yok. Kontroller talep üzerine çalışır.

## 2. V0.5 hedefleri ve kabul kriterleri

1. **Test Engine:** Proje dilini mevcut profil/manifest üzerinden bul; uygun test komutlarını güvenli argüman listeleriyle çalıştır. Çıkış kodu, süre, stdout/stderr özeti ve araç sürümünü raporla. Timeout ve iptal desteği ekle.
2. **Quality Gates:** `PASS`, `WARN`, `FAIL`, `NOT_RUN` durumlarını açık ve deterministik kurallarla hesapla. Zorunlu kontrol `NOT_RUN` ise release sonucu `BLOCKED` olmalı; `PASS` olmamalı.
3. **Performance Analyzer:** Süre, peak RSS/working set, CPU time ve mümkünse boşta CPU örneklemesi. OS ve ölçüm yöntemini kaydet. Desteklenmeyen metrikler `UNAVAILABLE` olsun.
4. **Security Scanner:** Yerel statik kontroller ve kullanıcı izin verdiğinde kurulu ekosistem araçları. Kritik bulgu varsa gate `FAIL`; araç eksikse `NOT_RUN`.
5. **Code Quality:** Format/lint/statik analiz; salt-okunur modda otomatik formatlama veya fix çalıştırma.
6. **Build Validator:** Derleme ve gerekiyorsa paketleme doğrulaması; çıktıyı ve hataları kaydet.
7. **Regression Guard:** Önceki güvenilir baseline ile aynı koşullarda karşılaştırma; uyumsuz koşulları kıyaslanamaz olarak işaretle.
8. **Quality Reporter:** Hem Markdown hem JSON rapor üret; şema sürümü ve timestamp içersin; hassas yolları/token'ları maskele.
9. **Memory Bridge:** V0.3 `.mt-dna` yapısına uyumlu rapor ve baseline yaz; mevcut hafızayı bozmadan atomik yazım/yedekleme kullan.
10. **Provider Compatibility:** Codex/Claude Code/Gemini CLI skill ve çağırma yöntemlerini mevcut V0.2 adapter mantığına göre üret ve doğrula. `/mt-developer-dna` desteği sağlayıcıya göre değişebilir; doğrulanmamış desteği iddia etme.

## 3. Hedef komutlar

```text
/mt-developer-dna test
/mt-developer-dna quality
/mt-developer-dna performance
/mt-developer-dna security
/mt-developer-dna release-check
```

Komutlar skill içinde niyet/alt komut olarak tanımlanır. Her provider için gerçek çağırma biçimini desteklenen yeteneklerine göre eşleştir. CLI eşdeğerlerini mevcut `dna-cli` tasarımına uygun ekle; var olan komutları kırma.

## 4. Dil adapter matrisi

| Dil | Test | Format / Lint | Build | Güvenlik (opsiyonel) |
|---|---|---|---|---|
| Rust | `cargo test` | `cargo fmt --check`, `cargo clippy` | `cargo build --release` | `cargo audit` |
| C# | `dotnet test` | `dotnet format --verify-no-changes` | `dotnet build -c Release` | NuGet advisory kontrolleri |
| Python | `pytest` | `ruff check`, `ruff format --check` | `python -m compileall` (sınırlı doğrulama) | `pip-audit` |
| C++ | CTest (varsa) | `clang-format --dry-run --Werror` | CMake build (varsa) | `clang-tidy`/dependency scan |
| Go | `go test ./...` | `gofmt -l`, `go vet ./...` | `go build ./...` | `govulncheck` |
| Java | Maven/Gradle test | Projenin seçili lint aracı | Maven/Gradle build | Dependency-check vb. |
| TypeScript | Proje test script'i | ESLint/Prettier (varsa) | Proje build script'i | Paket denetimi |

**Kurallar:** Komutları proje yapılandırmasına göre keşfet. Kurulu olmayan aracı otomatik yükleme. Keyfi shell string birleştirme kullanma; komut/argüman ayrımı, izinli çalışma dizini ve timeout uygula. Python `compileall` test veya paketleme başarısı yerine geçmez. Lint araçlarının varsayılan olarak dosya değiştirmediğini doğrula.

## 5. Önerilen modüler tasarım

Mevcut workspace'i inceleyip uygun crate'e entegre et; gerekiyorsa `dna-quality` gibi yeni crate oluştur, fakat gereksiz crate çoğaltma.

- `ToolDetector`: araç yolu/sürümü ve uygunluğu
- `CheckRunner`: süreç çalıştırma, timeout, iptal, çıktı sınırı
- `LanguageQualityAdapter`: dil/proje tipine göre kontroller
- `GateEvaluator`: konfigürasyondan deterministik karar
- `PerformanceSampler`: OS'e özgü ölçümler, kapasite bildirimi
- `BaselineStore`: karşılaştırılabilir baseline okuma/yazma
- `ReportWriter`: JSON + Markdown
- `MemoryIntegration`: `.mt-dna` bağlantısı

Her modül test edilebilir arayüzlere sahip olmalı. Proje hafızası için mevcut V0.3 API'sini yeniden kullan.

## 6. Quality gate politikası

Önerilen `quality-policy.json` örneği:

```json
{
  "schema_version": "0.5",
  "required_checks": ["build", "tests", "security_critical"],
  "warnings": ["format", "lint", "performance_regression"],
  "performance": {
    "regression_warning_percent": 15,
    "minimum_repetitions": 3
  },
  "timeout_seconds": 600,
  "max_output_kb_per_check": 512
}
```

- Zorunlu kontrolde `FAIL` => release `FAIL`.
- Zorunlu kontrolde `NOT_RUN`/`UNAVAILABLE` => release `BLOCKED`.
- Yalnızca uyarı niteliğinde bulgular => `WARN`.
- Tüm zorunlu kontroller `PASS`, uyarı yok => `PASS`.
- Güvenlik taramasının kapsamını raporla; temiz tarama, güvenlik garantisi değildir.
- Proje bazlı eşikler konfigüre edilebilir; başlangıç eşikleri ölçüm sonucu gibi sunulmaz.

## 7. Performans ölçüm protokolü

1. Test edilecek program/komut ve iş yükünü açıkça kaydet.
2. Ortam: OS, mimari, CPU, RAM, derleme modu, araç sürümü.
3. Warm-up ve en az 3 tekrar; medyan süre ve peak RAM raporla.
4. CPU kullanımında toplam CPU time ile yüzde CPU kullanımını ayır; çok çekirdekli sistemlerde yüzde tanımını belirt.
5. Boşta CPU testi için programın gerçekten idle durumuna geçmesini bekle ve belirli bir pencere boyunca ölç; ölçüm desteklenmiyorsa `UNAVAILABLE`.
6. Baseline yalnızca eşdeğer iş yükü, makine ve derleme modu için karşılaştırılır.
7. Ölçüm yükü azaltmak için örnekleme aralığı ve işlem sayısını sınırla; arka planda sürekli izleme başlatma.
8. GPU ölçümü opsiyonel; destek yoksa 0 yazma.

## 8. Hafıza ve rapor dizinleri

```text
.mt-dna/
├── project.json                 # V0.3: koru
├── architecture.md              # V0.3: koru
├── decisions.md                 # V0.3: koru
├── progress.md                  # V0.3: koru
├── standards.json               # V0.3: koru
├── reports/
│   ├── quality/
│   │   ├── <timestamp>.json
│   │   └── <timestamp>.md
│   ├── performance/
│   ├── security/
│   └── testing/
└── baselines/
    └── performance.json
```

`<timestamp>` çakışmaya dayanıklı ISO tarih/saat + kısa kimlik olabilir. Raporları büyüklük/saklama politikasıyla yönet; varsayılan olarak eski raporları silme.

## 9. JSON rapor sözleşmesi (örnek)

```json
{
  "schema_version": "0.5",
  "project": "sample",
  "mode": "REVIEW",
  "timestamp_utc": "2026-10-16T18:00:00Z",
  "environment": {"os": "windows", "build": "release"},
  "checks": [
    {"id": "tests", "status": "PASS", "duration_ms": 1024, "tool": "cargo", "evidence": "test summary"},
    {"id": "security_critical", "status": "NOT_RUN", "reason": "scanner not installed"}
  ],
  "performance": {"peak_rss_bytes": null, "cpu_time_ms": null},
  "release_gate": "BLOCKED"
}
```

Bu yalnızca **şema örneğidir**, gerçek ölçüm değildir. JSON alanları için serde modelleri, şema doğrulama ve geriye dönük uyumluluk testleri oluştur.

## 10. Güvenlik ve izin modeli

- Komut enjeksiyonunu önle; kullanıcı girdisini shell komutu olarak çalıştırma.
- Proje dışına yazma için açık onay al.
- `PLAN` ve `REVIEW` modlarında dosya değiştiren komutları çalıştırma. Testlerin yan ürün yazabileceğini dikkate al; salt-okunur garantisi gerekiyorsa izolasyon/ayrı geçici kopya kullan.
- Bağımlılık indirme veya dış ağa erişim gerekiyorsa izin al.
- Token, parola, `.env` içeriği ve kişisel dosya yollarını raporlarda maskele.
- Dosya değiştiren auto-fix yalnızca `BUILD` modunda ve kullanıcı izniyle yapılır.

## 11. Test senaryoları (asgari)

- Başarılı ve başarısız test komutu.
- Eksik test/lint/security aracı => `NOT_RUN`.
- Timeout ve kullanıcı iptali.
- Boş/bozuk JSON politika dosyası.
- Zorunlu `NOT_RUN` => release `BLOCKED`.
- Kritik güvenlik bulgusu => `FAIL`.
- Baseline makine/iş yükü uyuşmazlığı => karşılaştırma yapılmaz.
- Rapor atomik yazma ve yeniden okuma.
- Hassas bilgilerin redaksiyonu.
- V0.3 hafızasıyla uyumluluk; eski kayıtların korunması.
- V0.4 PLAN/BUILD/REVIEW izin sınırları.
- Her provider adapter'ında çıktı üretimi ve dosya doğrulaması.

## 12. Codex uygulama sırası

**Aşama A — Keşif:** Workspace ve mevcut komut/skill/manifest yapısını incele. V0.1–V0.4 testlerini çalıştır; mevcut durumu kaydet.

**Aşama B — Çekirdek:** CheckRunner, tool discovery, adapter sözleşmeleri ve Quality Gate evaluator geliştir. Birim testleri yaz.

**Aşama C — Entegrasyon:** Dil adapter'ları, `dna-cli` komutları, skill yönergeleri, V0.3 memory bridge ve V0.4 mod politikaları.

**Aşama D — Ölçüm:** Windows öncelikli performans ölçümü; desteklenmeyen platformlarda açık kapasite bildirimi. Baseline kıyaslaması.

**Aşama E — Rapor:** JSON/Markdown çıktıları, redaksiyon ve doğrulama.

**Aşama F — Doğrulama:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace --release`; ortam/bağımlılık nedeniyle çalışmayanları gerekçeli raporla.

## 13. Teslim kriterleri

- Mevcut V0.1–V0.4 özellikleri bozulmamış.
- V0.5 komutları desteklenen provider çağırma yöntemlerinde tanımlı.
- En az bir gerçek proje üzerinde test/quality/release-check uçtan uca denenmiş.
- JSON + Markdown rapor üretilmiş.
- Zorunlu kontrol çalışmadığında yanlış `PASS` verilmemiş.
- Ölçüm sonuçları kanıt ve ortam bilgisiyle raporlanmış.
- README, CHANGELOG ve kullanım örnekleri güncellenmiş.
- Test komutları ve gerçek sonuçları raporlanmış; doğrulanmamış işlevler açıkça belirtilmiş.

## 14. Codex'e doğrudan görev

> Bu belgeyi MT Developer DNA V0.5 için ana teknik şartname kabul et. Önce mevcut Rust workspace'i ve V0.1–V0.4 kodlarını incele; çalışan modülleri koru. Masaüstü GUI oluşturma. Quality Gates, Test Engine, Performance Analyzer, Security Scanner, raporlama ve V0.3 proje hafızası entegrasyonunu aşamalı geliştir. Kullanıcının dil seçimine saygı göster; REVIEW/PLAN modlarında kod değiştirme. Eksik araçları `NOT_RUN` olarak raporla, ölçülmeyen performansı uydurma. Her aşamayı test et; sonunda değişiklikler, komutlar, test çıktıları, eksikler ve risklerle bir uygulama raporu ver.
