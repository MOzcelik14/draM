<div align="center">
  <img src="assets/logo-mark.svg" alt="draM. logosu" width="84" height="84">
  <h1>draM.</h1>
  <p><strong>Türkçe ve İngilizce senaryolar için saf Rust ile geliştirilmiş masaüstü senaryo yazım uygulaması.</strong></p>
  <p><em><a href="https://github.com/MOzcelik14/prism-suite">prisM. Suite</a> ailesinin bir parçasıdır.</em></p>

  <p>
    <a href="https://github.com/MOzcelik14/draM/releases/latest">
      <img src="https://img.shields.io/github/v/release/MOzcelik14/draM?style=flat-square&color=A31D1D&label=s%C3%BCr%C3%BCm" alt="Son Sürüm">
    </a>
    <a href="https://github.com/MOzcelik14/draM/actions/workflows/ci.yml">
      <img src="https://img.shields.io/github/actions/workflow/status/MOzcelik14/draM/ci.yml?style=flat-square&label=CI" alt="CI Durumu">
    </a>
    <a href="https://github.com/MOzcelik14/draM/actions/workflows/release.yml">
      <img src="https://img.shields.io/github/actions/workflow/status/MOzcelik14/draM/release.yml?style=flat-square&label=Release" alt="Release Durumu">
    </a>
    <a href="LICENSE-MIT">
      <img src="https://img.shields.io/badge/lisans-MIT%20%2F%20Apache--2.0-3B82F6?style=flat-square" alt="Lisans: MIT / Apache-2.0">
    </a>
    <img src="https://img.shields.io/badge/Rust-2021%20%7C%20100%25%20Safe-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021 - 100% Safe">
    <a href="https://github.com/MOzcelik14/prism-suite">
      <img src="https://img.shields.io/badge/prisM.-Suite-18191C?style=flat-square" alt="prisM. Suite">
    </a>
  </p>

  <p>
    <a href="https://mozcelik14.github.io/draM/">Web Sitesi</a>
    &nbsp;&middot;&nbsp;
    <a href="https://github.com/MOzcelik14/draM/releases/latest">İndir</a>
    &nbsp;&middot;&nbsp;
    <a href="CHANGELOG.md">Değişiklik Günlüğü</a>
    &nbsp;&middot;&nbsp;
    <a href="FORMAT.md">.dram Formatı</a>
    &nbsp;&middot;&nbsp;
    <a href="ROADMAP.md">Yol Haritası</a>
  </p>
</div>

---

## Genel Bakış

**draM.**, senaristler, yönetmenler ve yapım ekipleri için tasarlanmış; sıfır `unsafe` kod (`#![forbid(unsafe_code)]`) ile yazılmış, hızlı ve dikkat dağıtmayan bir masaüstü senaryo editörüdür.

Türkçe (`İ/i`, `I/ı`, `İÇ.`, `DIŞ.`, `(DEVAM)`) ve İngilizce (`INT.`, `EXT.`, `(MORE)`, `(CONT'D)`) senaryo kurallarını çekirdek motor seviyesinde yerel olarak destekler. Endüstri standardı sayfalama, gömülü Courier Prime ile baskıya hazır PDF üretimi, ikili diyalog, indeks kartları, revizyon takibi, senaryo raporları ve yazım denetimini tek bir hafif ikili dosyada birleştirir.

---

## Öne Çıkan Özellikler

### Senaryo Editörü ve Sayfalama
- **Final Draft Tarzı Akış:** Sahne başlığı, aksiyon, karakter, parantez içi, diyalog, ikili diyalog, geçiş ve çekim elemanları arasında `Enter`, `Tab` ve `Ctrl+1..8` ile kesintisiz yazım.
- **Editörde Zengin Metin ve Seçili Metne Stil:** Gömülü Courier Prime Regular, Bold, Italic ve BoldItalic yazı tipleriyle `Ctrl+B` / `Ctrl+I` / `Ctrl+U` stilleri doğrudan editör kağıdında görsel olarak işlenir; seçili kelime aralığına veya tüm elemana uygulanabilir.
- **İkili Diyalog (`Dual Dialogue`):** Aynı gruptaki sol ve sağ karakter diyalogları yan yana iki sütun olarak sayfalanır ve PDF çıktısında hizalı şekilde basılır.
- **Canlı Sayfa Sonu Ayraçları ve Sanallaştırma:** Önbelleğe alınmış sayfalama motoru her sayfa geçişini kağıt üzerinde kesikli ayraç ve sayfa numarasıyla gösterir; 150+ sayfalık uzun senaryolarda pencereleme ile yüksek akıcılık korunur.
- **Satır İçi Otomatik Tamamlama:** Karakter, sahne başlığı ve geçiş önerileri Türkçe alfabe sırasıyla hem üst öneri şeridinde hem de odaklı satırın altında açılır listede sunulur.

### Türkçe ve İngilizce Dil Desteği
- **Bağımsız Arayüz ve Senaryo Dili:** Türkçe arayüzle İngilizce senaryo veya İngilizce arayüzle Türkçe senaryo yazımı.
- **Saf Rust Yazım Denetimi (`Spellcheck`):** Türkçe ek soyucu, belgedeki karakter ve mekân adlarını otomatik tanıma, kalıcı kullanıcı sözlüğü (`user_dict.txt`), kırmızı alt çizgi vurgusu ve sağ tık düzeltme önerileri.
- **Linux X11/IBus Türkçe IME Uyumu:** `ü`, `ş`, `ğ`, `ı`, `İ`, `ö`, `ç` karakterleri tüm platformlarda kayıpsız işlenir.

