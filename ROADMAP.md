# `draM.` Geliştirme Yol Haritası (`v0.2.0+`)

Bu belge, `v0.1.0` sürümü sonrasında `draM.` için planlanan özellikleri ve teknik geliştirmeleri öncelik sırasına göre listeler.

---

## 1. Editör ve Sayfalama (Yüksek Öncelik — `v0.2.0` Adayları)

- [x] **İkili Diyalog (`Dual Dialogue`) Yan Yana Sayfalama ve PDF Çıktısı**
  - `dram-core` içindeki `pagination.rs` modülüne `DualSide::Left` ve `DualSide::Right` gruplarını yan yana iki sütun olarak yerleştirme desteği eklenmesi.
  - `dram-pdf` çıktısında iki sütunun hizalı şekilde render edilmesi ve birim/entegrasyon testlerinin genişletilmesi.

- [x] **Editör İçinde Zengin Metin Görünümü (`LayoutJob`) ve Seçili Metne Stil**
  - `egui::TextEdit` için özel `layouter` (`LayoutJob`) yazarak kalın (`bold`), italik (`italic`) ve altı çizili (`underline`) `TextRun` stillerinin editör kağıdı üzerinde görsel olarak gösterilmesi.
  - İmleç seçimi (selection range) üzerinden yalnızca seçili kelime veya aralığa `Ctrl+B` / `Ctrl+I` / `Ctrl+U` uygulanabilmesi.

- [x] **Sayfa Sonu (Page Break) Görsel Ayracı ve Sayfa Numarası Göstergesi**
  - Önbelleğe alınan `PaginatedDocument` verisi kullanılarak editör tuvalinde sayfa geçişlerine yatay ayraç ve sayfa numarası çizilmesi.

- [x] **Hassas Sahne Atlama ve Kaydırma Hizalaması**
  - Sağ paneldeki sahne gezgininden uzak bir sahneye tıklandığında editör `ScrollArea` konumunun doğrudan ilgili satıra kaydırılması.

---

## 2. Senarist ve Yapım Araçları (Orta Öncelik)

- [x] **İndeks Kartları (Index Cards / Beat Board) Görünümü**
  - Her sahneyi başlık, özet (`Synopsis`) ve etiketle kartlar halinde gösteren, sürükle-bırak ile sahne sırasını değiştirmeye izin veren görünüm sekmesi.

- [x] **Revizyon Modu ve Revizyon Geçmişi**
  - Değiştirilen veya yeni eklenen satırları otomatik olarak seçili revizyon rengiyle (`RevisionMark`) işaretleme.
  - Revizyon sayfalarını filtreleme ve PDF çıktıda kenar yıldızı (`*`) basımı.

- [x] **Senaryo Raporları (Breakdown ve İstatistikler)**
  - Karakter konuşma sayısı, kelime/sayfa oranı ve sahne görünme matrisi.
  - Mekan (`İÇ.` / `DIŞ.`) ve zaman (`GÜN` / `GECE`) dağılım raporu (CSV ve PDF dışa aktarım).

- [x] **Türkçe Yazım Denetimi (Spellcheck)**
  - Saf Rust tabanlı veya gömülü sözlük destekli Türkçe/İngilizce yazım denetimi.

- [x] **Satır İçi Otomatik Tamamlama Açılır Penceresi (Inline Popup)**
  - `autocomplete_candidates` önerilerini yazım sırasında doğrudan imleç altında `Yukarı`/`Aşağı`/`Enter` ile seçilebilir açılır liste olarak sunma.

---

## 3. `.dram` Formatı ve `prisM. Suite` Ekosistemi (Uzun Vadeli)

- [x] **Gömülü Medya ve Referans Görselleri (`embedded_assets`)**
  - `DramManifest.embedded_assets` alanı kullanılarak sahne veya karakter notlarına referans görselleri ekleme.

- [x] **Başlık Sayfası Canlı Önizlemesi ve Şablonlar**
  - Uzun metraj, kısa film ve dizi bölümü için hazır başlık sayfası ve proje şablonları.

- [x] **Dosya İlişkilendirme ve Son Açılanlar (Recent Files)**
  - Komut satırı argümanı ile `.dram` dosyası açma (`dram <dosya.dram>`), işletim sistemi uzantı ilişkilendirmesi ve *Dosya -> Son Kullanılanlar* menüsü.

