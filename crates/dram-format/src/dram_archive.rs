//! `.dram` versioned ZIP container reader, writer, migration, and crash recovery.

use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use dram_core::{Document, DocumentSettings, Element, Metadata, Note, TitlePage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::FormatError;

/// Current `.dram` container format version.
pub const CURRENT_FORMAT_VERSION: u32 = 1;

const ENTRY_MANIFEST: &str = "manifest.json";
const ENTRY_SCRIPT: &str = "script.json";
const ENTRY_NOTES: &str = "notes.json";
const ENTRY_SETTINGS: &str = "settings.json";

/// Container manifest stored at `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DramManifest {
    /// Container schema version.
    pub format_version: u32,
    /// Document UUID.
    pub document_id: Uuid,
    /// Application version that wrote this archive.
    pub app_version: String,
    /// UTC timestamp when the archive was written.
    pub saved_at: DateTime<Utc>,
    /// Reserved paths of embedded assets (e.g., `assets/cover.png`).
    #[serde(default)]
    pub embedded_assets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScriptPayload {
    metadata: Metadata,
    title_page: TitlePage,
    elements: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotesPayload {
    notes: Vec<Note>,
}

/// Writes `doc` to `path` atomically by writing to a temporary file first and renaming.
pub fn save_dram(path: &Path, doc: &Document) -> Result<(), FormatError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    write_atomic_with(parent, path, |file| write_dram_to_writer(file, doc))
}

/// Atomically writes `bytes` to `path` via a temporary file, `sync_all`, and rename.
pub fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), FormatError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    write_atomic_with(parent, path, |file| {
        use std::io::Write;
        let mut f = file;
        f.write_all(bytes)?;
        Ok(())
    })
}

fn write_atomic_with<F>(parent: &Path, path: &Path, write_fn: F) -> Result<(), FormatError>
where
    F: FnOnce(File) -> Result<(), FormatError>,
{
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project.tmp");
    let tmp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));

    let result = (|| -> Result<(), FormatError> {
        let file = File::create(&tmp_path)?;
        write_fn(file)?;
        // Ensure file contents are flushed to disk before rename.
        let reopened = File::open(&tmp_path)?;
        reopened.sync_all()?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

/// Writes `doc` as a `.dram` ZIP archive into any `Write + Seek` sink.
pub fn write_dram_to_writer<W: Write + Seek>(writer: W, doc: &Document) -> Result<(), FormatError> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored_options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    let embedded_assets: Vec<String> = doc.assets.keys().cloned().collect();
    let manifest = DramManifest {
        format_version: CURRENT_FORMAT_VERSION,
        document_id: doc.id,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        saved_at: Utc::now(),
        embedded_assets,
    };
    let script = ScriptPayload {
        metadata: doc.metadata.clone(),
        title_page: doc.title_page.clone(),
        elements: doc.elements.clone(),
    };
    let notes = NotesPayload {
        notes: doc.notes.clone(),
    };

    write_json_entry(&mut zip, ENTRY_MANIFEST, &manifest, options)?;
    write_json_entry(&mut zip, ENTRY_SCRIPT, &script, options)?;
    write_json_entry(&mut zip, ENTRY_NOTES, &notes, options)?;
    write_json_entry(&mut zip, ENTRY_SETTINGS, &doc.settings, options)?;

    for (asset_path, bytes) in &doc.assets {
        if asset_path.starts_with("assets/") && !asset_path.contains("..") {
            zip.start_file(asset_path, stored_options)?;
            zip.write_all(bytes)?;
        }
    }

    zip.finish()?;
    Ok(())
}

fn write_json_entry<W: Write + Seek, T: Serialize>(
    zip: &mut ZipWriter<W>,
    name: &str,
    value: &T,
    options: SimpleFileOptions,
) -> Result<(), FormatError> {
    zip.start_file(name, options)?;
    serde_json::to_writer_pretty(&mut *zip, value).map_err(|source| FormatError::Json {
        entry: name.to_string(),
        source,
    })?;
    Ok(())
}

/// Loads a [`Document`] from a `.dram` file at `path`.
pub fn load_dram(path: &Path) -> Result<Document, FormatError> {
    let file = File::open(path)?;
    read_dram_from_reader(file)
}

/// Reads a [`Document`] from any `Read + Seek` source containing a `.dram` archive.
pub fn read_dram_from_reader<R: Read + Seek>(reader: R) -> Result<Document, FormatError> {
    let mut archive = ZipArchive::new(reader)?;

    let manifest: DramManifest = read_json_entry(&mut archive, ENTRY_MANIFEST)?;
    if manifest.format_version > CURRENT_FORMAT_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: manifest.format_version,
            max_supported: CURRENT_FORMAT_VERSION,
        });
    }

    let mut script_raw = read_raw_entry(&mut archive, ENTRY_SCRIPT)?;
    let mut notes_raw = read_raw_entry(&mut archive, ENTRY_NOTES)?;
    let mut settings_raw = read_raw_entry(&mut archive, ENTRY_SETTINGS)?;

    // Run version migrations sequentially if reading an older format_version.
    if manifest.format_version < CURRENT_FORMAT_VERSION {
        migrate_payloads(
            manifest.format_version,
            &mut script_raw,
            &mut notes_raw,
            &mut settings_raw,
        )?;
    }

    let script: ScriptPayload =
        serde_json::from_slice(&script_raw).map_err(|source| FormatError::Json {
            entry: ENTRY_SCRIPT.to_string(),
            source,
        })?;
    let notes: NotesPayload =
        serde_json::from_slice(&notes_raw).map_err(|source| FormatError::Json {
            entry: ENTRY_NOTES.to_string(),
            source,
        })?;
    let settings: DocumentSettings =
        serde_json::from_slice(&settings_raw).map_err(|source| FormatError::Json {
            entry: ENTRY_SETTINGS.to_string(),
            source,
        })?;

    const MAX_ASSET_BYTES: u64 = 10 * 1024 * 1024;
    let mut assets = std::collections::BTreeMap::new();
    for asset_path in &manifest.embedded_assets {
        if !asset_path.starts_with("assets/") || asset_path.contains("..") {
            continue;
        }
        if let Ok(entry) = archive.by_name(asset_path) {
            let mut buf = Vec::new();
            if entry.take(MAX_ASSET_BYTES).read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                assets.insert(asset_path.clone(), buf);
            }
        }
    }

    Ok(Document {
        id: manifest.document_id,
        metadata: script.metadata,
        title_page: script.title_page,
        elements: script.elements,
        notes: notes.notes,
        settings,
        assets,
    })
}

