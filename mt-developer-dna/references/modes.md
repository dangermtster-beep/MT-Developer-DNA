# PLAN, BUILD ve REVIEW sözleşmesi

## Router

Kullanıcının açık `plan`, `build` veya `review` seçimini uygula. “Tasarla” ve
“mimari öner” PLAN; “uygula”, “kodla”, “geliştir” ve “düzelt” BUILD; “incele”,
“analiz et” ve “değerlendir” REVIEW niyetidir. Niyet belirsizse salt-okunur REVIEW
seç ve kaynak değişikliği öncesinde kullanıcı onayı iste.

## PLAN

Projeyi bounded ve ignore-aware tara. `.git`, `.mt-dna`, `target`, `bin`, `obj`,
`node_modules`, sanal ortamlar ve derleme çıktıları kapsam dışıdır. Tarama dosya
ve derinlik sınırına ulaşırsa sonucu kısmi olarak işaretle.

Plan; kararlı kimlik, seçili dil, gereksinimler, açık sorular, mimari gerekçe,
riskler, bağımlı görevler ve doğrulama komutlarını içerir. Dil bilinmiyorsa uygun
alternatifler sun; kullanıcı seçmeden BUILD varsayma. PLAN varsayılan olarak
dosya yazmaz. Hafızaya kayıt istenirse önce diff göster ve açık onay al.

## BUILD

Yalnızca açık uygulama isteği veya onaylı planla çalış. Rust çekirdeği plan ve
izin sözleşmesi üretir; kaynak düzenleme, terminal komutları ve semantik kararlar
provider ajanının gerçek araçlarıyla yürütülür. Bu ayrımı gizleme.

Her doğrulamayı `passed`, `failed`, `blocked` veya `not_run` olarak kaydet.
`passed` için gerçek komut/çıktı kanıtı zorunludur. Hata halinde kapsamı sessizce
genişletme; geri alınabilir en küçük düzeltmeyi yap veya engeli raporla.

## REVIEW

Varsayılan olarak salt-okunurdur. Her bulgu şu alanlara sahip olmalıdır: önem,
dosya ve mümkünse satır, somut kanıt, etki, önerilen düzeltme. Yapısal taramayı
semantik kod incelemesi gibi gösterme; taranmayan alanları ve çalıştırılmayan
testleri açıkça belirt. Bulgu yokluğu kusursuzluk garantisi değildir.

## Project Memory aktarımı

V0.3 `.mt-dna/project.json`, `standards.json` ve `manifest.json` sözleşmesini
koru. Onaylı güncel planı `.mt-dna/plans/current.json` ve `current.md`; son review
raporunu `.mt-dna/reports/latest-review.json` ve `latest-review.md` altında yönet.
Bu dosyalar manifest hash ve nesil denetimine dahil edilir. Provider handoff;
plan kimliği, bekleyen görevler, doğrulama durumu ve engelleri gerçek halleriyle
taşır; sohbet geçmişini ortak hafıza saymaz.
