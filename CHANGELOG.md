# Değişiklik Günlüğü (Changelog)

Bu proje [Semantic Versioning](https://semver.org/) kurallarını takip eder.

## [1.0.2] - 2026-11-25

Veri bütünlüğü, test kapsamı, mimari düzenleme, performans doğrulaması ve platform olgunluğu sürümü.

### Veri Güvenliği ve Dayanıklılık
- **Autosave / Kayıt Senkronizasyonu:** `RecoveryCoordinator` ve `DramManifest.generation` sayacı ile arka plan otomatik kayıtları ile normal kaydetme/kapanış arasındaki yarış durumu giderildi; kapanışta devam eden autosave beklenip doğrulama yapılıyor.
- **Atomik `Batch` Undo/Redo ve Sınırlar:** `EditCommand::Batch` kısmi hata durumunda ters sıra rollback uygular hale getirildi; UUID tabanlı komut varyantları, `UndoLimits` bellek/adet sınırı ve `mark_checkpoint` eklendi.
- **`.dram` Arşiv Güvenliği:** Tekil JSON (`32 MiB`) ve asset (`10 MiB`) sınırlarına ek olarak toplam açılmış arşiv boyutu (`256 MiB`) ve maksimum asset sayısı (`256`) sınırları eklendi.

### Test Altyapısı, Performans ve Mimari
- **Kapsamlı Test Paketleri:** Pagination edge-case testleri, undo/redo derin zincir ve hata rollback testleri, Fountain 1.1 / FDX uyumluluk testleri, PDF yapısal golden reference testleri, Courier Prime font metric doğrulaması (`verify_font_metrics`) ve Linux Türkçe IME regresyon testleri eklendi.
- **Modüler `dram-app` Yapısı:** `main.rs` sorumluluklarına göre `app`, `commands`, `dialogs`, `document`, `panels`, `recovery`, `settings` ve `workspace` modüllerine ayrıldı.
- **Virtualization ve Benchmark:** `editor.rs` içinde sarılan satır sayısına göre off-screen yükseklik tahmini (`estimate_element_height`) iyileştirildi ve 5–500 sayfa sentetik `perf_bench` paketi eklendi.

### Platform ve Erişilebilirlik
- **Linux & macOS Paketleme:** AppStream metadata (`io.github.mozcelik14.dram.metainfo.xml`) ve MIME XML (`dram-mime.xml`) Linux paketlerine eklendi; `release.yml` içine secret tabanlı macOS `codesign` ve notarization adımları eklendi.
- **Erişilebilirlik:** Editör metin alanlarına `WidgetInfo::text_edit` ile yerelleştirilmiş eleman türü etiketleri bağlandı.

---

## [1.0.1] - 2026-11-22

Güvenlik, veri bütünlüğü, yazdırma dayanıklılığı ve arayüz performansına odaklanan bakım sürümü.

### Güvenlik ve Dayanıklılık
- **Bağımlılık Güncellemeleri:** `zip` (`>=9.0.5`, kilitli `9.8.1`) ve `quick-xml` (`>=0.41.0`) sürümlerine yükseltilerek bilinen RustSec zafiyetleri giderildi.
- **Sınırlandırılmış Okuma (ZIP Bomb / DoS Koruması):** `.dram` JSON girdileri, `.fadein` (`document.xml`), `.fdx` ve `.fountain`/`.txt` dosya okumalarına `32 MiB` boyut sınırı (`read_bounded`) eklendi.
- **Yazdırma Güvenliği ve Geçici Dosya Hijyeni:** `dram-print` içinde yazıcı adı ve sayfa aralığı doğrulaması, Unix'te `0600` izinli geçici PDF oluşturma, Windows PowerShell çağrısında çevre değişkeni kullanımı ve yazdırma penceresi/uygulama kapanışında geçici PDF temizliği sağlandı.
- **Atomik Dışa Aktarım ve Şema Göçü:** `.fdx`, `.fadein`, `.fountain`, `.pdf` ve yapılandırma dosyaları geçici dosya + `sync_all` + `rename` ile atomik yazıma geçirildi; `.dram` için `v0 -> v1` yük normalizasyonu eklendi.

### Performans ve CI
- **PDF Font Önbelleği:** `dram-pdf` font yükleme ve Türkçe glif doğrulaması `OnceLock` ile önbelleğe alındı.
- **Arayüz Akıcılığı:** Otomatik kurtarma kaydı arka plan iş parçacığına taşındı, tema uygulaması yalnızca değişimde çalıştırıldı ve yazım denetimi bağlam menüsü açılışıyla sınırlandırıldı.
- **CI:** GitHub Actions iş akışına `macos-ci` derleme ve test işi eklendi.

---

## [1.0.0] - 2026-11-21

İlk kararlı sürüm. `draM.`, Türkçe ve İngilizce senaryo yazımı için saf Rust (`#![forbid(unsafe_code)]`) ile geliştirilmiş tam donanımlı masaüstü senaryo editörü olarak yayınlandı.

### Eklenenler
- **İkili Diyalog (`Dual Dialogue`) Yan Yana Sayfalama ve PDF Çıktısı:** `dram-core` sayfalama motorunda aynı gruptaki sol/sağ karakter ve diyalog blokları yan yana iki sütun olarak yerleştirilir ve `dram-pdf` tarafından hizalı şekilde dışa aktarılır.
- **Editörde Zengin Metin (`LayoutJob`) ve Seçili Metne Stil:** Gömülü Courier Prime Regular, Bold, Italic ve BoldItalic yazı tipleriyle kalın, italik ve altı çizili stiller doğrudan editör kağıdı üzerinde görsel olarak işlenir; `Ctrl+B` / `Ctrl+I` / `Ctrl+U` kısayolları seçili kelime aralığına veya tüm elemana uygulanır.
- **Sayfa Sonu Görsel Ayracı ve Hassas Sahne Atlama:** Önbellekteki sayfalama verisiyle editör kağıdı üzerinde kesikli sayfa geçiş ayraçları ve sayfa numaraları gösterilir; sağ panelden uzak sahnelere tıklandığında editör doğrudan hedef satıra hizalanır.
- **İndeks Kartları (Beat Board) Görünümü:** Sahneleri çok sütunlu kartlar halinde listeleyen, doğrudan kart üzerinden `Synopsis` düzenleme, sahne sırası değiştirme ve editörde açma imkanı sunan alternatif görünüm eklendi.
- **Revizyon Modu ve PDF Kenar Yıldızı (`*`):** Revizyon Modu açıkken düzenlenen ve yeni eklenen satırlar seçili renkle otomatik işaretlenir; PDF çıktıda revizyonlu satırların sağ kenarına `*` basılır.
- **Senaryo Raporları ve CSV Dışa Aktarım:** Karakter (sahne, replik, kelime) ve mekan (`İÇ.`/`DIŞ.`, `GÜN`/`GECE`) dağılım istatistikleri hesaplanır, uygulama içi tablolarda görüntülenir ve CSV olarak dışa aktarılabilir.
- **Satır İçi Otomatik Tamamlama (Inline Popup):** Karakter, Sahne Başlığı ve Geçiş elemanları için odaklı satırın hemen altında klavye (`Yukarı`/`Aşağı`/`Ctrl+Space`/`Esc`) ve fare ile seçilebilir öneri listesi eklendi.
- **Saf Rust Türkçe ve İngilizce Yazım Denetimi (`Spellcheck`):** Türkçe ek soyucu, belgedeki karakter ve mekan adlarını otomatik tanıma, kalıcı kullanıcı sözlüğü (`user_dict.txt`), kırmızı alt çizgi vurgusu ve sağ tık düzeltme önerileri eklendi.
- **Gömülü Referans Görselleri (`embedded_assets`):** Notlar sekmesine PNG/JPEG referans görseli ekleme, küçük önizleme, tam boy görüntüleme ve görselleri doğrudan `.dram` ZIP arşivine gömme desteği eklendi.
- **Proje Şablonları ve Başlık Sayfası Canlı Önizlemesi:** Türkçe ve İngilizce Uzun Metraj Film, Kısa Film ve Dizi Bölümü şablonları ile Başlık Sayfası düzenleyicisinde anlık kağıt önizleme kartı eklendi.
- **CLI Dosya Açma ve Son Kullanılanlar:** `dram <dosya>` argümanı ile `.dram`, `.fountain`, `.fdx` ve `.fadein` dosyalarını doğrudan açma ve kalıcı *Dosya -> Son Kullanılanlar* menüsü eklendi.

### İyileştirmeler
- Genel arayüz ölçeği (`1.08x`), üst araç çubuğu dikey ortalama, belirgin öneri şeridi, sabit sütunlu biçim listesi ve uçtan uca orantılı sağ panel sekme hizalaması tamamlandı.

---

## [0.1.0] - 2026-11-16

### Eklenenler
- 15 saniyede bir otomatik kayıt ve açılışta çökme kurtarma (*Kurtar / Yoksay*) penceresi.
- Sayfalama önbelleği (`PaginatedDocument` caching) ve 120 elemandan uzun senaryolar için editör sanallaştırması.
- Satır içi stil kısayolları (`Ctrl+B`, `Ctrl+I`, `Ctrl+U`) ve düzenleme sırasında `TextRun` stil koruması.
- `Fountain -> Document -> .dram -> Document -> PDF` uçtan uca entegrasyon testi.
- `LICENSE-MIT`, `LICENSE-APACHE` dosyaları ve genişletilmiş `FORMAT.md` JSON şema örnekleri.

---

## [0.0.1] - 2026-11-16

### Eklenenler
- İlk halka açık önizleme sürümü: `.dram`, Fountain, FDX, Fade In ve saf Rust PDF (`krilla` + gömülü Courier Prime) desteği, Türkçe/İngilizce arayüz ve Linux X11/ibus Türkçe IME desteği.
