//! Core screenplay document model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::text::{to_upper_for_lang, to_upper_tr};

/// Screenplay writing language, independent of the UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguage {
    /// Turkish screenplay conventions (`İÇ.`, `DIŞ.`, `(DEVAM)`).
    #[default]
    Tr,
    /// English screenplay conventions (`INT.`, `EXT.`, `(MORE)` / `(CONT'D)`).
    En,
}

impl ScriptLanguage {
    /// Returns the ISO-style short code (`"tr"` or `"en"`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Tr => "tr",
            Self::En => "en",
        }
    }

    /// Returns the other supported script language.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Tr => Self::En,
            Self::En => Self::Tr,
        }
    }
}

/// Physical page size for pagination and PDF export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PageSize {
    /// ISO A4 (210 × 297 mm), default for `draM.`.
    #[default]
    A4,
    /// US Letter (8.5 × 11 in).
    UsLetter,
}

/// Language-specific screenplay vocabulary and markers, customizable per project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptDictionary {
    /// Scene heading prefixes (e.g., `İÇ.`, `DIŞ.`, `İÇ./DIŞ.` or `INT.`, `EXT.`).
    pub scene_prefixes: Vec<String>,
    /// Time-of-day suggestions (e.g., `GÜN`, `GECE` or `DAY`, `NIGHT`).
    pub time_of_day: Vec<String>,
    /// Transition suggestions (e.g., `KESME.`, `KARARMA.` or `CUT TO:`, `FADE OUT.`).
    pub transitions: Vec<String>,
    /// Marker placed at the bottom of a split dialogue block (e.g., `(DEVAM)` or `(MORE)`).
    pub dialogue_more: String,
    /// Marker placed beside the character name after a page split (e.g., `(DEVAM)` or `(CONT'D)`).
    pub dialogue_contd: String,
    /// Title page "Written by" label.
    pub label_written_by: String,
    /// Title page "Draft" label.
    pub label_draft: String,
}

impl ScriptDictionary {
    /// Returns the default dictionary for the given [`ScriptLanguage`].
    #[must_use]
    pub fn default_for(lang: ScriptLanguage) -> Self {
        match lang {
            ScriptLanguage::Tr => Self {
                scene_prefixes: vec![
                    "İÇ.".to_string(),
                    "DIŞ.".to_string(),
                    "İÇ./DIŞ.".to_string(),
                ],
                time_of_day: vec![
                    "GÜN".to_string(),
                    "GECE".to_string(),
                    "ŞAFAK".to_string(),
                    "AKŞAM".to_string(),
                    "ARDINDAN".to_string(),
                    "AYNI ANDA".to_string(),
                ],
                transitions: vec![
                    "KESME.".to_string(),
                    "KARARMA.".to_string(),
                    "ÇAPRAZ GEÇİŞ.".to_string(),
                ],
                dialogue_more: "(DEVAM)".to_string(),
                dialogue_contd: "(DEVAM)".to_string(),
                label_written_by: "Yazan".to_string(),
                label_draft: "Taslak".to_string(),
            },
            ScriptLanguage::En => Self {
                scene_prefixes: vec![
                    "INT.".to_string(),
                    "EXT.".to_string(),
                    "INT./EXT.".to_string(),
                ],
                time_of_day: vec![
                    "DAY".to_string(),
                    "NIGHT".to_string(),
                    "DAWN".to_string(),
                    "DUSK".to_string(),
                    "LATER".to_string(),
                    "CONTINUOUS".to_string(),
                ],
                transitions: vec![
                    "CUT TO:".to_string(),
                    "FADE OUT.".to_string(),
                    "DISSOLVE TO:".to_string(),
                ],
                dialogue_more: "(MORE)".to_string(),
                dialogue_contd: "(CONT'D)".to_string(),
                label_written_by: "Written by".to_string(),
                label_draft: "Draft".to_string(),
            },
        }
    }
}

