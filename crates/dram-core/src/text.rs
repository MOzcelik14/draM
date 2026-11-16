//! Turkish-aware text processing utilities.
//!
//! Standard [`str::to_uppercase`] and [`str::to_lowercase`] are locale-independent and
//! map `'i'` to `'I'` and `'I'` to `'i'`, which breaks Turkish orthography (`i` ↔ `İ`,
//! `ı` ↔ `I`). All screenplay text transformations in `draM.` route through this module.

use std::cmp::Ordering;
use unicode_segmentation::UnicodeSegmentation;

use crate::model::ScriptLanguage;

/// Converts `s` to uppercase using Turkish casing rules (`i` → `İ`, `ı` → `I`).
#[must_use]
pub fn to_upper_tr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'i' => out.push('İ'),
            'ı' => out.push('I'),
            _ => {
                for up in ch.to_uppercase() {
                    out.push(up);
                }
            }
        }
    }
    out
}

/// Converts `s` to lowercase using Turkish casing rules (`İ` → `i`, `I` → `ı`).
#[must_use]
pub fn to_lower_tr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'İ' => out.push('i'),
            'I' => out.push('ı'),
            '\u{0307}' => {
                // Combining dot above can appear after 'I' in decomposed Turkish text; skip if preceded by 'i'.
                if !out.ends_with('i') {
                    out.push('\u{0307}');
                }
            }
            _ => {
                for low in ch.to_lowercase() {
                    out.push(low);
                }
            }
        }
    }
    out
}

/// Converts `s` to uppercase according to `lang`.
#[must_use]
pub fn to_upper_for_lang(s: &str, lang: ScriptLanguage) -> String {
    match lang {
        ScriptLanguage::Tr => to_upper_tr(s),
        ScriptLanguage::En => s.to_uppercase(),
    }
}

/// Converts `s` to lowercase according to `lang`.
#[must_use]
pub fn to_lower_for_lang(s: &str, lang: ScriptLanguage) -> String {
    match lang {
        ScriptLanguage::Tr => to_lower_tr(s),
        ScriptLanguage::En => s.to_lowercase(),
    }
}

/// Returns a collation rank for a Turkish lowercase character so that
/// `c < ç`, `g < ğ`, `ı < i`, `o < ö`, `s < ş`, `u < ü`.
const fn tr_char_rank(ch: char) -> (u8, u32) {
    let primary = match ch {
        'a' => 10,
        'b' => 11,
        'c' => 12,
        'ç' => 13,
        'd' => 14,
        'e' => 15,
        'f' => 16,
        'g' => 17,
        'ğ' => 18,
        'h' => 19,
        'ı' => 20,
        'i' => 21,
        'j' => 22,
        'k' => 23,
        'l' => 24,
        'm' => 25,
        'n' => 26,
        'o' => 27,
        'ö' => 28,
        'p' => 29,
        'q' => 30,
        'r' => 31,
        's' => 32,
        'ş' => 33,
        't' => 34,
        'u' => 35,
        'ü' => 36,
        'v' => 37,
        'w' => 38,
        'x' => 39,
        'y' => 40,
        'z' => 41,
        _ => 255,
    };
    let secondary = if primary == 255 { ch as u32 } else { 0 };
    (primary, secondary)
}

