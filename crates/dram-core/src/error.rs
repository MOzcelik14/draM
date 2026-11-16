//! Error types for `dram-core`.

use thiserror::Error;
use uuid::Uuid;

/// Errors produced by core document and editing operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Referenced element ID was not found in the document.
    #[error("element not found: {0}")]
    ElementNotFound(Uuid),

    /// Element index is out of bounds.
    #[error("element index {index} out of bounds (len {len})")]
    IndexOutOfBounds {
        /// Requested index.
        index: usize,
        /// Current number of elements.
        len: usize,
    },

    /// Attempted to undo or redo when the respective stack is empty.
    #[error("nothing to {0}")]
    EmptyHistory(&'static str),
}