/// Per-document settings stored in `settings.json` inside `.dram`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSettings {
    /// Script language (independent from UI language).
    pub script_language: ScriptLanguage,
    /// Target page size.
    pub page_size: PageSize,
    /// Font family name (defaults to `"Courier Prime"`).
    pub font_family: String,
    /// Whether scene numbers are displayed/exported.
    pub show_scene_numbers: bool,
    /// Active screenplay vocabulary for this document.
    pub dictionary: ScriptDictionary,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        let script_language = ScriptLanguage::default();
        Self {
            script_language,
            page_size: PageSize::default(),
            font_family: "Courier Prime".to_string(),
            show_scene_numbers: false,
            dictionary: ScriptDictionary::default_for(script_language),
        }
    }
}

impl DocumentSettings {
    /// Switches the script language and resets the dictionary to that language's defaults.
    pub fn set_script_language(&mut self, lang: ScriptLanguage) {
        self.script_language = lang;
        self.dictionary = ScriptDictionary::default_for(lang);
    }

    /// Checks whether `line` starts with a recognized scene heading prefix.
    ///
    /// Prefixes from the active `script_language` are checked first, followed by
    /// the secondary language's prefixes so mixed-language scripts still work.
    #[must_use]
    pub fn is_scene_heading_prefix(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            return false;
        }
        let upper_primary = to_upper_for_lang(trimmed, self.script_language);
        for prefix in &self.dictionary.scene_prefixes {
            let p_upper = to_upper_for_lang(prefix, self.script_language);
            if matches_prefix_boundary(&upper_primary, &p_upper) {
                return true;
            }
        }
        let secondary = self.script_language.other();
        let upper_secondary = to_upper_for_lang(trimmed, secondary);
        for prefix in &ScriptDictionary::default_for(secondary).scene_prefixes {
            let p_upper = to_upper_for_lang(prefix, secondary);
            if matches_prefix_boundary(&upper_secondary, &p_upper) {
                return true;
            }
        }
        // Standard Fountain shorthand `I/E`
        matches_prefix_boundary(&upper_secondary, "I/E")
            || matches_prefix_boundary(&to_upper_tr(trimmed), "İ/D")
    }
}

fn matches_prefix_boundary(text_upper: &str, prefix_upper: &str) -> bool {
    if !text_upper.starts_with(prefix_upper) {
        return false;
    }
    let rest = &text_upper[prefix_upper.len()..];
    rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with('.')
}

/// Title page metadata fields.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TitlePage {
    /// Script title.
    pub title: String,
    /// Credit line (e.g., `"Yazan"` / `"Written by"`).
    pub credit: String,
    /// Author name(s).
    pub author: String,
    /// Source material (e.g., `"Based on the novel by..."`).
    pub source: String,
    /// Draft date or version string.
    pub draft_date: String,
    /// Contact information.
    pub contact: String,
    /// Copyright notice.
    pub copyright: String,
    /// Additional custom key-value pairs preserved from Fountain or user edits.
    pub extra: Vec<(String, String)>,
}

/// Document-level metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Timestamp when the document was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
    /// Version of `draM.` that last saved the document.
    pub app_version: String,
}

impl Default for Metadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Screenplay element types supported by `draM.`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    /// Scene heading / slugline (e.g., `İÇ. ODA - GÜN`).
    SceneHeading,
    /// Action / description paragraph.
    #[default]
    Action,
    /// Character cue.
    Character,
    /// Parenthetical direction inside dialogue.
    Parenthetical,
    /// Spoken dialogue.
    Dialogue,
    /// Scene transition (e.g., `KESME.`, `CUT TO:`).
    Transition,
    /// Camera shot.
    Shot,
    /// General unformatted paragraph.
    General,
    /// Centered text.
    Centered,
    /// Sung lyrics.
    Lyrics,
    /// Explicit page break.
    PageBreak,
    /// Outline section heading with depth (`1..=6`).
    Section(u8),
    /// Scene synopsis (`=`).
    Synopsis,
    /// Standalone note block.
    Note,
}

