//! Spellchecker abstraction for Phase 3 pure-Rust Hunspell integration.

use crate::model::ScriptLanguage;

/// Trait for spellcheckers operating on screenplay text.
///
/// Phase 1 provides [`NoopSpellChecker`]; Phase 3 will plug in a permissive-licensed
/// pure-Rust Hunspell-compatible engine (`tr_TR` and `en_US`).
pub trait SpellChecker: Send + Sync {
    /// Returns `true` if `word` is spelled correctly for `lang`.
    fn check_word(&self, word: &str, lang: ScriptLanguage) -> bool;

    /// Returns spelling suggestions for `word` in `lang`.
    fn suggestions(&self, word: &str, lang: ScriptLanguage) -> Vec<String>;

    /// Adds `word` to the user's custom dictionary.
    fn add_to_user_dict(&mut self, word: &str);
}

/// Default no-op spellchecker used before dictionary loading or in headless contexts.
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
