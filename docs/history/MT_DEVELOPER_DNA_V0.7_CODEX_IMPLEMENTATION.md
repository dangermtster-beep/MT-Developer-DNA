# MT Developer DNA V0.7 — Universal Plugin Deployment & Synchronization Engine

**Durum:** Codex için uygulama şartnamesi (henüz uygulanmadı)  
**Hedef:** Var olan Rust workspace ve skill paketlerini bozmadan Codex, Claude Code ve Gemini CLI dağıtımı, güncellemesi, senkronizasyonu ve tanılamasını sağlamak.

## 1. Kesin kapsam

- Bu proje **masaüstü uygulaması değildir**; Rust CLI + provider uyarlayıcıları + skill/talimat dosyalarıdır.
- Mevcut `dna-core`, `dna-render`, `dna-cli` ve V0.2–V0.6 işlevlerini koru. Önce kodu ve testleri incele; mevcut API'leri doğrulamadan yeni isimler varsayma.
- Ortak kaynak tek olmalı; sağlayıcı dosyaları buradan deterministik üretilmeli.
- Kullanıcı programlama dilini seçer. Kurulum işlemi bu seçimi değiştirmez.
- Başlıca öncelikler: minimum boşta CPU, düşük RAM, sıfır sürekli polling, atomik yazma, güvenli geri alma.

## 2. Provider desteği ve gerçekçi uyumluluk

Desteklenecek hedefler: **Codex**, **Claude Code**, **Gemini CLI**.

- Her hedefin **kurulu sürümünün desteklediği** skill, komut, talimat ve dizin biçimini tespit et.
- `SKILL.md` bulunması tek başına `/mt-developer-dna` slash komutunu garanti etmez.
- Sağlayıcı gerçekten slash çağrısı sunuyorsa bunu etkinleştir; aksi durumda belgelenmiş eşdeğer çağrıyı göster.
- Resmî ve sürüme uygun dokümanlara göre adaptör yaz; doğrulanamayan yeteneği `unsupported` veya `unverified` olarak raporla.
- Küresel (global) ve proje kapsamlı (project) kurulumlar ayrı hedeflerdir; kullanıcı seçim yapar.
- Windows öncelikli, platforma özgü yollar sabit kodlanmamalı; ortam değişkenleri ve platform API'leri kullanılmalı.

## 3. Rust mimarisi

Var olan workspace düzenine uygun, gerekirse şu modülleri ekle:

```text
crates/
  dna-core/            # mevcut
  dna-render/          # mevcut
  dna-cli/             # mevcut: yeni alt komutlar
  dna-deploy/          # yeni: keşif, planlama, kurulum, geri alma
  dna-provider-check/  # opsiyonel; küçük projede dna-deploy içinde kalabilir
```

Önerilen soyutlamalar:

- `ProviderAdapter`: `detect`, `capabilities`, `resolve_target`, `render_manifest`, `validate_install`.
- `InstallPlan`: kaynak, hedef, etkilenen dosyalar, checksum, çatışma ve yedekleme planı.
- `DeploymentTransaction`: hazırlık, doğrulama, yedek, atomik uygulama, doğrulama, rollback.
- `InstallRegistry`: hedef, sürüm, sahiplik, manifest hash, tarih, yedek kimliği; JSON veya mevcut depolama katmanı ile.
- `DiagnosticResult`: `PASS | WARN | FAIL | NOT_RUN | UNSUPPORTED`.

Yüksek maliyetli arka plan servisi oluşturma. Komut çalışınca başla, bitince çık.

## 4. Komut tasarımı

Rust CLI'de aşağıdaki komutları, mevcut adlandırma ile çakışmıyorsa uygula:

```text
mt-dna providers list
mt-dna deploy plan --provider codex --scope project --path <project>
mt-dna deploy install --provider codex --scope project --path <project>
mt-dna deploy install --provider all --scope global
mt-dna deploy status
mt-dna deploy sync --dry-run
mt-dna deploy sync --apply
mt-dna deploy update --provider claude
mt-dna deploy rollback --transaction <id>
mt-dna doctor --provider all
mt-dna doctor --provider gemini --json
```

**AI komut hedefleri:** `/mt-developer-dna doctor`, `/mt-developer-dna install`, `/mt-developer-dna sync` yalnızca provider destekliyorsa sağlanır; aksi halde skill çağrısından CLI aracına yönlendirme veya açık manuel komut verilir. CLI argümanlarıyla provider slash argümanlarının aynı olacağını varsayma.

## 5. Kurulum iş akışı

1. Kurulu provider'ı ve sürümünü keşfet; `not found` ile `unsupported version` ayrımını yap.
2. Hedef scope ve yazılacak gerçek dosyaları listele.
3. Paket üret; manifest, SHA-256 ve sürüm bilgisi hesapla.
4. `dry-run` ile dosya bazında `create/update/unchanged/conflict` farkını göster.
5. Kullanıcıdan açık onay al; `--yes` yalnızca CLI'de kullanıcı açıkça verdiğinde geçerli.
6. Değişecek mevcut dosyaları yedekle; üçüncü taraf dosyalarını varsayılan olarak üzerine yazma.
7. Geçici dosyalara yaz, doğrula ve aynı dosya sistemi içinde atomik yeniden adlandırma uygula (platform sınırlarını belgele).
8. Kurulum kaydını yaz; işlem kısmen başarısızsa mümkün olan güvenli geri almayı uygula.
9. Yapısal doğrulamayı çalıştır; canlı provider çağrısını ayrı bir manuel/entegrasyon testi olarak raporla.

## 6. Senkronizasyon ve çatışma politikası

