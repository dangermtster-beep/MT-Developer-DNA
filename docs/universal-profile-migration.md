# Universal Profile Geçiş Planı

V0.8 geriye dönük uyumludur: mevcut dil profilleri ve V0.3 `.mt-dna` hafıza
şeması değiştirilmez. Geçiş otomatik olarak global provider dosyalarına yazmaz.

1. `profiles/universal.json` dosyasını `profiles validate` ile doğrula.
2. Dil seçilmemişse profil `ask_user` durumunda kalır; Rust varsayılan yapılmaz.
3. Öncelik sırası: açık kullanıcı isteği, proje hafızası/kararları, universal
   varsayılan, provider fallback. Somut dil yoksa kullanıcıya sorulur.
4. `deploy plan` ile project/global hedef ve create/update/conflict farkını incele.
5. Yalnız seçilmiş provider için `deploy install ... --yes` uygula. `provider all`
   sadece gerçekten algılanmış CLI veya doğrulanmış masaüstü kurulumlarına uygulanır.
6. `doctor` ve provider içindeki gerçek çağrıyı ayrı ayrı doğrula. Dosya/hash PASS
   sonucu canlı `/mt-developer-dna` kabul testi değildir.

Rollback registry'deki transaction kapsamıyla sınırlıdır. Kullanıcı tarafından
değiştirilmiş dosyada durur; toplu silme veya sessiz üzerine yazma yapmaz.

