# Güncelleme stratejisi

MT Developer DNA SemVer kullanır:

- `v0.8.0`: ilk public release.
- `v0.8.1`: geriye uyumlu hata düzeltmesi.
- `v0.9.0`: geriye uyumlu yeni özellikler.
- `v1.0.0`: stable API/release sınırı.

## GitHub Releases akışı

Canonical repository:
https://github.com/dangermtster-beep/MT-Developer-DNA

Kullanıcı Releases sayfasından mevcut sürümü ve asset SHA-256 değerini kontrol
eder, paketi kendisi indirir ve dry-run planını inceler. MT Developer DNA kullanıcı onayı
olmadan GitHub'a bağlanıp indirme, kurulum veya provider dosyası güncellemesi yapmaz.

```powershell
.\dna-cli.exe --version
.\dna-cli.exe deploy sync --profile .\profiles\universal.json `
  --provider all --scope project --path C:\Projects\Example
```

`deploy sync` yalnız yerel canonical paket ile kurulu provider çıktısını karşılaştırır
ve varsayılan dry-run'dır. Uygulamak için `--apply --yes` gerekir. Conflict dosyaları
sessizce ezilmez; transaction kimliği ve registry hash'leri rollback sınırını korur.

## Maintainer release kontrolü

1. Sürümü Cargo workspace, changelog ve belgelerde güncelleyin.
2. Format, clippy, tüm testler ve release build çalıştırın.
3. Universal Profile ile provider render/dry-run smoke testlerini çalıştırın.
4. Gizli bilgi ve mutlak kullanıcı yolu taraması yapın.
5. Windows paketini yeniden üretin ve SHA-256 kaydedin.
6. Release Notes'u doğrulama sonuçları ve `NOT_RUN` kontrollerle yayımlayın.

Release yayınlamak ayrı ve açık bir maintainer işlemidir; paket hazırlama otomatik
olarak push veya GitHub Release oluşturmaz.