impl ElementType {
    /// Returns the element type automatically inserted when pressing `Enter` at the end
    /// of an element of this type (Final Draft style flow).
    #[must_use]
    pub const fn next_on_enter(self) -> Self {
        match self {
            Self::SceneHeading => Self::Action,
            Self::Action => Self::Action,
            Self::Character => Self::Dialogue,
            Self::Parenthetical => Self::Dialogue,
            Self::Dialogue => Self::Action,
            Self::Transition => Self::SceneHeading,
            Self::Shot => Self::Action,
            Self::General => Self::General,
            Self::Centered => Self::Action,
            Self::Lyrics => Self::Lyrics,
            Self::PageBreak => Self::SceneHeading,
            Self::Section(_) => Self::Action,
            Self::Synopsis => Self::Action,
            Self::Note => Self::Action,
        }
    }

    /// Returns the next element type when pressing `Tab` on an element.
    #[must_use]
    pub const fn next_on_tab(self) -> Self {
        match self {
            Self::Action => Self::Character,
            Self::Character => Self::Transition,
            Self::Dialogue => Self::Parenthetical,
            Self::Parenthetical => Self::Dialogue,
            Self::Transition => Self::SceneHeading,
            Self::SceneHeading => Self::Action,
            other => other,
        }
    }

    /// Returns `true` if the element's text should be auto-uppercased when rendered/entered.
    #[must_use]
    pub const fn is_uppercase_convention(self) -> bool {
        matches!(
            self,
            Self::SceneHeading | Self::Character | Self::Transition | Self::Shot
        )
    }
}

/// Inline text styling flags (supports any combination of bold, italic, underline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct InlineStyle {
    /// Bold emphasis.
    pub bold: bool,
    /// Italic emphasis.
    pub italic: bool,
    /// Underline emphasis.
    pub underline: bool,
}

impl InlineStyle {
    /// Plain unstyled text.
    pub const PLAIN: Self = Self {
        bold: false,
        italic: false,
        underline: false,
    };

    /// Returns `true` if no styling flags are set.
    #[must_use]
    pub const fn is_plain(self) -> bool {
        !self.bold && !self.italic && !self.underline
    }
}

/// A styled span of text within an [`Element`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRun {
    /// UTF-8 text content of this run.
    pub text: String,
    /// Inline styling for this run.
    #[serde(default)]
    pub style: InlineStyle,
}

impl TextRun {
    /// Creates a plain [`TextRun`].
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: InlineStyle::PLAIN,
        }
    }
}

/// An inline note attached to a character offset within an [`Element`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineNote {
    /// Unique identifier for the inline note.
    pub id: Uuid,
    /// Byte offset in the element's plain text where the note is anchored.
    pub offset: usize,
    /// Note body text.
    pub text: String,
    /// Optional color tag.
    pub color: Option<String>,
}

/// Scene number attached to a [`ElementType::SceneHeading`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneNumber {
    /// Scene number string (e.g., `"1"`, `"12A"`).
    pub value: String,
    /// Whether the scene number is locked against automatic renumbering.
    pub locked: bool,
}

/// Side of a dual dialogue pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DualSide {
    /// Left column of dual dialogue.
    Left,
    /// Right column of dual dialogue.
    Right,
}

/// Dual dialogue grouping metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualDialogueGroup {
    /// Shared group ID linking left and right dialogue blocks.
    pub group_id: Uuid,
    /// Side within the dual dialogue pair.
    pub side: DualSide,
}

/// Revision tracking mark (prepared for Phase 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionMark {
    /// Revision color name (e.g., `"blue"`, `"pink"`).
    pub color: String,
    /// Revision label or date.
    pub label: String,
}

