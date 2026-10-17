# Değişiklik günlüğü

MT Developer DNA, MT SOFTWARE tarafından geliştirilmektedir.

## 0.8.0 — 2026-10-17

- MT SOFTWARE marka, MIT lisans, iki dilli README, açık kaynak katkı/güvenlik
  belgeleri, provider kurulum rehberleri ve yeniden üretilebilir Windows paketi eklendi.
- Universal `ask_user` profili, dil öncelik çözümleyicisi ve profil CLI komutları eklendi.
- Provider registry Antigravity, Cursor, Cline ve OpenCode ile yedi adapter'a çıkarıldı.
- Provider-bağımsız hafıza aktarımı ve güvenli project/global dağıtım yolları genişletildi.
- Antigravity discovery gerçek `agy` CLI komutu, Windows IDE kurulum yolu ve
  doğrulanan `--provider-path` desteğiyle düzeltildi; IDE/CLI global skill hedefleri ayrıldı.
- Cline discovery `saoudrizwan.claude-dev` manifest doğrulamasıyla standart VS Code,
  Insiders, profil paket deposu, `VSCODE_EXTENSIONS` ve özel extension köklerini destekleyecek biçimde düzeltildi.
- OpenCode discovery Windows PATH üzerindeki native `.exe` ile npm `.cmd`/`.ps1`
  shim'lerini ve execution-policy korumalı sürüm doğrulamasını destekleyecek biçimde düzeltildi.
- Ayrıntılar `V0.8_CHANGELOG.md` ve `docs/provider-compatibility.md` dosyalarındadır.

## 0.7.0 — 2026-10-16

- `dna-deploy` crate'i ile provider discovery, project/global hedef çözümleme,
  SHA-256 paket manifesti, registry ve transaction lock eklendi.
- Dry-run plan, açık onaylı install/update/sync, kullanıcı değişikliği çatışması,
  secret yedekleme reddi, atomik yazma ve hash-korumalı rollback eklendi.
- `providers list`, `deploy plan|install|status|sync|update|rollback` ve `doctor`
  komutları eklendi; eski V0.1–V0.6 komutları korundu.
- Canonical ve provider skill paketleri V0.7 deployment güvenlik sözleşmesiyle
  genişletildi; GUI, daemon, watcher veya polling eklenmedi.

## 0.6.0 — 2026-10-16

- `dna-core` içine sürümlü optimizasyon bulgusu, risk, güven, izin, durum geçişi
  ve düzeltme işlem kaydı sözleşmeleri eklendi.
- Bounded ve salt-okunur kaynak analizi; trailing whitespace için deterministik
  düşük riskli Auto-Fix, polling/kopyalama adayları için ölçüm gerektiren öneriler eklendi.
- `optimize`, `fix` ve `optimization-report` CLI akışları; ANALYZE, PREVIEW,
  açık onaylı APPLY ve işlem-kapsamlı ROLLBACK modları eklendi.
- APPLY öncesi/sonrası V0.5 quality gate, hedef hash doğrulaması, atomik yazma,
  hazırlık manifesti, yedek hash'i ve sonraki kullanıcı değişikliğinde güvenli duruş eklendi.
- V0.3 hafızasına `.mt-dna/reports/optimization/` raporları ve
  `.mt-dna/transactions/` rollback artifact'leri entegre edildi.
- Canonical skill ve Codex/Claude/Gemini paketleri kontrollü optimizasyon
  çağrılarıyla genişletildi. GUI, daemon, watcher veya sürekli polling eklenmedi.

## 0.5.0 — 2026-10-16

- `dna-quality` crate'i; güvenli program/argüman çalıştırma, timeout, iptal ve sınırlı
  stdout/stderr toplama ile eklendi.
- Rust, C#, Python, C++, Go, Java ve TypeScript test/format/lint/build/security
  adapter matrisi eklendi; araçlar otomatik kurulmaz.
