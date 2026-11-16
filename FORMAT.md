# `.dram` Dosya Formatı Spesifikasyonu (v1)

`.dram`, `draM.` senaryo yazım uygulamasının yerel proje formatıdır. Standart bir **ZIP** (Deflate sıkıştırmalı) konteyneridir ve içinde UTF-8 kodlamalı JSON dosyaları barındırır.

## Arşiv İçeriği

| Dosya | Açıklama |
| :--- | :--- |
| `manifest.json` | Format sürümü (`format_version`), belge UUID'si (`document_id`), uygulama sürümü (`app_version`), son kayıt zamanı (`saved_at`) ve ileride eklenecek gömülü medya/ek dosyaların yol listesi (`embedded_assets`). |
| `script.json` | Belge üst verisi (`metadata`), başlık sayfası (`title_page`) ve sıralı senaryo elemanları (`elements`). |
| `notes.json` | Belgeye veya elemanlara bağlı bağımsız notlar (`notes`). |
| `settings.json` | Proje ayarları: `script_language` (`"tr"` \| `"en"`), `page_size` (`"a4"` \| `"us_letter"`), `font_family`, `show_scene_numbers` ve özelleştirilebilir `dictionary`. |

## Sürüm ve Migrasyon

- Güncel sürüm: `format_version = 1`.
- Daha eski bir sürüm açıldığında `dram-format` içindeki migrasyon hattı (`migrate_payloads`) ham JSON içeriğini sırayla güncel şemaya yükseltir.
- Uygulamanın desteklediğinden daha yeni bir `format_version` içeren dosyalar `UnsupportedVersion` hatası döndürür; panic oluşmaz.

## Atomik Kayıt ve Kurtarma

- Kayıt sırasında önce aynı dizinde geçici bir dosya (`.<dosya>.<uuid>.tmp`) oluşturulur, `sync_all()` ile diske yazılır ve ardından hedef dosya üzerine atomik `rename` yapılır.
- Çökme kurtarma kopyaları `<proje>.dram.recovery` adıyla saklanır ve başarılı kayıtta temizlenir.