/// A single screenplay element (line/paragraph).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    /// Unique element ID.
    pub id: Uuid,
    /// Screenplay element type.
    pub kind: ElementType,
    /// Styled text runs composing this element.
    pub runs: Vec<TextRun>,
    /// Inline notes anchored in this element.
    #[serde(default)]
    pub inline_notes: Vec<InlineNote>,
    /// Optional scene number (used on `SceneHeading`).
    #[serde(default)]
    pub scene_number: Option<SceneNumber>,
    /// Optional dual dialogue grouping.
    #[serde(default)]
    pub dual_dialogue: Option<DualDialogueGroup>,
    /// Optional revision mark.
    #[serde(default)]
    pub revision: Option<RevisionMark>,
}

impl Element {
    /// Creates a new element of `kind` with plain `text`.
    #[must_use]
    pub fn new(kind: ElementType, text: impl Into<String>) -> Self {
        let text_str = text.into();
        let runs = if text_str.is_empty() {
            Vec::new()
        } else {
            vec![TextRun::plain(text_str)]
        };
        Self {
            id: Uuid::new_v4(),
            kind,
            runs,
            inline_notes: Vec::new(),
            scene_number: None,
            dual_dialogue: None,
            revision: None,
        }
    }

    /// Concatenates all text runs into a plain `String`.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let total: usize = self.runs.iter().map(|r| r.text.len()).sum();
        let mut out = String::with_capacity(total);
        for run in &self.runs {
            out.push_str(&run.text);
        }
        out
    }

    /// Replaces the element's runs with a single plain text run (or empty if `text` is empty).
    pub fn set_plain_text(&mut self, text: impl Into<String>) {
        let text_str = text.into();
        self.runs.clear();
        if !text_str.is_empty() {
            self.runs.push(TextRun::plain(text_str));
        }
    }

    /// Returns `true` if the element has no text content.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.runs.iter().all(|r| r.text.is_empty())
    }
}

/// A standalone document note stored in `notes.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Unique note ID.
    pub id: Uuid,
    /// Optional element ID this note is linked to.
    pub element_id: Option<Uuid>,
    /// Note title.
    pub title: String,
    /// Note body.
    pub body: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Complete screenplay document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Unique document ID.
    pub id: Uuid,
    /// Document metadata.
    pub metadata: Metadata,
    /// Title page fields.
    pub title_page: TitlePage,
    /// Ordered list of screenplay elements.
    pub elements: Vec<Element>,
    /// Standalone document notes.
    pub notes: Vec<Note>,
    /// Per-document settings.
    pub settings: DocumentSettings,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(ScriptLanguage::default())
    }
}

impl Document {
    /// Creates a new empty screenplay document with a single initial `SceneHeading` element.
    #[must_use]
    pub fn new(script_language: ScriptLanguage) -> Self {
        let mut settings = DocumentSettings::default();
        settings.set_script_language(script_language);
        Self {
            id: Uuid::new_v4(),
            metadata: Metadata::default(),
            title_page: TitlePage::default(),
            elements: vec![Element::new(ElementType::SceneHeading, "")],
            notes: Vec::new(),
            settings,
        }
    }

    /// Updates `metadata.updated_at` to the current UTC timestamp.
    pub fn touch(&mut self) {
        self.metadata.updated_at = Utc::now();
    }

    /// Automatically numbers all `SceneHeading` elements while preserving locked scene numbers.
    pub fn renumber_scenes(&mut self) {
        let mut counter: usize = 1;
        for el in &mut self.elements {
            if el.kind == ElementType::SceneHeading {
                if let Some(existing) = &el.scene_number {
                    if existing.locked {
                        if let Ok(parsed) = existing.value.parse::<usize>() {
                            counter = parsed + 1;
                        }
                        continue;
                    }
                }
                el.scene_number = Some(SceneNumber {
                    value: counter.to_string(),
                    locked: false,
                });
                counter += 1;
            }
        }
    }
}
