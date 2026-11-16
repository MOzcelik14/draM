const FALLBACK_VERSION = "0.0.1";
const REPO = "MOzcelik14/draM";

const I18N = {
  tr: {
    "nav-downloads": "İndir",
    "nav-features": "Özellikler",
    "nav-formats": "Formatlar",
    "nav-start": "Başlangıç",
    "theme-btn": "Tema",
    "hero-eyebrow": "Masaüstü Senaryo Yazım Uygulaması",
    "hero-title": "Türkçe ve İngilizce senaryolar için sade, hızlı ve güvenilir çalışma alanı.",
    "hero-lead": "draM., senaristler ve yapım ekipleri için tasarlanmış yerel bir masaüstü uygulamasıdır. Yazım akışını bozmadan endüstri standardında biçimlendirme, baskıya hazır PDF üretimi ve yaygın formatlarla tam uyum sunar.",
    "cta-download": "Son Sürümü İndir",
    "cta-formats": "Desteklenen Formatlar",
    "hero-meta": "Linux, Windows ve macOS için yerel masaüstü uygulaması. MIT / Apache-2.0 lisanslı.",
    "sheet-title": "ornek_senaryo.dram",
    "sheet-lang": "Senaryo Dili: TR",
    "sc-heading": "İÇ. KAFE - GECE",
    "sc-action": "Yağmur İstanbul sokaklarını yıkar. ÇİĞDEM (30) pencere kenarındaki masada not defterini açar.",
    "sc-character": "ÇİĞDEM",
    "sc-paren": "(alçak sesle)",
    "sc-dialogue": "Bu gece her şey değişecek. İlk sahneden itibaren.",
    "sc-transition": "KESME.",
    "dock-fmt": "Biçim",
    "dock-chr": "Karakter",
    "dock-item-1": "1 Sahne Başlığı",
    "dock-item-2": "2 Aksiyon",
    "dock-item-3": "3 Karakter",
    "dock-item-4": "5 Diyalog",
    "dock-page": "Sayfa 1 / 1",
    "dl-title": "İndir ve Sürümler",
    "dl-sub": "Güncel kararlı sürüm paketleri her güncellemede otomatik olarak derlenir ve SHA-256 doğrulamasıyla yayınlanır.",
    "dl-current-label": "Güncel Sürüm",
    "dl-linux-title": "Debian / Ubuntu & Taşınabilir",
    "dl-linux-desc": "x86_64 sistemler için .deb kurulum paketi veya kurulum gerektirmeyen tar.gz arşivi.",
    "dl-win-title": "Windows 10 / 11 (x86_64)",
    "dl-win-desc": "Doğrudan çalıştırılabilen dram.exe ve dil dosyalarını içeren taşınabilir ZIP paketi.",
    "dl-mac-title": "Apple Silicon (arm64)",
    "dl-mac-desc": "Apple M-serisi işlemcili Mac bilgisayarlar için derlenmiş yerel uygulama arşivi.",
    "dl-all-releases": "Tüm Sürümler ve Sürüm Notları",
    "feat-title": "Yazım Sürecine Odaklanan Mimari",
    "feat-sub": "Karmaşık menüler ve dikkat dağıtıcı araçlar yerine doğrudan senaryo üretimine hizmet eden temel araçlar.",
    "f1-title": "Akışkan Senaryo Editörü",
    "f1-desc": "Enter ve Tab tuşlarıyla sahne başlığı, aksiyon, karakter, parantez içi ve diyalog arasında kesintisiz geçiş. Ctrl+1–8 kısayollarıyla anında tür değişimi.",
    "f2-title": "Birinci Sınıf Türkçe ve İngilizce",
    "f2-desc": "Arayüz dili ve senaryo dili birbirinden bağımsızdır. Türkçe İ/i ve I/ı büyük-küçük harf kuralları, Türkçe alfabe sıralaması ve yerel terimler (İÇ., DIŞ., DEVAM) yerleşiktir.",
    "f3-title": "Baskıya Hazır PDF ve Yazdırma",
    "f3-desc": "Courier Prime yazı tipi gömülü, A4 ve US Letter uyumlu, başlık sayfalı ve sayfa bölünmelerinde otomatik (DEVAM)/(MORE) işaretli PDF üretimi.",
    "f4-title": "Sahne Gezgini, Notlar ve Revizyonlar",
    "f4-desc": "Sahneler arasında tek tıkla gezinin, sahne bloklarını geri alınabilir şekilde sıralayın, çalışma notları ekleyin ve revizyon renkleriyle değişiklikleri izleyin.",
    "fmt-title": "Format Uyumluluğu",
    "fmt-sub": "Projelerinizi açık ve güvenli .dram arşivinde saklayın; sektördeki diğer yazılımlarla sorunsuz dosya alışverişi yapın.",
    "th-format": "Format",
    "th-ext": "Uzantı",
    "th-support": "Destek",
    "th-notes": "Açıklama",
    "f-dram-sup": "Okuma / Yazma",
    "f-dram-note": "Sürümlü ZIP arşivi, atomik kayıt ve çökme kurtarma.",
    "f-pdf-sup": "Dışa Aktarma",
    "f-pdf-note": "Gömülü Courier Prime, A4 ve US Letter, başlık sayfası.",
    "f-fnt-sup": "İçe / Dışa Aktarma",
    "f-fnt-note": "Fountain 1.1 tam spesifikasyon ve Türkçe karakter uyumu.",
    "f-fi-sup": "İçe / Dışa Aktarma",
    "f-fi-note": "Open Screenplay Format tabanlı .fadein belge uyumu.",
    "f-fdx-sup": "İçe / Dışa Aktarma",
    "f-fdx-note": "Final Draft XML belge ve temel metin stilleri uyumu.",
    "start-title": "Kurulum ve Kısayollar",
    "start-sub": "Hazır paketleri indirerek veya güncel Rust araç zinciriyle kaynaktan derleyerek hemen kullanabilirsiniz.",
    "card-install-title": "Kaynaktan Çalıştırma",
    "card-install-desc": "Linux, Windows ve macOS üzerinde harici tarayıcı motoru veya çalışma zamanı gerektirmeden doğrudan yerel uygulama olarak çalışır.",
    "card-keys-title": "Temel Klavye Kısayolları",
    "card-keys-desc": "Enter: Sonraki eleman | Tab: Tür değiştir | Ctrl+1..8: Eleman seç | Ctrl+S: Kaydet | Ctrl+F: Bul/Değiştir | Ctrl+P: Yazdır | F11: Odaklanma Modu",
    "footer-left": "draM. — Senaryo Yazım Uygulaması",
    "footer-right": "MIT / Apache-2.0 Lisansı | Courier Prime (SIL OFL 1.1)"
  },
  en: {
    "nav-downloads": "Download",
    "nav-features": "Features",
    "nav-formats": "Formats",
    "nav-start": "Get Started",
    "theme-btn": "Theme",
    "hero-eyebrow": "Desktop Screenwriting Application",
    "hero-title": "A focused, reliable workspace for Turkish and English screenplays.",
    "hero-lead": "draM. is a native desktop application built for screenwriters and production teams. It delivers distraction-free formatting flow, print-ready PDF output, and broad compatibility with industry formats.",
    "cta-download": "Download Latest",
    "cta-formats": "Supported Formats",
    "hero-meta": "Native desktop app for Linux, Windows, and macOS. Licensed under MIT / Apache-2.0.",
    "sheet-title": "sample_script.dram",
    "sheet-lang": "Script Language: EN",
    "sc-heading": "INT. CAFE - NIGHT",
    "sc-action": "Rain washes the Istanbul streets. CIGDEM (30) opens her notebook at a quiet corner table.",
    "sc-character": "CIGDEM",
    "sc-paren": "(under her breath)",
    "sc-dialogue": "Everything changes tonight. Starting from the very first scene.",
    "sc-transition": "CUT TO:",
    "dock-fmt": "Format",
    "dock-chr": "Characters",
    "dock-item-1": "1 Scene Heading",
    "dock-item-2": "2 Action",
    "dock-item-3": "3 Character",
    "dock-item-4": "5 Dialogue",
    "dock-page": "Page 1 of 1",
    "dl-title": "Download & Releases",
    "dl-sub": "Stable release packages are built automatically on every update and published with SHA-256 checksums.",
    "dl-current-label": "Latest Version",
    "dl-linux-title": "Debian / Ubuntu & Portable",
    "dl-linux-desc": "Installable .deb package or standalone tar.gz archive for x86_64 Linux systems.",
    "dl-win-title": "Windows 10 / 11 (x86_64)",
    "dl-win-desc": "Portable ZIP archive containing dram.exe and localization files.",
    "dl-mac-title": "Apple Silicon (arm64)",
    "dl-mac-desc": "Native application archive compiled for Apple Silicon Mac computers.",
    "dl-all-releases": "All Releases & Release Notes",
    "feat-title": "Built Around the Writing Flow",
    "feat-sub": "Essential tools that serve screenplay craft without visual clutter or bloated menus.",
    "f1-title": "Fluid Screenplay Editor",
    "f1-desc": "Move effortlessly between scene headings, action, characters, parentheticals, and dialogue with Enter and Tab, or switch types directly with Ctrl+1–8.",
    "f2-title": "First-Class Turkish & English",
    "f2-desc": "UI language and script language are independent. Proper Turkish İ/i and I/ı casing, Turkish alphabetical sorting, and localized markers (İÇ./DIŞ. vs INT./EXT.) are built in.",
    "f3-title": "Print-Ready PDF & Printing",
    "f3-desc": "Generates industry-standard PDFs with embedded Courier Prime, A4/US Letter support, separate title pages, and automatic (MORE)/(CONT'D) dialogue splits.",
    "f4-title": "Scene Navigator, Notes & Revisions",
    "f4-desc": "Jump between scenes, reorder scene blocks with full undo support, attach production notes, and track edits with revision colors.",
    "fmt-title": "Format Compatibility",
    "fmt-sub": "Store projects safely in the open .dram archive while exchanging scripts seamlessly with other tools.",
    "th-format": "Format",
    "th-ext": "Extension",
    "th-support": "Support",
    "th-notes": "Notes",
    "f-dram-sup": "Read / Write",
    "f-dram-note": "Versioned ZIP container with atomic saves and crash recovery.",
    "f-pdf-sup": "Export",
    "f-pdf-note": "Embedded Courier Prime, A4 and US Letter, title page.",
    "f-fnt-sup": "Import / Export",
    "f-fnt-note": "Full Fountain 1.1 specification with Turkish text support.",
    "f-fi-sup": "Import / Export",
    "f-fi-note": "Open Screenplay Format (.fadein) document compatibility.",
    "f-fdx-sup": "Import / Export",
    "f-fdx-note": "Final Draft XML document and inline styling support.",
    "start-title": "Installation & Shortcuts",
    "start-sub": "Download a prebuilt package or build natively from source with an up-to-date Rust toolchain.",
    "card-install-title": "Build from Source",
    "card-install-desc": "Runs natively on Linux, Windows, and macOS with no embedded browser runtime required.",
    "card-keys-title": "Core Keyboard Shortcuts",
    "card-keys-desc": "Enter: Next element | Tab: Cycle type | Ctrl+1..8: Pick element | Ctrl+S: Save | Ctrl+F: Find/Replace | Ctrl+P: Print | F11: Focus Mode",
    "footer-left": "draM. — Screenwriting Application",
    "footer-right": "MIT / Apache-2.0 License | Courier Prime (SIL OFL 1.1)"
  }
};

