//! `dram-core`: UI-independent screenplay document model, undo/redo, and Turkish-aware text utilities.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod pagination;
pub mod reports;
pub mod spell;
pub mod templates;
pub mod text;
pub mod undo;

pub use error::CoreError;
pub use model::{
    Document, DocumentSettings, DualDialogueGroup, DualSide, Element, ElementType, InlineNote,
    InlineStyle, Metadata, Note, PageSize, RevisionMark, SceneNumber, ScriptDictionary,
    ScriptLanguage, TextRun, TitlePage,
};
pub use pagination::{
    paginate, LayoutProfile, LineAlign, PaginatedDocument, PaginatedLine, PaginatedPage,
};
pub use reports::{
    breakdown_to_csv, generate_breakdown, CharacterReport, LocationReport, ScriptBreakdown,
};
pub use spell::{BuiltinSpellChecker, NoopSpellChecker, SpellChecker};
pub use templates::{document_from_template, ProjectTemplate};
pub use undo::{EditCommand, UndoManager};