- **Tek kaynak:** V0.2–V0.6 standartları ve profiller; adaptörler yalnızca çıktı üretir.
- Aynı girdi + aynı adaptör sürümü => aynı içerik (zaman damgasını hash'e katma).
- Kurulu dosyanın hash'i kayıtlı hash'ten farklıysa `user-modified` kabul et; sessizce üzerine yazma.
- `sync --dry-run` varsayılan; `--apply` öncesi farkları göster ve onay iste.
- Kullanıcının düzenlediği dosyalar için `skip`, `backup-and-replace` veya `abort` seçenekleri sun; otomatik merge zorunlu değil.
- Silme işlemi yalnızca kurulum kayıtlarıyla sahipliği kanıtlanan dosyalarda ve açık onayla yapılabilir.

## 7. Sürüm ve uyumluluk

- SemVer kullan; paket sürümü, şema sürümü ve provider adaptör sürümünü ayrı sakla.
- Manifest örneği:

```json
{
  "schema_version": 1,
  "package": "mt-developer-dna",
  "version": "0.7.0",
  "provider": "codex",
  "scope": "project",
  "adapter_version": "0.7.0",
  "files": [{"path": "relative/path", "sha256": "<computed>"}]
}
```

- Gelecekteki şema sürümlerini sessizce dönüştürme; açık hata veya desteklenen migration planı üret.
- Downgrade için yedek ve şema uyumluluğunu kontrol et.

## 8. Doctor / Health Check

`mt-dna doctor` şu kontrolleri kapsamalı:

- CLI ve Rust motorunun kullanılabilirliği.
- Provider executable/kurulum yolu, sürüm ve desteklenen çağırma mekanizması.
- Skill/talimat dosyalarının varlığı, sözdizimi, manifest hash ve erişim izinleri.
- Paket sürümü, kaynak/çıktı eşleşmesi, yerel değişiklikler.
- Son kurulumun ve rollback yedeğinin bütünlüğü.
- V0.3 proje hafızası şemasının mevcut sürümle uyumu.
- **Canlı provider çalıştırma:** yalnızca güvenli ve mümkünse isteğe bağlı test; yapılmadıysa `NOT_RUN`.

Çıktı hem okunabilir metin hem JSON olsun. Exit code: 0 başarılı; 1 uyarı/eksik; 2 başarısızlık; mevcut CLI sözleşmesi farklıysa geriye uyumlu eşle.

## 9. Güvenlik

- Path traversal, symlink/junction kaçışı ve beklenmeyen hedef dizinlerini engelle.
- Provider ayarlarını, API anahtarlarını ve token'ları loglama veya yedeklere kopyalama.
- Hedef yolun çalışma alanı dışında olması durumunda açık onay iste.
- `--provider all` hiçbir zaman otomatik olarak kurulu olmayan sağlayıcı için dosya oluşturmasın.
- Kurulum betiklerini veya uzak içerikleri otomatik çalıştırma.
- Yedeklerde sınırlı saklama politikası uygula; silmeden önce sahiplik ve kullanıcı onayı doğrula.
- Eşzamanlı iki kurulum için lock/transaction koruması sağla.

## 10. Test matrisi

- Unit: provider algılama, sürüm parse, yol çözümleme, manifest ve hash.
- Integration: geçici dizinde kurulum, tekrar kurulum (idempotency), sync, rollback.
- Negatif: izin hatası, yarım kalan işlem, bozuk manifest, hash uyuşmazlığı, değiştirilmiş dosya, symlink saldırısı.
- Platform: Windows 10/11 ana hedef; desteklenen diğer platformlarda koşullu test.
- Provider: Codex, Claude Code, Gemini CLI için fixture tabanlı test; gerçek kurulum varsa ayrıca smoke test.
- Regression: V0.1–V0.6 komutları, proje hafızası ve kalite kapıları bozulmamalı.

Komutlar:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Araç eksikse `NOT_RUN` raporla; test sonucu uydurma.

## 11. Kabul kriterleri

- [ ] Her provider için sürüm/özellik keşfi ve destek durumu raporu.
- [ ] Dry-run çıktısı hiçbir dosyayı değiştirmiyor.
- [ ] Kurulum aynı girdiyle ikinci kez çalışınca gereksiz değişiklik yapmıyor.
- [ ] Kullanıcı değişiklikleri üzerine izinsiz yazılmıyor.
- [ ] Yedekleme ve transaction rollback testleri başarılı.
- [ ] Provider'lara uygun paketler ortak kaynaktan üretiliyor.
- [ ] Doctor PASS/WARN/FAIL/NOT_RUN/UNSUPPORTED ayrımını koruyor.
- [ ] Slash komut gerçekten desteklenmiyorsa açıkça belirtiliyor.
- [ ] V0.1–V0.6 regresyon testleri çalışıyor.
- [ ] Boşta çalışan servis/polling yok.

## 12. Codex uygulama talimatı

> MT Developer DNA V0.7'yi bu şartnameye göre uygula. Önce repository'yi, V0.1–V0.6 modüllerini ve mevcut provider adaptörlerini incele. Resmî provider dokümantasyonundan mevcut skill/komut konumlarını ve sürüm desteğini doğrula; tahminle klasör üretme. İlk aşamada mimari ve etkilenecek dosyaları raporla. Ardından Rust CLI üzerinden provider discovery, dry-run deployment, güvenli install, sync, rollback, doctor ve testleri geliştir. Masaüstü UI ekleme. Mevcut dosyaları izinsiz değiştirme. Sonunda test çıktıları, değişen dosyalar, doğrulanamayan provider özellikleri ve kalan riskleri raporla.

**Teslimatlar:** kaynak kod, provider fixture'ları, CLI yardım metinleri, örnek manifest, `V0.7_CHANGELOG.md`, `V0.7_TEST_REPORT.md`, kurulum/geri alma kullanım kılavuzu.
