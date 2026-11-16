//! Fluent-based internationalization (i18n) layer for `dram-app`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

const DEFAULT_TR_FTL: &str = include_str!("../../../locales/tr/main.ftl");
const DEFAULT_EN_FTL: &str = include_str!("../../../locales/en/main.ftl");

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UiLanguage {
    /// Turkish (default).
    #[default]
    Tr,
    /// English (also used as fallback for missing keys).
    En,
}

impl UiLanguage {
    /// Detects the initial UI language from the OS locale, defaulting to Turkish.
    #[must_use]
    pub fn detect_system() -> Self {
        if let Some(locale) = sys_locale::get_locale() {
            let lower = locale.to_ascii_lowercase();
            if lower.starts_with("en") {
                return Self::En;
            }
        }
        Self::Tr
    }

    /// Returns the BCP-47 language code (`"tr"` or `"en"`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Tr => "tr",
            Self::En => "en",
        }
    }
}

/// Manages loaded Fluent bundles and provides panic-free message formatting.
pub struct I18n {
    active: UiLanguage,
    bundles: HashMap<UiLanguage, FluentBundle<FluentResource>>,
}

impl I18n {
    /// Initializes [`I18n`] using `locales/` on disk if present, falling back to embedded `.ftl` files.
    #[must_use]
    pub fn new(initial: UiLanguage) -> Self {
        let mut bundles = HashMap::new();
        bundles.insert(
            UiLanguage::Tr,
            build_bundle("tr", load_ftl_or_default("tr", DEFAULT_TR_FTL)),
        );
        bundles.insert(
            UiLanguage::En,
            build_bundle("en", load_ftl_or_default("en", DEFAULT_EN_FTL)),
        );
        Self {
            active: initial,
            bundles,
        }
    }

    /// Returns the currently active UI language.
    #[must_use]
    pub const fn active(&self) -> UiLanguage {
        self.active
    }

    /// Switches the active UI language immediately without restarting.
    pub fn set_language(&mut self, lang: UiLanguage) {
        self.active = lang;
    }

    /// Translates `key` in the active language, falling back to English if missing.
    #[must_use]
    pub fn t(&self, key: &str) -> String {
        self.format_message(key, None)
    }

    /// Translates `key` with Fluent arguments, falling back to English if missing.
    #[must_use]
    pub fn t_args(&self, key: &str, args: &FluentArgs<'_>) -> String {
        self.format_message(key, Some(args))
    }

    fn format_message(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        if let Some(msg) = try_format(self.bundles.get(&self.active), key, args) {
            return msg;
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[i18n] missing key `{key}` for locale `{}` — falling back to `en`",
            self.active.code()
        );
        if self.active != UiLanguage::En {
            if let Some(msg) = try_format(self.bundles.get(&UiLanguage::En), key, args) {
                return msg;
            }
        }
        key.to_string()
    }
}

fn try_format(
    bundle: Option<&FluentBundle<FluentResource>>,
    key: &str,
    args: Option<&FluentArgs<'_>>,
) -> Option<String> {
    let bundle = bundle?;
    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;
    let mut errors = Vec::new();
    let formatted = bundle.format_pattern(pattern, args, &mut errors);
    Some(formatted.into_owned())
}

fn load_ftl_or_default(lang_code: &str, embedded: &'static str) -> String {
    let disk_path = Path::new("locales").join(lang_code).join("main.ftl");
    fs::read_to_string(disk_path).unwrap_or_else(|_| embedded.to_string())
}

fn build_bundle(lang_code: &str, source: String) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = lang_code.parse().expect("valid static langid");
    let mut bundle = FluentBundle::new(vec![langid]);
    // Disable Unicode bidi isolation marks so strings render cleanly in egui.
    bundle.set_use_isolating(false);
    if let Ok(resource) = FluentResource::try_new(source) {
        let _ = bundle.add_resource(resource);
    }
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_and_number_formatting_works_in_both_languages() {
        let mut i18n = I18n::new(UiLanguage::Tr);
        let mut args1 = FluentArgs::new();
        args1.set("count", 1);
        assert_eq!(i18n.t_args("status-scenes-count", &args1), "1 sahne");

        let mut args12 = FluentArgs::new();
        args12.set("count", 12);
        assert_eq!(i18n.t_args("status-scenes-count", &args12), "12 sahne");

        i18n.set_language(UiLanguage::En);
        assert_eq!(i18n.t_args("status-scenes-count", &args1), "1 scene");
        assert_eq!(i18n.t_args("status-scenes-count", &args12), "12 scenes");
    }

    #[test]
    fn missing_key_does_not_panic_and_falls_back() {
        let i18n = I18n::new(UiLanguage::Tr);
        assert_eq!(i18n.t("non-existent-key"), "non-existent-key");
    }

    #[test]
    fn tr_and_en_bundles_have_matching_message_keys() {
        let extract_keys = |ftl: &str| -> Vec<String> {
            let mut keys: Vec<String> = ftl
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with('#') || trimmed.is_empty() || line.starts_with(' ') {
                        return None;
                    }
                    let (key, _) = line.split_once('=')?;
                    let key = key.trim();
                    if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                        Some(key.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            keys.sort();
            keys
        };

        let tr_keys = extract_keys(DEFAULT_TR_FTL);
        let en_keys = extract_keys(DEFAULT_EN_FTL);
        assert_eq!(tr_keys, en_keys);

        let i18n = I18n::new(UiLanguage::Tr);
        for key in &tr_keys {
            assert!(i18n.bundles[&UiLanguage::Tr].has_message(key));
            assert!(i18n.bundles[&UiLanguage::En].has_message(key));
        }
    }
}