- Deterministik `PASS/WARN/FAIL/NOT_RUN/UNAVAILABLE` kontrolleri ve zorunlu eksikte
  `BLOCKED` release gate eklendi.
- Windows wall-clock, process CPU time ve peak working set ölçümü; sınırlı tekrar,
  medyan ve karşılaştırılabilir baseline modeli eklendi. Idle CPU/GPU ölçülmüyorsa
  `UNAVAILABLE` raporlanır.
- Bounded yerel secret taraması, JSON/Markdown redaksiyonu ve V0.3 manifest tabanlı
  quality/performance/security/testing raporları ile performance baseline eklendi.
- CLI ve üç provider paketi `test`, `quality`, `performance`, `security` ve
  `release-check` niyetleriyle genişletildi. GUI, daemon ve polling eklenmedi.

## 0.4.0 — 2026-10-16

- PLAN, BUILD ve REVIEW sözleşmeleri, güvenli ModeRouter, gereksinim motoru,
  mimari danışmanı ve bağımlı görev planlayıcısı eklendi.
- Bounded, deterministik ve ignore-aware proje analizi; kısmi tarama raporu,
  Rust/C# dahil dil ve build/test kanıt tespiti eklendi.
- PLAN ve REVIEW salt-okunur varsayılanları; BUILD için açık onay ve provider ajanı
  uygulama sınırı tanımlandı. Çekirdeğin yapmadığı kaynak/terminal işlemleri iddia edilmez.
- V0.3 hafıza şeması korunarak `plans/current.*` ve `reports/latest-review.*`
  manifest yönetimine alındı; güncel plan provider handoff'a eklendi.
- Provider paketleri ve canonical skill üç modu, izinleri ve hafıza aktarımını açıklayacak
  biçimde genişletildi. Masaüstü uygulaması, watcher veya polling eklenmedi.

## 0.3.0 — 2026-10-16

- Provider bağımsız, sürümlü `.mt-dna` Project Memory şeması eklendi.
- `init`, `resume`, `update`, `handoff`, `memory check` ve önizlemeli/onaylı
  `memory repair` CLI akışları eklendi.
- Alan allowlist'i, gizli bilgi redaksiyonu, `.env` dışlama, path traversal ve
  symlink/reparse koruması uygulandı.
- Manifest nesli ve içerik hash'leriyle optimistic concurrency; yazma kilidi,
  yedekleme ve atomik dosya değiştirme eklendi.
- Kaynak yapısı parmak izi, stale kayıt, bozuk link, şema, hash ve dil tutarlılığı
  denetimleri eklendi.
- Codex, Claude Code ve Gemini CLI paketleri aynı `.mt-dna` sözleşmesini okuyacak
  şekilde genişletildi; provider sohbet geçmişlerinin paylaşılmadığı açıklandı.
- Masaüstü uygulaması, watcher ve sürekli polling eklenmedi.

## 0.2.0 — 2026-10-16

- Ürün sınırı evrensel AI yazılım geliştirme skill/plugin sistemi olarak düzeltildi;
  masaüstü UI planı kaldırıldı.
- Canonical `mt-developer-dna/SKILL.md`, ortak standartlar ve yedi dil profili eklendi.
- Skill ve dil profili doğrulama API'leri eklendi.
- Provider renderer adapter yapısına geçirildi; mevcut public render API'si korundu.
- Codex `$mt-developer-dna`, Claude `/mt-developer-dna`, Gemini
  `/mt-developer-dna` çağırma mekanizmaları üretildi.
- Gemini skill ve custom-command dosyaları eklendi.
- Kurulum planı/diff, açık onay, yedek manifesti, atomik yazma, idempotency ve
  rollback eklendi.
- Salt-okunur provider sürüm raporu eklendi.

## 0.1.0 — 2026-10-16

- Rust workspace, profil doğrulama, deterministik provider renderer'ı ve güvenli
  export komutları oluşturuldu.