### Yapım ve Organizasyon Araçları
- **İndeks Kartları (Beat Board):** Sahneleri çok sütunlu kart ızgarasında görüntüleme, doğrudan kart üzerinden `Synopsis` düzenleme, sahne sırasını değiştirme ve tek tıkla editöre atlama.
- **Revizyon Modu ve PDF Kenar Yıldızı (`*`):** Revizyon Modu aktifken değiştirilen veya eklenen satırlar seçili renkle (mavi, pembe, sarı, yeşil) otomatik işaretlenir ve PDF çıktıda sağ kenara `*` basılır.
- **Senaryo Raporları (Breakdown) ve CSV Dışa Aktarım:** Karakter replik/kelime/sahne dağılımı ile mekân (`İÇ.`/`DIŞ.`, `GÜN`/`GECE`) istatistikleri ve tek tıkla CSV aktarımı.
- **Gömülü Referans Görselleri:** Notlar sekmesine PNG/JPEG referans görselleri ekleme, küçük önizleme, tam boy görüntüleme ve görselleri doğrudan `.dram` ZIP arşivine gömme.
- **Proje Şablonları ve Başlık Sayfası Canlı Önizlemesi:** Uzun Metraj Film, Kısa Film ve Dizi Bölümü şablonları ile anlık güncellenen başlık sayfası önizlemesi.
- **Atomik Kayıt, Otomatik Kayıt ve Çökme Kurtarma:** 15 saniyede bir kurtarma kopyası, açılışta *Kurtar / Yoksay* penceresi ve kaydedilmemiş değişikliklerle kapanış uyarısı.

---

## Desteklenen Formatlar

| Format | Uzantı | İçe Aktarma | Dışa Aktarma | Açıklama |
| :--- | :--- | :---: | :---: | :--- |
| **draM. Proje** | `.dram` | Evet | Evet | Sürümlü ZIP + JSON arşivi, gömülü referans görselleri ([FORMAT.md](FORMAT.md)) |
| **Fountain** | `.fountain` | Evet | Evet | Düz metin senaryo standardı (ikili diyalog ve satır içi stiller dahil) |
| **Final Draft** | `.fdx` | Evet | Evet | XML tabanlı endüstri standardı değişim formatı |
| **Fade In** | `.fadein` | Evet | Evet | Fade In proje arşivi uyumluluğu |
| **PDF** | `.pdf` | — | Evet | Saf Rust `krilla` + gömülü Courier Prime (A4 / US Letter, başlık sayfası, `*` revizyon işareti) |
| **Rapor (CSV)** | `.csv` | — | Evet | Karakter ve mekân dağılım raporu |

---

## Klavye Kısayolları

| Kısayol | İşlev |
| :--- | :--- |
| `Enter` | Senaryo akışına göre bir sonraki eleman türünü ekler |
| `Tab` | Bulunulan satırın eleman türünü değiştirir |
| `Ctrl+1` – `Ctrl+8` | Eleman türünü doğrudan seçer |
| `Ctrl+B` / `Ctrl+I` / `Ctrl+U` | Seçili metne veya elemana kalın / italik / altı çizili uygular |
| `Ctrl+Space` | Satır içi otomatik tamamlama önerisini onaylar |
| `Ctrl+S` | Projeyi kaydeder |
| `Ctrl+Z` / `Ctrl+Y` | Son işlemi geri alır / yineler |
| `Ctrl+F` | Bul ve Değiştir penceresini açar |
| `Ctrl+P` | Yazdırma penceresini açar |
| `F11` | Odaklanma modunu açar veya kapatır |

---

## Çalışma Alanı Mimarisi

| Crate | Rol |
| :--- | :--- |
| [`dram-core`](crates/dram-core) | Belge modeli, geri alma/yineleme, sayfalama motoru (ikili diyalog dahil), Türkçe metin yardımcıları, yazım denetimi, şablonlar ve senaryo raporları |
| [`dram-format`](crates/dram-format) | `.dram` ZIP arşiv okuyucu/yazıcı, gömülü medya, çökme kurtarma, Fountain, FDX ve Fade In dönüştürücüleri |
| [`dram-pdf`](crates/dram-pdf) | `krilla` ve gömülü Courier Prime ile saf Rust PDF üreticisi |
| [`dram-print`](crates/dram-print) | Platforma özgü (Linux/macOS/Windows) yazdırma ve PDF görüntüleyici entegrasyonu |
| [`dram-app`](crates/dram-app) | `eframe`/`egui` tabanlı masaüstü kullanıcı arayüzü, Fluent yerelleştirme (`tr`/`en`) ve tema motoru |

---

## Kurulum ve Çalıştırma

Hazır derlenmiş sürümleri [GitHub Releases](https://github.com/MOzcelik14/draM/releases/latest) sayfasından indirebilir veya güncel Rust araç zinciriyle kaynaktan derleyebilirsiniz:

```bash
# Uygulamayı derleyin ve başlatın
cargo run -p dram-app

# Optimize edilmiş (release) derleme ile başlatın
cargo run --release -p dram-app

# Komut satırından doğrudan bir senaryo dosyası açın
cargo run -p dram-app -- yol/senaryo.dram
```

### Geliştirme ve Doğrulama

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Lisans

Bu proje **MIT** ([LICENSE-MIT](LICENSE-MIT)) veya **Apache-2.0** ([LICENSE-APACHE](LICENSE-APACHE)) lisansları altında çift lisanslıdır.

Gömülü **Courier Prime** yazı tipi ailesi **SIL Open Font License 1.1** kapsamında dağıtılmaktadır.