/// Compares two strings using Turkish alphabetical collation (`c < ç`, `g < ğ`, `ı < i`,
/// `o < ö`, `s < ş`, `u < ü`), case-insensitively first and breaking ties by original chars.
#[must_use]
pub fn cmp_tr(a: &str, b: &str) -> Ordering {
    let a_low = to_lower_tr(a);
    let b_low = to_lower_tr(b);
    for (ca, cb) in a_low.chars().zip(b_low.chars()) {
        let ord = tr_char_rank(ca).cmp(&tr_char_rank(cb));
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a_low
        .chars()
        .count()
        .cmp(&b_low.chars().count())
        .then_with(|| a.cmp(b))
}

/// Finds the first case-insensitive match of `needle` in `haystack` using `lang`'s casing rules.
///
/// Returns `(byte_start, byte_end)` in `haystack` if found.
#[must_use]
pub fn contains_case_insensitive(
    haystack: &str,
    needle: &str,
    lang: ScriptLanguage,
) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return Some((0, 0));
    }
    let needle_low = to_lower_for_lang(needle, lang);
    let needle_chars: Vec<char> = needle_low.chars().collect();
    if needle_chars.is_empty() {
        return Some((0, 0));
    }

    let hay_indices: Vec<(usize, char)> = haystack.char_indices().collect();
    for start_idx in 0..hay_indices.len() {
        let mut matched = true;
        let mut cur_idx = start_idx;
        for &nc in &needle_chars {
            if cur_idx >= hay_indices.len() {
                matched = false;
                break;
            }
            let hc = hay_indices[cur_idx].1;
            let mut buf = [0u8; 4];
            let hc_low = to_lower_for_lang(hc.encode_utf8(&mut buf), lang);
            if !hc_low.starts_with(nc) {
                matched = false;
                break;
            }
            cur_idx += 1;
        }
        if matched {
            let byte_start = hay_indices[start_idx].0;
            let byte_end = if cur_idx < hay_indices.len() {
                hay_indices[cur_idx].0
            } else {
                haystack.len()
            };
            return Some((byte_start, byte_end));
        }
    }
    None
}

/// Extracts Unicode word tokens `(byte_start, byte_end, word_slice)` from `s`.
///
/// Uses UAX #29 word boundaries so Turkish apostrophe suffixes such as `"Ayşe'nin"` and
/// `"İstanbul'a"` stay intact as single word tokens.
#[must_use]
pub fn word_boundaries(s: &str) -> Vec<(usize, usize, &str)> {
    s.unicode_word_indices()
        .map(|(start, word)| (start, start + word.len(), word))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_case_conversion_preserves_i_pairs() {
        let sample = "İSTANBUL ışık ILIK iğne ÇİĞDEM öğrenci ŞÖFÖR Ğ";
        let upper = to_upper_tr(sample);
        assert_eq!(upper, "İSTANBUL IŞIK ILIK İĞNE ÇİĞDEM ÖĞRENCİ ŞÖFÖR Ğ");
        let lower = to_lower_tr(&upper);
        assert_eq!(lower, "istanbul ışık ılık iğne çiğdem öğrenci şöför ğ");
    }

    #[test]
    fn english_case_conversion_standard() {
        assert_eq!(
            to_upper_for_lang("int. office", ScriptLanguage::En),
            "INT. OFFICE"
        );
        assert_eq!(
            to_lower_for_lang("INT. OFFICE", ScriptLanguage::En),
            "int. office"
        );
    }

    #[test]
    fn turkish_collation_orders_diacritics_correctly() {
        let mut words = vec![
            "şoför",
            "soğuk",
            "çam",
            "cam",
            "ılık",
            "iğne",
            "üzüm",
            "uzak",
            "öğrenci",
            "oda",
            "dağ",
            "dağı",
        ];
        words.sort_by(|a, b| cmp_tr(a, b));
        assert_eq!(
            words,
            vec![
                "cam",
                "çam",
                "dağ",
                "dağı",
                "ılık",
                "iğne",
                "oda",
                "öğrenci",
                "soğuk",
                "şoför",
                "uzak",
                "üzüm"
            ]
        );
    }

    #[test]
    fn case_insensitive_search_respects_turkish_i() {
        let text = "İSTANBUL sokaklarında IŞIK vardı.";
        let hit1 = contains_case_insensitive(text, "istanbul", ScriptLanguage::Tr);
        assert_eq!(hit1, Some((0, "İSTANBUL".len())));

        let hit2 = contains_case_insensitive(text, "ışık", ScriptLanguage::Tr);
        assert!(hit2.is_some());
        let (s, e) = hit2.unwrap();
        assert_eq!(&text[s..e], "IŞIK");

        // In Turkish, "I" should NOT match "i".
        assert_eq!(
            contains_case_insensitive("ILIK", "ilik", ScriptLanguage::Tr),
            None
        );
    }

    #[test]
    fn word_boundaries_keep_turkish_apostrophes_intact() {
        let text = "Ayşe'nin İstanbul'a gitmesi gerekiyor.";
        let words: Vec<&str> = word_boundaries(text)
            .into_iter()
            .map(|(_, _, w)| w)
            .collect();
        assert_eq!(
            words,
            vec!["Ayşe'nin", "İstanbul'a", "gitmesi", "gerekiyor"]
        );
    }
}
