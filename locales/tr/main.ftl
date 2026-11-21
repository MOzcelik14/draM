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

tab-format = Biçim
tab-characters = Karakterler
tab-locations = Mekânlar
tab-notes = Notlar
page-x-of-y = Sayfa { $current } / { $total }
empty-list = Henüz kayıt yok.

menu-view = Görünüm
menu-format = Biçim
menu-document = Belge
menu-help = Yardım
menu-import = İçe Aktar
menu-export = Dışa Aktar
menu-renumber-scenes = Sahneleri Yeniden Numarala
menu-shortcuts = Klavye Kısayolları...
shortcuts-title = Klavye Kısayolları
shortcuts-body = Enter: Sonraki eleman türüne geç
    Tab: Mevcut eleman türünü değiştir
    Ctrl+1..8: Eleman türünü doğrudan seç
    Ctrl+S: Projeyi kaydet
    Ctrl+Z / Ctrl+Y: Geri Al / Yinele
    Ctrl+F: Bul / Değiştir
    Ctrl+P: Yazdır
    F11: Odaklanma Modu

menu-about = draM. Hakkında...
about-title = draM. Hakkında
about-version = Sürüm { $version }
about-desc = Türkçe ve İngilizce senaryolar için geliştirilmiş masaüstü senaryo yazım uygulaması.
about-suite = prisM. Suite ailesinin bir parçasıdır.
about-license = Lisans: MIT / Apache-2.0 | Yazı Tipi: Courier Prime (SIL OFL 1.1)

unsaved-title = Kaydedilmemiş Değişiklikler
unsaved-message = Belgede kaydedilmemiş değişiklikler var. Kapatmadan önce kaydetmek ister misiniz?
btn-save-and-close = Kaydet
btn-discard-close = Kaydetmeden Kapat
btn-cancel = İptal

recovery-title = Kurtarma Dosyası Bulundu
recovery-message = Beklenmedik bir kapanmadan kalan kaydedilmemiş bir oturum bulundu. Kurtarmak ister misiniz?
btn-recover = Kurtar
btn-discard-recovery = Yoksay

fmt-bold = Kalın (Ctrl+B)
fmt-italic = İtalik (Ctrl+I)
fmt-underline = Altı Çizili (Ctrl+U)

view-editor = Editör
view-beat-board = İndeks Kartları
beat-board-title = İndeks Kartları
beat-board-empty = Henüz sahne başlığı yok. Editörde bir sahne başlığı ekleyerek başlayın.
beat-board-open = Editörde Aç
beat-board-synopsis-placeholder = Sahne özeti ekle...
beat-board-move-prev = Önceye Taşı
beat-board-move-next = Sonraya Taşı

rev-mode-toggle = Revizyon Modu
rev-mode-active = Revizyon: Açık
rev-clear-all = Revizyonları Temizle

menu-reports = Senaryo Raporları...
reports-title = Senaryo Raporları
reports-summary = Toplam { $scenes } sahne · İÇ.: { $int } · DIŞ.: { $ext } · GÜN: { $day } · GECE: { $night }
reports-tab-characters = Karakterler
reports-tab-locations = Mekanlar
reports-col-name = Karakter
reports-col-lines = Replik
reports-col-words = Kelime
reports-col-scenes = Sahne
reports-col-location = Mekan
reports-col-int-ext = İÇ./DIŞ.
reports-col-day-night = GÜN/GECE
reports-export-csv = CSV Dışa Aktar...
reports-exported = Rapor CSV olarak kaydedildi.

menu-new-template = Şablondan Yeni
template-feature = Uzun Metraj Film
template-short = Kısa Film
template-series = Dizi Bölümü

menu-recent-files = Son Kullanılanlar
menu-recent-empty = (Henüz dosya yok)
menu-recent-clear = Listeyi Temizle

tp-preview-heading = Canlı Önizleme

spellcheck-toggle = Yazım Denetimi
spell-add-dict = Sözlüğe Ekle
spell-no-suggestions = (Öneri yok)

notes-attach-image = Görsel Ekle...
notes-remove-image = Görseli Kaldır
notes-preview-title = Referans Görseli
