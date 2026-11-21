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
- Çökme kurtarma kopyaları `<proje>.dram.recovery` adıyla saklanır ve başarılı kayıtta veya temiz kapanışta temizlenir.

## Dosya Şemaları ve Örnekler

### 1. `manifest.json`

| Alan | Tip | Açıklama |
| :--- | :--- | :--- |
| `format_version` | `u32` | Konteyner şema sürümü (güncel: `1`). |
| `document_id` | `UUID` | Belgenin benzersiz kimliği. |
| `app_version` | `string` | Arşivi oluşturan `draM.` uygulama sürümü. |
| `saved_at` | `RFC 3339 string` | Son kayıt zaman damgası (UTC). |
| `embedded_assets` | `string[]` | İleride eklenecek gömülü medya/ek dosya yolları. |

```json
{
  "format_version": 1,
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "app_version": "1.0.0",
  "saved_at": "2026-11-16T12:00:00Z",
  "embedded_assets": []
}
```

### 2. `script.json`

| Alan | Tip | Açıklama |
| :--- | :--- | :--- |
| `metadata` | `object` | `created_at` ve `updated_at` UTC zaman damgaları. |
| `title_page` | `object` | `title`, `credit`, `author`, `source`, `draft_date`, `contact`, `copyright`. |
| `elements` | `Element[]` | Sıralı senaryo elemanları; her elemanda `id`, `kind`, `runs` (`text` + `style`), isteğe bağlı `scene_number`, `revision` ve `dual_dialogue` bulunur. |

```json
{
  "metadata": {
    "created_at": "2026-11-16T10:00:00Z",
    "updated_at": "2026-11-16T12:00:00Z"
  },
  "title_page": {
    "title": "GECE VARDİYASI",
    "credit": "Yazan",
    "author": "Örnek Yazar",
    "source": "",
    "draft_date": "16 Kasım 2026",
    "contact": "",
    "copyright": ""
  },
  "elements": [
    {
      "id": "a1b2c3d4-0000-4000-8000-000000000001",
      "kind": "scene_heading",
      "runs": [
        {
          "text": "İÇ. İSTASYON - GECE",
          "style": { "bold": true, "italic": false, "underline": false }
        }
      ],
      "scene_number": { "value": "1", "locked": false },
      "revision": null,
      "dual_dialogue": null
    },
    {
      "id": "a1b2c3d4-0000-4000-8000-000000000002",
      "kind": "action",
      "runs": [
        {
          "text": "Yağmur camlara vurur. Peron boştur.",
          "style": { "bold": false, "italic": false, "underline": false }
        }
      ],
      "scene_number": null,
      "revision": null,
      "dual_dialogue": null
    }
  ]
}
```

### 3. `notes.json`

| Alan | Tip | Açıklama |
| :--- | :--- | :--- |
| `notes` | `Note[]` | Belgeye veya belirli bir `element_id`'ye bağlı bağımsız yazar notları. |

```json
{
  "notes": [
    {
      "id": "b2c3d4e5-0000-4000-8000-000000000010",
      "element_id": "a1b2c3d4-0000-4000-8000-000000000001",
      "title": "Işık Notu",
      "body": "Soğuk floresan tonu vurgulanacak.",
      "created_at": "2026-11-16T11:30:00Z"
    }
  ]
}
```

### 4. `settings.json`

| Alan | Tip | Açıklama |
| :--- | :--- | :--- |
| `script_language` | `"tr"` \| `"en"` | Senaryo dili ve büyük/küçük harf kuralı (`İ`/`ı`, `i`/`I`). |
| `page_size` | `"a4"` \| `"us_letter"` | Sayfalama ve PDF çıktı boyutu. |
| `font_family` | `string` | Varsayılan yazı tipi ailesi (`"Courier Prime"`). |
| `show_scene_numbers` | `bool` | Sahne numaralarının editör ve PDF'te gösterimi. |
| `dictionary` | `object` | Otomatik tamamlama için `scene_prefixes` ve `transitions` listeleri. |

```json
{
  "script_language": "tr",
  "page_size": "a4",
  "font_family": "Courier Prime",
  "show_scene_numbers": false,
  "dictionary": {
    "scene_prefixes": ["İÇ.", "DIŞ.", "İÇ./DIŞ."],
    "transitions": ["KESME:", "KARARMA:", "AÇILMA:"]
  }
}
```

