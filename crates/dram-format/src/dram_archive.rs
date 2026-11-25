//! `.dram` versioned ZIP container reader, writer, migration, and crash recovery.

use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
    /// Monotonic document generation counter for recovery snapshot validation.
    #[serde(default)]
    pub generation: u64,
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
        let sync_handle = file.try_clone()?;
        write_fn(file)?;
        // Ensure file contents are flushed to disk before rename.
        sync_handle.sync_all()?;
        drop(sync_handle);
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
    write_dram_to_writer_with_generation(writer, doc, 0)
}

/// Writes `doc` as a `.dram` ZIP archive with an explicit `generation` marker.
pub fn write_dram_to_writer_with_generation<W: Write + Seek>(
    writer: W,
    doc: &Document,
    generation: u64,
) -> Result<(), FormatError> {
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
        generation,
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

/// Maximum uncompressed size allowed for JSON payloads inside `.dram` archives (32 MiB).
pub const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;
/// Maximum uncompressed size allowed for a single embedded asset (10 MiB).
pub const MAX_ASSET_BYTES: u64 = 10 * 1024 * 1024;
/// Maximum number of embedded assets allowed in a `.dram` archive.
pub const MAX_ASSET_COUNT: usize = 256;
/// Maximum total uncompressed size across all entries in a `.dram` archive (256 MiB).
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;

/// Configurable decompression safety limits for `.dram` archives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    /// Maximum uncompressed size of each JSON entry.
    pub max_json_bytes: u64,
    /// Maximum uncompressed size of each embedded asset.
    pub max_asset_bytes: u64,
    /// Maximum number of embedded assets.
    pub max_asset_count: usize,
    /// Maximum cumulative uncompressed bytes across the entire archive.
    pub max_total_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_json_bytes: MAX_JSON_BYTES,
            max_asset_bytes: MAX_ASSET_BYTES,
            max_asset_count: MAX_ASSET_COUNT,
            max_total_bytes: MAX_TOTAL_UNCOMPRESSED_BYTES,
        }
    }
}

/// Reads only the `manifest.json` header from a `.dram` file at `path`.
pub fn read_dram_manifest(path: &Path) -> Result<DramManifest, FormatError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let (manifest, _) = read_json_entry(&mut archive, ENTRY_MANIFEST, MAX_JSON_BYTES)?;
    Ok(manifest)
}

/// Loads a [`Document`] from a `.dram` file at `path`.
pub fn load_dram(path: &Path) -> Result<Document, FormatError> {
    let file = File::open(path)?;
    read_dram_from_reader(file)
}

/// Reads a [`Document`] from any `Read + Seek` source containing a `.dram` archive.
pub fn read_dram_from_reader<R: Read + Seek>(reader: R) -> Result<Document, FormatError> {
    read_dram_from_reader_with_limits(reader, ArchiveLimits::default())
}