function applyVersion(rawVersion) {
  const ver = String(rawVersion || FALLBACK_VERSION).replace(/^v/, "");
  const tag = `v${ver}`;
  const base = `https://github.com/${REPO}/releases/download/${tag}`;

  const setText = (id, text) => {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  };
  const setHref = (id, url) => {
    const el = document.getElementById(id);
    if (el) el.setAttribute("href", url);
  };

  setText("hero-version-pill", tag);
  setText("dl-version-text", tag);
  setText("footer-version", tag);

  setHref("dl-linux-deb", `${base}/draM_${ver}_amd64.deb`);
  setHref("dl-linux-tar", `${base}/draM-${ver}-linux-x86_64.tar.gz`);
  setHref("dl-win-zip", `${base}/draM-${ver}-windows-x86_64.zip`);
  setHref("dl-mac-tar", `${base}/draM-${ver}-macos-arm64.tar.gz`);
  setHref("dl-checksums", `${base}/SHA256SUMS.txt`);
  setHref("dl-release-page", `https://github.com/${REPO}/releases/tag/${tag}`);
}

async function syncLatestVersion() {
  applyVersion(FALLBACK_VERSION);
  try {
    const res = await fetch("./version.json", { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      if (data && data.version) {
        applyVersion(data.version);
      }
    }
  } catch (_) {
    // Keep fallback version when opened locally
  }
}

function setLang(lang) {
  const dict = I18N[lang] || I18N.tr;
  document.documentElement.lang = lang;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (dict[key]) {
      el.textContent = dict[key];
    }
  });
  document.querySelectorAll("[data-lang-btn]").forEach((btn) => {
    btn.setAttribute("aria-pressed", btn.getAttribute("data-lang-btn") === lang ? "true" : "false");
  });
}

function toggleTheme() {
  const cur = document.documentElement.getAttribute("data-theme");
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const isDark = cur ? cur === "dark" : prefersDark;
  const next = isDark ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", next);
  const btn = document.getElementById("theme-toggle");
  if (btn) {
    btn.setAttribute("aria-pressed", next === "dark" ? "true" : "false");
  }
}

document.addEventListener("DOMContentLoaded", () => {
  document.querySelectorAll("[data-lang-btn]").forEach((btn) => {
    btn.addEventListener("click", () => setLang(btn.getAttribute("data-lang-btn")));
  });
  const themeBtn = document.getElementById("theme-toggle");
  if (themeBtn) {
    themeBtn.addEventListener("click", toggleTheme);
  }
  setLang("tr");
  syncLatestVersion();
});