/// Maximum uncompressed size allowed for JSON payloads inside `.dram` archives (32 MiB).
pub const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;

/// Reads up to `max_bytes` from `reader`, returning [`FormatError::EntryTooLarge`] if exceeded.
pub(crate) fn read_bounded<R: Read>(
    reader: R,
    entry: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, FormatError> {
    let mut buf = Vec::new();
    let mut limited = reader.take(max_bytes.saturating_add(1));
    limited.read_to_end(&mut buf)?;
    if (buf.len() as u64) > max_bytes {
        return Err(FormatError::EntryTooLarge {
            entry: entry.to_string(),
            max_bytes,
        });
    }
    Ok(buf)
}

fn read_raw_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, FormatError> {
    let entry = match archive.by_name(name) {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => {
            return Err(FormatError::MissingEntry(name.to_string()));
        }
        Err(err) => return Err(FormatError::Zip(err)),
    };
    read_bounded(entry, name, MAX_JSON_BYTES)
}

fn read_json_entry<R: Read + Seek, T: for<'de> Deserialize<'de>>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<T, FormatError> {
    let raw = read_raw_entry(archive, name)?;
    serde_json::from_slice(&raw).map_err(|source| FormatError::Json {
        entry: name.to_string(),
        source,
    })
}

/// Sequential schema migration hook for older `.dram` versions (`v0 -> v1`, etc.).
fn migrate_payloads(
    from_version: u32,
    script_raw: &mut Vec<u8>,
    notes_raw: &mut Vec<u8>,
    settings_raw: &mut Vec<u8>,
) -> Result<(), FormatError> {
    let mut version = from_version;
    while version < CURRENT_FORMAT_VERSION {
        if version == 0 {
            let mut script_val: serde_json::Value =
                serde_json::from_slice(script_raw).map_err(|source| FormatError::Json {
                    entry: ENTRY_SCRIPT.to_string(),
                    source,
                })?;
            if let Some(obj) = script_val.as_object_mut() {
                obj.entry("elements")
                    .or_insert_with(|| serde_json::json!([]));
            }
            *script_raw = serde_json::to_vec(&script_val).map_err(|source| FormatError::Json {
                entry: ENTRY_SCRIPT.to_string(),
                source,
            })?;

            let mut notes_val: serde_json::Value =
                serde_json::from_slice(notes_raw).map_err(|source| FormatError::Json {
                    entry: ENTRY_NOTES.to_string(),
                    source,
                })?;
            if let Some(obj) = notes_val.as_object_mut() {
                obj.entry("notes").or_insert_with(|| serde_json::json!([]));
            }
            *notes_raw = serde_json::to_vec(&notes_val).map_err(|source| FormatError::Json {
                entry: ENTRY_NOTES.to_string(),
                source,
            })?;

            let settings_val: serde_json::Value =
                serde_json::from_slice(settings_raw).map_err(|source| FormatError::Json {
                    entry: ENTRY_SETTINGS.to_string(),
                    source,
                })?;
            *settings_raw =
                serde_json::to_vec(&settings_val).map_err(|source| FormatError::Json {
                    entry: ENTRY_SETTINGS.to_string(),
                    source,
                })?;
        }
        version += 1;
    }
    Ok(())
}

/// Returns the crash-recovery path associated with a `.dram` project file.
#[must_use]
pub fn recovery_path_for(project_path: &Path) -> PathBuf {
    let mut os = project_path.as_os_str().to_os_string();
    os.push(".recovery");
    PathBuf::from(os)
}

/// Writes an atomic crash-recovery snapshot next to `project_path`.
pub fn write_recovery(project_path: &Path, doc: &Document) -> Result<PathBuf, FormatError> {
    let rec = recovery_path_for(project_path);
    save_dram(&rec, doc)?;
    Ok(rec)
}

/// Removes the crash-recovery snapshot for `project_path` if it exists.
pub fn remove_recovery(project_path: &Path) -> Result<(), FormatError> {
    let rec = recovery_path_for(project_path);
    if rec.exists() {
        fs::remove_file(rec)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dram_core::{ElementType, ScriptLanguage};
    use std::io::Cursor;

    #[test]
    fn dram_round_trip_preserves_turkish_content() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.title_page.title = "İSTANBUL GÜNLÜĞÜ".to_string();
        doc.title_page.author = "Çiğdem Şöför".to_string();
        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "İÇ. KAFE - GÜN"),
            Element::new(
                ElementType::Action,
                "İSTANBUL ışık ILIK iğne ÇİĞDEM öğrenci ŞÖFÖR Ğ.",
            ),
            Element::new(ElementType::Character, "ÇİĞDEM"),
            Element::new(ElementType::Dialogue, "Günaydın!"),
        ];

        let mut buf = Cursor::new(Vec::new());
        write_dram_to_writer(&mut buf, &doc).unwrap();
        buf.set_position(0);

        let loaded = read_dram_from_reader(buf).unwrap();
        assert_eq!(loaded.id, doc.id);
        assert_eq!(loaded.title_page, doc.title_page);
        assert_eq!(loaded.elements, doc.elements);
        assert_eq!(loaded.settings, doc.settings);
    }

    #[test]
    fn corrupt_archive_returns_error_without_panic() {
        let corrupt = Cursor::new(b"not a zip archive at all".to_vec());
        let err = read_dram_from_reader(corrupt).unwrap_err();
        assert!(matches!(err, FormatError::Zip(_)));
    }

    #[test]
    fn unsupported_future_version_returns_clear_error() {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            let future_manifest = DramManifest {
                format_version: 999,
                document_id: Uuid::new_v4(),
                app_version: "99.0.0".to_string(),
                saved_at: Utc::now(),
                embedded_assets: Vec::new(),
            };
            write_json_entry(&mut zip, ENTRY_MANIFEST, &future_manifest, options).unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);
        let err = read_dram_from_reader(buf).unwrap_err();
        assert!(matches!(
            err,
            FormatError::UnsupportedVersion { found: 999, .. }
        ));
    }

    #[test]
    fn dram_round_trip_preserves_embedded_assets() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let asset_key = "assets/ref_location.png".to_string();
        let fake_png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
        doc.assets.insert(asset_key.clone(), fake_png.clone());
        doc.notes.push(Note {
            id: Uuid::new_v4(),
            element_id: None,
            title: "Mekan Referansı".to_string(),
            body: "Gece ışıklandırması".to_string(),
            asset_path: Some(asset_key.clone()),
            created_at: Utc::now(),
        });

        let mut buf = Cursor::new(Vec::new());
        write_dram_to_writer(&mut buf, &doc).expect("write");
        buf.set_position(0);
        let loaded = read_dram_from_reader(buf).expect("read");
        assert_eq!(loaded.assets.get(&asset_key), Some(&fake_png));
        assert_eq!(
            loaded.notes[0].asset_path.as_deref(),
            Some(asset_key.as_str())
        );
    }

    #[test]
    fn read_bounded_rejects_oversized_payload() {
        let data = vec![b'a'; 128];
        let err = read_bounded(Cursor::new(data), "script.json", 64).unwrap_err();
        assert!(matches!(
            err,
            FormatError::EntryTooLarge { max_bytes: 64, .. }
        ));
    }
}