/// Reads a [`Document`] from `reader` enforcing explicit [`ArchiveLimits`].
pub fn read_dram_from_reader_with_limits<R: Read + Seek>(
    reader: R,
    limits: ArchiveLimits,
) -> Result<Document, FormatError> {
    let mut archive = ZipArchive::new(reader)?;
    let mut total_bytes: u64 = 0;

    let (manifest, manifest_len): (DramManifest, u64) =
        read_json_entry(&mut archive, ENTRY_MANIFEST, limits.max_json_bytes)?;
    add_total_bytes(&mut total_bytes, manifest_len, limits.max_total_bytes)?;

    if manifest.format_version > CURRENT_FORMAT_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: manifest.format_version,
            max_supported: CURRENT_FORMAT_VERSION,
        });
    }

    if manifest.embedded_assets.len() > limits.max_asset_count {
        return Err(FormatError::TooManyAssets {
            count: manifest.embedded_assets.len(),
            max_count: limits.max_asset_count,
        });
    }

    let mut script_raw = read_raw_entry(&mut archive, ENTRY_SCRIPT, limits.max_json_bytes)?;
    add_total_bytes(
        &mut total_bytes,
        script_raw.len() as u64,
        limits.max_total_bytes,
    )?;

    let mut notes_raw = read_raw_entry(&mut archive, ENTRY_NOTES, limits.max_json_bytes)?;
    add_total_bytes(
        &mut total_bytes,
        notes_raw.len() as u64,
        limits.max_total_bytes,
    )?;

    let mut settings_raw = read_raw_entry(&mut archive, ENTRY_SETTINGS, limits.max_json_bytes)?;
    add_total_bytes(
        &mut total_bytes,
        settings_raw.len() as u64,
        limits.max_total_bytes,
    )?;

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

    let mut assets = std::collections::BTreeMap::new();
    for asset_path in &manifest.embedded_assets {
        if !asset_path.starts_with("assets/") || asset_path.contains("..") {
            continue;
        }
        if let Ok(entry) = archive.by_name(asset_path) {
            let buf = read_bounded(entry, asset_path, limits.max_asset_bytes)?;
            add_total_bytes(&mut total_bytes, buf.len() as u64, limits.max_total_bytes)?;
            if !buf.is_empty() {
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

fn add_total_bytes(total: &mut u64, added: u64, max_total: u64) -> Result<(), FormatError> {
    *total = total.saturating_add(added);
    if *total > max_total {
        return Err(FormatError::ArchiveTooLarge {
            max_bytes: max_total,
        });
    }
    Ok(())
}

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
    max_bytes: u64,
) -> Result<Vec<u8>, FormatError> {
    let entry = match archive.by_name(name) {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => {
            return Err(FormatError::MissingEntry(name.to_string()));
        }
        Err(err) => return Err(FormatError::Zip(err)),
    };
    read_bounded(entry, name, max_bytes)
}

fn read_json_entry<R: Read + Seek, T: for<'de> Deserialize<'de>>(
    archive: &mut ZipArchive<R>,
    name: &str,
    max_bytes: u64,
) -> Result<(T, u64), FormatError> {
    let raw = read_raw_entry(archive, name, max_bytes)?;
    let len = raw.len() as u64;
    let parsed = serde_json::from_slice(&raw).map_err(|source| FormatError::Json {
        entry: name.to_string(),
        source,
    })?;
    Ok((parsed, len))
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

/// Coordinates crash-recovery snapshot writes against normal saves and app shutdown
/// using monotonic document generation counters and an I/O serialization lock.
#[derive(Debug, Default)]
pub struct RecoveryCoordinator {
    generation: AtomicU64,
    io_lock: Mutex<()>,
}

impl RecoveryCoordinator {
    /// Creates a new coordinator starting at generation `0`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current monotonic generation number.
    #[must_use]
    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Advances the generation number when the document changes or is saved,
    /// invalidating any in-flight snapshots from earlier generations.
    pub fn bump_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Writes a recovery snapshot only if `snapshot_gen` is still the active generation
    /// both before writing and atomically under the I/O lock before renaming.
    ///
    /// Returns `Ok(true)` if the recovery file was committed, or `Ok(false)` if
    /// superseded by a newer edit or normal save.
    pub fn try_write_recovery(
        &self,
        project_path: &Path,
        doc: &Document,
        snapshot_gen: u64,
    ) -> Result<bool, FormatError> {
        if self.generation.load(Ordering::Acquire) != snapshot_gen {
            return Ok(false);
        }

        let rec_path = recovery_path_for(project_path);
        let parent = rec_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let file_name = rec_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recovery.tmp");
        let tmp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));

        let write_res = (|| -> Result<(), FormatError> {
            let file = File::create(&tmp_path)?;
            let sync_handle = file.try_clone()?;
            write_dram_to_writer_with_generation(file, doc, snapshot_gen)?;
            sync_handle.sync_all()?;
            Ok(())
        })();

        if let Err(err) = write_res {
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        }

        let _guard = self.io_lock.lock().unwrap_or_else(|e| e.into_inner());
        if self.generation.load(Ordering::Acquire) != snapshot_gen {
            let _ = fs::remove_file(&tmp_path);
            return Ok(false);
        }

        if let Err(err) = fs::rename(&tmp_path, &rec_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err.into());
        }
        Ok(true)
    }

    /// Invalidates all prior snapshot generations and removes any existing recovery file
    /// under the I/O lock.
    pub fn invalidate_and_remove(&self, project_path: &Path) -> Result<u64, FormatError> {
        let _guard = self.io_lock.lock().unwrap_or_else(|e| e.into_inner());
        let next = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        remove_recovery(project_path)?;
        Ok(next)
    }

    /// Validates that a recovery file on disk is readable and optionally matches `expected_doc_id`.
    #[must_use]
    pub fn validate_recovery_file(rec_path: &Path, expected_doc_id: Option<Uuid>) -> bool {
        let Ok(manifest) = read_dram_manifest(rec_path) else {
            return false;
        };
        if let Some(doc_id) = expected_doc_id {
            if manifest.document_id != doc_id {
                return false;
            }
        }
        manifest.format_version <= CURRENT_FORMAT_VERSION
    }
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
                generation: 0,
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

    #[test]
    fn archive_limits_reject_too_many_assets() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.assets.insert("assets/a.png".to_string(), vec![1, 2, 3]);
        doc.assets.insert("assets/b.png".to_string(), vec![4, 5, 6]);

        let mut buf = Cursor::new(Vec::new());
        write_dram_to_writer(&mut buf, &doc).unwrap();
        buf.set_position(0);

        let limits = ArchiveLimits {
            max_asset_count: 1,
            ..ArchiveLimits::default()
        };
        let err = read_dram_from_reader_with_limits(buf, limits).unwrap_err();
        assert!(matches!(
            err,
            FormatError::TooManyAssets {
                count: 2,
                max_count: 1
            }
        ));
    }

    #[test]
    fn archive_limits_reject_excessive_total_uncompressed_size() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.assets
            .insert("assets/large.bin".to_string(), vec![42u8; 4096]);

        let mut buf = Cursor::new(Vec::new());
        write_dram_to_writer(&mut buf, &doc).unwrap();
        buf.set_position(0);

        let limits = ArchiveLimits {
            max_total_bytes: 2048,
            ..ArchiveLimits::default()
        };
        let err = read_dram_from_reader_with_limits(buf, limits).unwrap_err();
        assert!(matches!(
            err,
            FormatError::ArchiveTooLarge { max_bytes: 2048 }
        ));
    }

    #[test]
    fn recovery_coordinator_rejects_stale_generation_and_races_safely() {
        let tmp_dir = std::env::temp_dir().join(format!("dram_rec_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&tmp_dir).unwrap();
        let project_path = tmp_dir.join("film.dram");
        let rec_path = recovery_path_for(&project_path);

        let coord = std::sync::Arc::new(RecoveryCoordinator::new());
        let doc = Document::new(ScriptLanguage::Tr);

        // Stale generation before write starts is rejected immediately.
        let gen0 = coord.current_generation();
        let gen1 = coord.bump_generation();
        assert_eq!(gen1, gen0 + 1);
        assert!(!coord.try_write_recovery(&project_path, &doc, gen0).unwrap());
        assert!(!rec_path.exists());

        // Current generation writes a valid recovery snapshot with generation metadata.
        assert!(coord.try_write_recovery(&project_path, &doc, gen1).unwrap());
        assert!(rec_path.exists());
        assert!(RecoveryCoordinator::validate_recovery_file(
            &rec_path,
            Some(doc.id)
        ));
        let manifest = read_dram_manifest(&rec_path).unwrap();
        assert_eq!(manifest.generation, gen1);

        // Concurrent autosave vs normal save invalidation: after invalidate_and_remove,
        // no stale snapshot may leave a recovery file behind.
        let coord_bg = std::sync::Arc::clone(&coord);
        let project_bg = project_path.clone();
        let doc_bg = doc.clone();
        let stale_gen = coord.current_generation();
        coord.invalidate_and_remove(&project_path).unwrap();

        let handle = std::thread::spawn(move || {
            let _ = coord_bg.try_write_recovery(&project_bg, &doc_bg, stale_gen);
        });
        handle.join().unwrap();
        assert!(!rec_path.exists());

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
