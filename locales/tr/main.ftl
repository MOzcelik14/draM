# draM. Türkçe Arayüz Çevirileri

app-title = draM. — Senaryo Editörü
untitled-document = Adsız Senaryo

menu-file = Dosya
menu-new = Yeni
menu-open = Aç...
menu-save = Kaydet
menu-save-as = Farklı Kaydet...
menu-import-fountain = Fountain İçe Aktar...
menu-export-fountain = Fountain Dışa Aktar...
menu-export-pdf = PDF Dışa Aktar...
menu-title-page = Başlık Sayfası...
menu-edit = Düzen
menu-undo = Geri Al
menu-redo = Yinele
menu-settings = Ayarlar

ui-language-label = Arayüz Dili:
ui-language-tr = Türkçe
ui-language-en = English

script-language-label = Senaryo Dili:
script-language-tr = Türkçe (İÇ. / DIŞ.)
script-language-en = İngilizce (INT. / EXT.)

page-size-label = Sayfa Boyutu:
page-size-a4 = A4 (210 × 297 mm)
page-size-us-letter = US Letter (8.5 × 11 in)
show-scene-numbers = Sahne Numaralarını Göster

title-page-heading = Başlık Sayfası
tp-title = Başlık
tp-credit = Yazan Etiketi
tp-author = Yazar
tp-source = Kaynak
tp-draft-date = Taslak Tarihi
tp-contact = İletişim
tp-copyright = Telif Hakkı
btn-close = Kapat

toolbar-element-type = Eleman Türü:
elem-scene-heading = Sahne Başlığı
elem-action = Aksiyon
elem-character = Karakter
elem-parenthetical = Parantez İçi
elem-dialogue = Diyalog
elem-transition = Geçiş
elem-shot = Çekim
elem-general = Genel
elem-centered = Ortalanmış
elem-lyrics = Şarkı Sözü
elem-page-break = Sayfa Sonu
elem-section = Bölüm
elem-synopsis = Özet
elem-note = Not

status-scenes-count = { $count ->
    [one] { $count } sahne
   *[other] { $count } sahne
}
status-elements-count = { $count ->
    [one] { $count } satır
   *[other] { $count } satır
}
status-pages-runtime = { $pages ->
    [one] { $pages } sayfa (~{ $minutes } dk)
   *[other] { $pages } sayfa (~{ $minutes } dk)
}
status-saved = Kaydedildi
status-modified = Değiştirildi
status-hint-flow = Enter: Sonraki eleman | Tab: Tür değiştir | Ctrl+1..8: Hızlı tür | Ctrl+S: Kaydet

error-open-failed = Dosya açılamadı: { $reason }
error-save-failed = Dosya kaydedilemedi: { $reason }
error-pdf-failed = PDF oluşturulamadı: { $reason }

menu-import-fadein = Fade In İçe Aktar...
menu-export-fadein = Fade In Dışa Aktar...
menu-print = Yazdır...
menu-find-replace = Bul / Değiştir...

nav-heading = Sahne Gezgini
nav-toggle = Gezgin
nav-move-up = Yukarı
nav-move-down = Aşağı

print-dialog-title = Yazdır
print-printer = Yazıcı:
print-default-printer = Sistem Varsayılanı
print-copies = Kopya Sayısı:
print-page-range = Sayfa Aralığı (örn. 1-5):
print-btn = Yazdır
print-open-viewer = PDF'i Varsayılan Görüntüleyicide Aç
print-success = Yazdırma işi gönderildi.

find-label = Bul:
replace-label = Değiştir:
find-case-sensitive = Büyük/Küçük Harf Duyarlı
find-next-btn = Sonrakini Bul
replace-btn = Değiştir
replace-all-btn = Tümünü Değiştir
find-no-match = Eşleşme bulunamadı.

autocomplete-suggestions = Öneriler:

menu-import-fdx = Final Draft (.fdx) İçe Aktar...
menu-export-fdx = Final Draft (.fdx) Dışa Aktar...

theme-label = Tema:
theme-dark = Koyu
theme-light = Açık
focus-mode-toggle = Odaklanma Modu (F11)

notes-heading = Notlar
notes-toggle = Notlar
notes-add = + Not Ekle
notes-delete = Sil
notes-title-placeholder = Not başlığı
notes-body-placeholder = Not içeriği...

dual-dialogue-btn = İkili Diyalog
dual-left-badge = [SOL İKİLİ]
dual-right-badge = [SAĞ İKİLİ]

revision-label = Revizyon:
rev-none = Yok
rev-blue = Mavi
rev-pink = Pembe
rev-yellow = Sarı
rev-green = Yeşil
