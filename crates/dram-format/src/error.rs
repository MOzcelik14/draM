//! Error types for `dram-format`.

use thiserror::Error;

/// Errors produced when reading, migrating, or writing screenplay files.
#[derive(Debug, Error)]
pub enum FormatError {
    /// Underlying filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Corrupt or invalid ZIP archive.
    #[error("invalid archive container: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Invalid JSON payload inside `.dram` archive.
    #[error("invalid JSON in `{entry}`: {source}")]
    Json {
        /// Archive entry name where parsing failed.
        entry: String,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// Required archive entry is missing.
    #[error("missing required entry `{0}` in .dram archive")]
    MissingEntry(String),

    /// `.dram` format version is newer than this application supports.
    #[error("unsupported .dram format version {found} (max supported: {max_supported})")]
    UnsupportedVersion {
        /// Version found in `manifest.json`.
        found: u32,
        /// Maximum version supported by this build.
        max_supported: u32,
    },

    /// XML parsing or serialization error (e.g., `.fadein` Open Screenplay Format).
    #[error("XML error: {0}")]
    Xml(String),
}
