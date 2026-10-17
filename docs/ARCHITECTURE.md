# Mimari

MT Developer DNA bir Rust workspace ve Agent Skill paketidir; masaüstü uygulaması,
web UI, daemon, watcher veya sürekli çalışan servis değildir.

## Crate sınırları

```text
dna-cli
  |-- dna-core
  |-- dna-memory  --> dna-core
  |-- dna-quality --> dna-core
  |-- dna-render  --> dna-core
  `-- dna-deploy  --> dna-render
```

- `dna-core`: I/O'dan bağımsız profil, dil, workflow, plan/review ve optimizasyon veri sözleşmeleri.
- `dna-render`: Ortak profilden deterministik provider paketleri ve raporlar.
- `dna-memory`: `.mt-dna` allowlist'i, sınırlı tarama, redaksiyon, hash, kilit, yedek ve atomik yazma.
- `dna-quality`: Program/argüman ayrımı, timeout, çıktı limiti, kalite kapıları ve Windows process ölçümü.
- `dna-deploy`: Discovery, scope çözümleme, SHA-256 manifest, registry, transaction, backup, sync, Doctor ve rollback.
- `dna-cli`: Kullanıcıya dönük Türkçe orchestration ve onay sınırı.

## Güven akışları

```text
provider discovery -> render -> dry-run plan
                  -> explicit approval -> lock -> backup -> atomic apply
                  -> registry/hash validation -> scoped rollback
```

```text
bounded scan -> PLAN/REVIEW or memory preview
            -> explicit approval -> stale/hash check -> atomic memory write
```

Symlink/reparse ve path traversal hedefleri reddedilir. Registry dışı veya değişmiş
provider dosyası otomatik birleştirilmez. Rollback yalnız kendi transaction
kaydındaki dosyalara dokunur ve sonraki kullanıcı değişikliğinde durur.

## Project Memory

Tek ortak kaynak proje kökündeki `.mt-dna/` klasörüdür. `project.json`, kararlar,
ilerleme, handoff, plan/review/quality raporları ve manifest hash'leri burada
yönetilir. Provider sohbet geçmişleri paylaşılmış kabul edilmez. `.env`, token,
anahtar ve şüpheli sırlar kaydedilmez.

## Genişleme

Yeni dil, doğrulanan bir JSON `LanguageProfile` eklenerek; yeni provider,
`ProviderDescriptor` ve renderer/deployment sözleşmesi uygulanarak eklenir. Mevcut
public API ve crate bağımlılık yönü gereksiz yere değiştirilmez.

## Kaynak kullanımı

Normal kullanım kısa ömürlü CLI işlemleridir. Tarama bounded ve ignore-aware,
provider rendering deterministik, quality process çıktıları sınırlıdır. Boşta
çalışan process olmadığı için uygulamaya ait sürekli idle CPU/RAM yüzeyi yoktur;
gerçek işlem tüketimi yalnız ölçümle raporlanır.
