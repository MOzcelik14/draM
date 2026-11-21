//! Pure-Rust Turkish and English spellchecker with suffix awareness, user dictionary,
//! and document-term whitelisting.

use std::collections::HashSet;

use crate::model::{Document, ElementType, ScriptLanguage};
use crate::text::{cmp_tr, to_lower_for_lang};

/// Trait for spellcheckers operating on screenplay text.
pub trait SpellChecker: Send + Sync {
    /// Returns `true` if `word` is spelled correctly for `lang`.
    fn check_word(&self, word: &str, lang: ScriptLanguage) -> bool;

    /// Returns spelling suggestions for `word` in `lang`.
    fn suggestions(&self, word: &str, lang: ScriptLanguage) -> Vec<String>;

    /// Adds `word` to the user's custom dictionary.
    fn add_to_user_dict(&mut self, word: &str);
}

/// Default no-op spellchecker used in headless contexts or when spellcheck is disabled.
#[derive(Debug, Default, Clone)]
pub struct NoopSpellChecker;

impl SpellChecker for NoopSpellChecker {
    fn check_word(&self, _word: &str, _lang: ScriptLanguage) -> bool {
        true
    }

    fn suggestions(&self, _word: &str, _lang: ScriptLanguage) -> Vec<String> {
        Vec::new()
    }

    fn add_to_user_dict(&mut self, _word: &str) {}
}

/// Built-in pure-Rust spellchecker supporting Turkish and English core vocabularies,
/// Turkish agglutinative suffix stripping, screenplay proper-noun whitelisting,
/// and a persistent user dictionary.
#[derive(Debug, Clone)]
pub struct BuiltinSpellChecker {
    tr_words: HashSet<String>,
    en_words: HashSet<String>,
    user_words: HashSet<String>,
    doc_words: HashSet<String>,
}

impl Default for BuiltinSpellChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinSpellChecker {
    /// Creates a new [`BuiltinSpellChecker`] preloaded with core Turkish and English dictionaries.
    #[must_use]
    pub fn new() -> Self {
        let tr_words: HashSet<String> = CORE_TR_WORDS
            .split_whitespace()
            .map(|w| to_lower_for_lang(w, ScriptLanguage::Tr))
            .collect();
        let en_words: HashSet<String> = CORE_EN_WORDS
            .split_whitespace()
            .map(|w| to_lower_for_lang(w, ScriptLanguage::En))
            .collect();
        Self {
            tr_words,
            en_words,
            user_words: HashSet::new(),
            doc_words: HashSet::new(),
        }
    }

