# V0.7 kurulum, senkronizasyon ve geri alma

`deploy plan` ve `deploy sync` varsayılan olarak salt-okunurdur. Profil kullanıcının
dil seçimini taşır; deployment bu seçimi değiştirmez.

Project kurulum için `--scope project --path <proje>`, kullanıcı geneli için
`--scope global` kullanın. Global hedef platformun kullanıcı ana dizininden çözülür.
`--provider all` yalnız doğrulanmış CLI veya masaüstü kurulum keşfinde bulunan
provider'ları işler. Antigravity için `agy` CLI ve Windows kullanıcı-scope IDE
kurulumu ayrı ayrı tanınır. Farklı bir konum tek provider ile
`--provider-path <executable-veya-klasör>` verilerek doğrulanabilir.

Cline bağımsız CLI olarak aranmaz. `saoudrizwan.claude-dev` extension manifesti;
standart VS Code, VS Code Insiders, `VSCODE_EXTENSIONS` ve açıkça seçilen extension
köklerinde doğrulanır. Fiziksel paket discovery sonucu `PASS` olsa bile aktif
profilde etkinlik ve canlı `/mt-developer-dna` çağrısı ayrıca doğrulanmadıysa
`NOT_RUN` olarak kalır.

OpenCode npm kurulumunda Rust'ın doğrudan process çözümlemesine güvenilmez;
PATH dizinlerinde `opencode.exe`, `opencode.cmd` ve `opencode.ps1` aranır.
PowerShell execution policy değiştirilmez veya bypass edilmez. `.ps1` probe'u
engellenirse doğrulanmış `.cmd`/`.exe` sürümü kullanılır; hiçbir runner sürüm
veremezse kurulum varlığı ile sürüm doğrulaması ayrı raporlanır.

Install, update ve uygulanan sync için `--yes` zorunludur. Çatışma varsayılan olarak
`abort` olur; isteğe bağlı `--conflict skip` veya açıkça onaylanan
`--conflict backup-and-replace` kullanılabilir. Olası gizli bilgi içeren dosya
yedeklenmez ve işlem durur.

Başarılı mutasyon transaction kimliği verir. `deploy rollback --transaction <id>
--yes` yalnız o işlemde sahipliği kaydedilmiş dosyalara dokunur; dosya sonradan
değişmişse güvenli biçimde durur. Yedekler otomatik silinmez.

`doctor` provider kurulum türü/executable/sürümünü, belgelenmiş çağırma biçimini, registry/hash
durumunu ve proje hafızasının varlığını denetler. Canlı interaktif çağrı gerçekten
çalıştırılmadıysa `NOT_RUN` kalır.
