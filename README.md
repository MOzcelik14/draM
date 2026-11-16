<p>
  <img src="assets/logo-mark.svg" alt="draM. logosu" width="64" height="64">
</p>

# draM.

**draM.**, [prisM. Suite](https://github.com/MOzcelik14/prism-suite) ailesinin bir parçası olarak senaristler, yönetmenler ve yapım ekipleri için geliştirilmiş, Türkçe ve İngilizce senaryo yazım kurallarını doğrudan destekleyen masaüstü senaryo yazım uygulamasıdır.

Yalnızca yazı yazmaya odaklanan sade arayüzü, otomatik biçimlendirme akışı, endüstri standardında PDF çıktısı ve yaygın senaryo formatlarıyla tam uyumluluğuyla projelerinizi tek bir çalışma alanında yönetmenizi sağlar.

---

## Temel Özellikler

### Endüstri Standardı Senaryo Biçimlendirme
- Sahne başlığı, aksiyon, karakter, parantez içi, diyalog, ikili diyalog, geçiş ve çekim elemanları arasında `Enter` ve `Tab` tuşlarıyla kesintisiz geçiş.
- `Ctrl+1` – `Ctrl+8` kısayollarıyla tek tuşla eleman türü değiştirme.
- Belge içindeki karakter ve mekân adlarından öğrenen, Türkçe alfabe sırasına duyarlı otomatik tamamlama.

### Türkçe ve İngilizce İçin Tam Uyum
- **Bağımsız Arayüz ve Senaryo Dili:** Türkçe arayüzle İngilizce senaryo veya İngilizce arayüzle Türkçe senaryo yazabilirsiniz.
- **Doğru Büyük/Küçük Harf Kuralları:** `İ/i` ve `I/ı` dönüşümleri senaryo diline göre otomatik ve hatasız işlenir.
- **Yerel Terimler:** Türkçe senaryolarda `İÇ.`, `DIŞ.`, `KESME.` ve `(DEVAM)`; İngilizce senaryolarda `INT.`, `EXT.`, `CUT TO:`, `(MORE)` ve `(CONT'D)` kuralları uygulanır.

### Dosya Formatları ve Baskı Kalitesi
- **`.dram` Proje Formatı:** Tüm senaryo içeriğini, notları ve proje ayarlarını tek bir güvenli arşivde saklar; atomik kayıt ve çökme kurtarma desteği sunar.
- **PDF Dışa Aktarma:** Courier Prime yazı tipi gömülü, A4 ve US Letter seçenekli, başlık sayfalı, otomatik sayfa numaralı ve sayfa sonu kurallarına uygun baskıya hazır PDF üretir.
- **Doğrudan Yazdırma:** İşletim sistemindeki yazıcılara doğrudan gönderim ve varsayılan PDF görüntüleyicide açma seçeneği.
- **Geniş Format Uyumu:** Fountain (`.fountain`), Fade In (`.fadein`) ve Final Draft (`.fdx`) formatlarında içe ve dışa aktarma.

### Çalışma Alanı Araçları
- **Sahne Gezgini:** Sahneler arası hızlı geçiş ve sahne bloklarını geri alınabilir şekilde yeniden sıralama.
- **Notlar Paneli:** Sahnelere ve satırlara bağlı veya bağımsız çalışma notları.
- **Bul ve Değiştir:** Türkçe büyük/küçük harf kurallarına duyarlı arama ve toplu değiştirme.
- **Revizyon Renkleri ve Temalar:** Mavi, pembe, sarı ve yeşil revizyon işaretleri; koyu/açık tema ve `F11` odaklanma modu.

---

## Klavye Kısayolları

| Kısayol | İşlev |
| :--- | :--- |
| `Enter` | Senaryo akışına göre bir sonraki eleman türünü ekler |
| `Tab` | Bulunulan satırın eleman türünü değiştirir |
| `Ctrl+1` – `Ctrl+8` | Eleman türünü doğrudan seçer |
| `Ctrl+S` | Projeyi kaydeder |
| `Ctrl+Z` / `Ctrl+Y` | Son işlemi geri alır / yineler |
| `Ctrl+F` | Bul ve Değiştir penceresini açar |
| `Ctrl+P` | Yazdırma penceresini açar |
| `F11` | Odaklanma modunu açar veya kapatır |

---

## Kurulum ve Kaynaktan Çalıştırma

Uygulama Linux, Windows ve macOS sistemlerde çalışacak şekilde tasarlanmıştır. Kaynak koddan çalıştırmak için sisteminizde güncel Rust araç zincirinin (`rustc` / `cargo`) kurulu olması yeterlidir.

```bash
# Uygulamayı derleyin ve başlatın
cargo run -p dram-app
```

Test ve kalite doğrulamalarını çalıştırmak için:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Ek Belgeler ve Lisans

- Proje dosya formatı ayrıntıları: [FORMAT.md](FORMAT.md)
- Çoklu platform dağıtım yapılandırması: [dist-workspace.toml](dist-workspace.toml)
- Lisans: MIT veya Apache-2.0 (Courier Prime yazı tipi SIL Open Font License 1.1 kapsamındadır).