    /// Loads newline-separated custom user dictionary entries.
    pub fn load_user_words(&mut self, raw: &str) {
        for line in raw.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.user_words
                    .insert(to_lower_for_lang(trimmed, ScriptLanguage::Tr));
                self.user_words
                    .insert(to_lower_for_lang(trimmed, ScriptLanguage::En));
            }
        }
    }

    /// Returns all custom user dictionary words sorted using Turkish collation.
    #[must_use]
    pub fn user_words_sorted(&self) -> Vec<String> {
        let mut words: Vec<String> = self.user_words.iter().cloned().collect();
        words.sort_by(|a, b| cmp_tr(a, b));
        words
    }

    /// Refreshes the per-document whitelist from character names, scene headings, and dictionary terms.
    pub fn sync_document_terms(&mut self, doc: &Document) {
        self.doc_words.clear();
        let lang = doc.settings.script_language;
        for el in &doc.elements {
            if matches!(el.kind, ElementType::Character | ElementType::SceneHeading) {
                for tok in el.plain_text().split(|c: char| !c.is_alphabetic()) {
                    if tok.chars().count() >= 2 {
                        self.doc_words.insert(to_lower_for_lang(tok, lang));
                    }
                }
            }
        }
        for term in doc
            .settings
            .dictionary
            .scene_prefixes
            .iter()
            .chain(doc.settings.dictionary.transitions.iter())
        {
            for tok in term.split(|c: char| !c.is_alphabetic()) {
                if tok.chars().count() >= 2 {
                    self.doc_words.insert(to_lower_for_lang(tok, lang));
                }
            }
        }
    }

    fn known_in_dict(&self, lower: &str, lang: ScriptLanguage) -> bool {
        if self.user_words.contains(lower) || self.doc_words.contains(lower) {
            return true;
        }
        match lang {
            ScriptLanguage::Tr => self.tr_words.contains(lower),
            ScriptLanguage::En => self.en_words.contains(lower),
        }
    }

    fn matches_turkish_stem(&self, lower: &str) -> bool {
        if self.known_in_dict(lower, ScriptLanguage::Tr) {
            return true;
        }
        // Handle Turkish apostrophe-separated proper nouns (e.g. "istanbul'da", "ayşe'nin").
        if let Some((stem, _)) = lower.split_once('\'') {
            if self.known_in_dict(stem, ScriptLanguage::Tr) || stem.chars().count() >= 2 {
                return true;
            }
        }

        // Iteratively strip common Turkish suffixes (up to 3 layers).
        let mut frontier = vec![lower.to_string()];
        let mut visited = HashSet::new();
        visited.insert(lower.to_string());

        for _ in 0..3 {
            let mut next = Vec::new();
            for candidate in &frontier {
                for suffix in TR_SUFFIXES {
                    if let Some(stem) = candidate.strip_suffix(suffix) {
                        if stem.chars().count() < 2 {
                            continue;
                        }
                        if self.known_in_dict(stem, ScriptLanguage::Tr) {
                            return true;
                        }
                        // Handle Turkish consonant softening (kitab -> kitap, sokağ -> sokak, ağac -> ağaç, reng -> renk, gid -> git)
                        for hardened in harden_turkish_stem(stem) {
                            if self.known_in_dict(&hardened, ScriptLanguage::Tr) {
                                return true;
                            }
                            if visited.insert(hardened.clone()) {
                                next.push(hardened);
                            }
                        }
                        if visited.insert(stem.to_string()) {
                            next.push(stem.to_string());
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }

        false
    }

    fn matches_english_stem(&self, lower: &str) -> bool {
        if self.known_in_dict(lower, ScriptLanguage::En) {
            return true;
        }
        if let Some((stem, _)) = lower.split_once('\'') {
            if self.known_in_dict(stem, ScriptLanguage::En) {
                return true;
            }
        }
        for suffix in EN_SUFFIXES {
            if let Some(stem) = lower.strip_suffix(suffix) {
                if stem.chars().count() < 2 {
                    continue;
                }
                if self.known_in_dict(stem, ScriptLanguage::En) {
                    return true;
                }
                let with_e = format!("{stem}e");
                if self.known_in_dict(&with_e, ScriptLanguage::En) {
                    return true;
                }
                if let Some(stripped) = stem.strip_suffix('i') {
                    let with_y = format!("{stripped}y");
                    if self.known_in_dict(&with_y, ScriptLanguage::En) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl SpellChecker for BuiltinSpellChecker {
    fn check_word(&self, word: &str, lang: ScriptLanguage) -> bool {
        let trimmed = word.trim_matches(|c: char| !c.is_alphabetic() && c != '\'');
        if trimmed.chars().count() < 3 {
            return true;
        }
        // Ignore words containing digits or all-uppercase acronyms/character cues.
        if trimmed.chars().any(|c| c.is_numeric()) || trimmed.chars().all(|c| !c.is_lowercase()) {
            return true;
        }

        let lower = to_lower_for_lang(trimmed, lang);
        match lang {
            ScriptLanguage::Tr => self.matches_turkish_stem(&lower),
            ScriptLanguage::En => self.matches_english_stem(&lower),
        }
    }

    fn suggestions(&self, word: &str, lang: ScriptLanguage) -> Vec<String> {
        let lower = to_lower_for_lang(word.trim(), lang);
        if lower.is_empty() {
            return Vec::new();
        }
        let target_len = lower.chars().count();
        let dict = match lang {
            ScriptLanguage::Tr => &self.tr_words,
            ScriptLanguage::En => &self.en_words,
        };

        let mut scored: Vec<(usize, &String)> = dict
            .iter()
            .chain(self.user_words.iter())
            .chain(self.doc_words.iter())
            .filter(|cand| cand.chars().count().abs_diff(target_len) <= 2)
            .filter_map(|cand| {
                let d = damerau_levenshtein(&lower, cand);
                (d > 0 && d <= 2).then_some((d, cand))
            })
            .collect();

        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| cmp_tr(a.1, b.1)));
        scored.dedup_by(|a, b| a.1 == b.1);

        let is_capitalized = word.chars().next().is_some_and(char::is_uppercase);
        scored
            .into_iter()
            .take(5)
            .map(|(_, cand)| {
                if is_capitalized {
                    capitalize_first(cand, lang)
                } else {
                    cand.clone()
                }
            })
            .collect()
    }

    fn add_to_user_dict(&mut self, word: &str) {
        let trimmed = word.trim();
        if !trimmed.is_empty() {
            self.user_words
                .insert(to_lower_for_lang(trimmed, ScriptLanguage::Tr));
            self.user_words
                .insert(to_lower_for_lang(trimmed, ScriptLanguage::En));
        }
    }
}

fn capitalize_first(word: &str, lang: ScriptLanguage) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let head = crate::text::to_upper_for_lang(&first.to_string(), lang);
    format!("{head}{}", chars.as_str())
}

fn harden_turkish_stem(stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let pairs = [('b', 'p'), ('c', 'ç'), ('d', 't'), ('ğ', 'k'), ('g', 'k')];
    for (soft, hard) in pairs {
        if let Some(prefix) = stem.strip_suffix(soft) {
            out.push(format!("{prefix}{hard}"));
        }
    }
    out
}

fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(dp[i - 2][j - 2] + 1);
            }
            dp[i][j] = best;
        }
    }
    dp[n][m]
}

const TR_SUFFIXES: &[&str] = &[
    "iyorum", "ıyorum", "uyorum", "üyorum", "iyorsun", "ıyorsun", "uyorsun", "üyorsun", "iyor",
    "ıyor", "uyor", "üyor", "yor", "ecek", "acak", "miş", "mış", "muş", "müş", "ler", "lar", "den",
    "dan", "ten", "tan", "nin", "nın", "nun", "nün", "nde", "nda", "nden", "ndan", "yle", "yla",
    "erek", "arak", "meden", "madan", "lik", "lık", "luk", "lük", "siz", "sız", "suz", "süz",
    "mek", "mak", "di", "dı", "du", "dü", "ti", "tı", "tu", "tü", "de", "da", "te", "ta", "in",
    "ın", "un", "ün", "im", "ım", "um", "üm", "iz", "ız", "uz", "üz", "le", "la", "li", "lı", "lu",
    "lü", "ci", "cı", "cu", "cü", "ki", "si", "sı", "su", "sü", "ye", "ya", "yi", "yı", "en", "an",
    "ip", "ıp", "up", "üp", "se", "sa", "e", "a", "i", "ı", "u", "ü", "m", "n", "r",
];

const EN_SUFFIXES: &[&str] = &["ing", "ed", "es", "ly", "er", "est", "s", "d"];

const CORE_TR_WORDS: &str = "
ve veya ile için ama fakat ancak çünkü gibi kadar daha çok az her hiç bir iki üç dört beş
altı yedi sekiz dokuz on bu şu o biz siz onlar ben sen kendi hep bütün tüm bazı birkaç
var yok değil evet hayır belki şimdi sonra önce bugün yarın dün sabah öğle akşam gece gün
yıl ay hafta saat dakika saniye zaman an yer mekan oda ev sokak cadde şehir istanbul ankara
izmir kapı pencere cam duvar tavan zemin masa sandalye koltuk yatak mutfak salon koridor
merdiven asansör bahçe park orman deniz sahil gökyüzü bulut güneş ay yıldız yağmur kar rüzgar
fırtına ışık karanlık gölge ses sessizlik gürültü müzik şarkı telefon ekran bilgisayar kağıt
kalem kitap defter mektup fotoğraf resim tablo ayna saat anahtar çanta cüzdan para araba
otobüs tren istasyon peron uçak gemi tekne yol köprü tünel kafe restoran bar otel hastane
okul ofis fabrika dükkan market polis doktor avukat öğretmen öğrenci yazar yönetmen oyuncu
adam kadın çocuk kız erkek genç yaşlı anne baba kardeş abi abla dede nine arkadaş dost
sevgili eş komşu yabancı insan kişi kalabalık göz yüz el kol ayak baş saç kalp nefes gözyaşı
gülümseme bakış söz konuşma diyalog fısıltı çığlık kahkaha sessiz sakin hızlı yavaş uzun
kısa büyük küçük eski yeni sıcak soğuk ağır hafif açık kapalı boş dolu karanlık aydınlık
temiz kirli güzel çirkin iyi kötü doğru yanlış gerçek yalan kolay zor derin yüksek alçak
uzak yakın sağ sol üst alt iç dış ön arka yan orta kenar köşe başlangıç son final sahne
senaryo kamera kadraj çekim plan kesme kararma açılma devam aksiyon karakter replik not
gel git bak gör duy dinle konuş söyle sor cevapla düşün anla bil unut hatırla bekle dur
yürü koş kaç yakala tut bırak aç kapat vur kır düş kalk otur kalkın yat uyu uyan ye iç
al ver koy çıkar çek it taşı getir götür yaz oku çiz sil gönder ara bul kaybet kazan
başla bitir devam et çalış oyna gül ağla bağır fısılda sarıl öp dön çevir uzat sakla
göster izle takip et yaklaş uzaklaş gir çık in bin sür geç kal öl yaşa sev nefret et kork
şaşır sevin üzül kız sakinleş nefes al ver bakışır gülümser yürür koşar durur bekler
oturur kalkar açar kapatır vurur parlar söner yanar akar eser yağar uyur uyanır gider gelir
";

const CORE_EN_WORDS: &str = "
the and for that with this from have not but they you are was were will would can could
should about into over after before under between through during without within along
across behind beyond near around among against upon toward inside outside above below
one two three four five six seven eight nine ten first second third last next other another
some any every all both each few many much more most such only own same than too very just
now then here there when where why how what which who whom whose yes no maybe never always
often sometimes still already yet soon late early today tomorrow yesterday morning afternoon
evening night day week month year hour minute second moment time place room house home
street road city town door window glass wall ceiling floor table chair desk bed kitchen
living hallway stairs elevator garden park forest sea beach sky cloud sun moon star rain
snow wind storm light dark shadow sound silence noise music song phone screen computer
paper pen book notebook letter photo picture mirror watch key bag wallet money car bus
train station platform plane ship boat bridge tunnel cafe restaurant bar hotel hospital
school office factory shop store police doctor lawyer teacher student writer director actor
man woman child girl boy young old mother father brother sister friend partner neighbor
stranger person people crowd eye face hand arm foot head hair heart breath tear smile look
word speech dialogue whisper scream laugh quiet calm fast slow long short big small old new
hot cold heavy light open closed empty full bright clean dirty beautiful ugly good bad right
wrong true false easy hard deep high low far close left right top bottom front back side
middle edge corner start end final scene script camera frame shot cut fade continue action
character line note come go look see hear listen speak talk say ask answer think know
understand forget remember wait stop walk run escape catch hold drop open close hit break
fall stand sit lie sleep wake eat drink take give put pull push carry bring write read draw
send search find lose win begin finish work play smile cry shout whisper hug turn hide show
watch follow approach leave enter exit climb drive pass stay live die love hate fear wonder
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Element;

    #[test]
    fn turkish_spellchecker_accepts_inflected_words_and_suggests_fixes() {
        let mut checker = BuiltinSpellChecker::new();
        assert!(checker.check_word("yağmurda", ScriptLanguage::Tr));
        assert!(checker.check_word("kapılar", ScriptLanguage::Tr));
        assert!(checker.check_word("İstanbul'dan", ScriptLanguage::Tr));
        assert!(checker.check_word("senaryo", ScriptLanguage::Tr));

        assert!(!checker.check_word("yağmr", ScriptLanguage::Tr));
        let sugg = checker.suggestions("yağmr", ScriptLanguage::Tr);
        assert!(sugg.iter().any(|s| s == "yağmur"));

        // User dictionary whitelisting
        assert!(!checker.check_word("Zebrania", ScriptLanguage::Tr));
        checker.add_to_user_dict("Zebrania");
        assert!(checker.check_word("Zebrania", ScriptLanguage::Tr));
    }

    #[test]
    fn document_terms_whitelist_character_and_location_names() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "İÇ. ZÜMRÜT KONAĞI - GECE"),
            Element::new(ElementType::Character, "MÜMTAZ"),
            Element::new(ElementType::Dialogue, "Mümtaz burada."),
        ];
        let mut checker = BuiltinSpellChecker::new();
        checker.sync_document_terms(&doc);
        assert!(checker.check_word("Mümtaz", ScriptLanguage::Tr));
        assert!(checker.check_word("Zümrüt", ScriptLanguage::Tr));
    }
}
