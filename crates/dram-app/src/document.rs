//! Document lifecycle, file I/O, and format import/export handlers.

use std::io::Read;

use std::fs;
use std::path::Path;

use dram_core::{document_from_template, Document, LayoutProfile, ProjectTemplate};
use dram_format::{
    load_dram, load_fadein, load_fdx, parse_fountain, recovery_path_for, save_dram, save_fadein,
    save_fdx, serialize_fountain, write_atomic_bytes, RecoveryCoordinator,
};
use dram_pdf::export_pdf_file;
use fluent_bundle::FluentArgs;

use crate::app::DramApp;
use crate::editor::EditorState;

pub(crate) const MAX_TEXT_BYTES: u64 = 32 * 1024 * 1024;

pub(crate) fn read_text_bounded(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let mut buf = Vec::new();
    let mut limited = file.take(max_bytes.saturating_add(1));
    limited.read_to_end(&mut buf)?;
    if (buf.len() as u64) > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file exceeds maximum allowed size of {max_bytes} bytes"),
        ));
    }
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

impl DramApp {
    pub(crate) fn new_document(&mut self) {
        self.doc = Document::new(self.doc.settings.script_language);
        self.undo.clear();
        self.editor = EditorState::default();
        self.current_path = None;
        self.dirty = false;
        self.pagination_dirty = true;
        self.status_error = None;
    }

    pub(crate) fn new_from_template(&mut self, template: ProjectTemplate) {
        self.doc = document_from_template(template, self.doc.settings.script_language);
        self.undo.clear();
        self.editor = EditorState::default();
        self.current_path = None;
        self.mark_dirty();
        self.status_error = None;
    }

    pub(crate) fn open_path(&mut self, path: &Path) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        match ext.as_deref() {
            Some("fountain") | Some("txt") => match read_text_bounded(path, MAX_TEXT_BYTES) {
                Ok(content) => {
                    self.doc = parse_fountain(&content, self.doc.settings.script_language);
                    self.undo.clear();
                    self.editor = EditorState::default();
                    self.current_path = None;
                    self.mark_dirty();
                    self.status_error = None;
                    self.push_recent_file(path);
                }
                Err(err) => {
                    let mut args = FluentArgs::new();
                    args.set("reason", err.to_string());
                    self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
                }
            },
            Some("fdx") => match load_fdx(path, self.doc.settings.script_language) {
                Ok(loaded) => {
                    self.doc = loaded;
                    self.undo.clear();
                    self.editor = EditorState::default();
                    self.current_path = None;
                    self.mark_dirty();
                    self.status_error = None;
                    self.push_recent_file(path);
                }
                Err(err) => {
                    let mut args = FluentArgs::new();
                    args.set("reason", err.to_string());
                    self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
                }
            },
            Some("fadein") => match load_fadein(path, self.doc.settings.script_language) {
                Ok(loaded) => {
                    self.doc = loaded;
                    self.undo.clear();
                    self.editor = EditorState::default();
                    self.current_path = None;
                    self.mark_dirty();
                    self.status_error = None;
                    self.push_recent_file(path);
                }
                Err(err) => {
                    let mut args = FluentArgs::new();
                    args.set("reason", err.to_string());
                    self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
                }
            },
            _ => match load_dram(path) {
                Ok(loaded) => {
                    let rec = recovery_path_for(path);
                    self.pending_recovery_path = (rec.exists()
                        && RecoveryCoordinator::validate_recovery_file(&rec, Some(loaded.id)))
                    .then_some(rec);
                    self.recovery.bump_generation();
                    self.doc = loaded;
                    self.undo.clear();
                    self.editor = EditorState::default();
                    self.current_path = Some(path.to_path_buf());
                    self.dirty = false;
                    self.pagination_dirty = true;
                    self.status_error = None;
                    self.push_recent_file(path);
                }
                Err(err) => {
                    let mut args = FluentArgs::new();
                    args.set("reason", err.to_string());
                    self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
                }
            },
        }
    }

    pub(crate) fn open_dram(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("draM", &["dram"])
            .pick_file()
        else {
            return;
        };
        self.open_path(&path);
    }

    pub(crate) fn save_current(&mut self) {
        if let Some(path) = self.current_path.clone() {
            self.write_to_path(&path);
        } else {
            self.save_as();
        }
    }

    pub(crate) fn save_as(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("draM", &["dram"])
            .set_file_name("senaryo.dram")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("dram");
        }
        self.write_to_path(&path);
    }

    pub(crate) fn write_to_path(&mut self, path: &Path) {
        match save_dram(path, &self.doc) {
            Ok(()) => {
                let _ = self.recovery.invalidate_and_remove(path);
                let _ = self
                    .recovery
                    .invalidate_and_remove(&std::env::temp_dir().join("dram_unsaved.dram"));
                self.current_path = Some(path.to_path_buf());
                self.dirty = false;
                self.status_error = None;
                self.push_recent_file(path);
            }
            Err(err) => {
                let mut args = FluentArgs::new();
                args.set("reason", err.to_string());
                self.status_error = Some(self.i18n.t_args("error-save-failed", &args));
            }
        }
    }

    pub(crate) fn import_fountain(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Fountain", &["fountain", "txt"])
            .pick_file()
        else {
            return;
        };
        match read_text_bounded(&path, MAX_TEXT_BYTES) {
            Ok(content) => {
                self.doc = parse_fountain(&content, self.doc.settings.script_language);
                self.undo.clear();
                self.editor = EditorState::default();
                self.current_path = None;
                self.mark_dirty();
                self.status_error = None;
            }
            Err(err) => {
                let mut args = FluentArgs::new();
                args.set("reason", err.to_string());
                self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
            }
        }
    }

    pub(crate) fn export_fountain(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("Fountain", &["fountain"])
            .set_file_name("senaryo.fountain")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("fountain");
        }
        let content = serialize_fountain(&self.doc);
        if let Err(err) = write_atomic_bytes(&path, content.as_bytes()) {
            let mut args = FluentArgs::new();
            args.set("reason", err.to_string());
            self.status_error = Some(self.i18n.t_args("error-save-failed", &args));
        } else {
            self.status_error = None;
        }
    }

    pub(crate) fn export_pdf(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("senaryo.pdf")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("pdf");
        }
        let profile = LayoutProfile::for_page_size(self.doc.settings.page_size);
        if let Err(err) = export_pdf_file(&path, &self.doc, &profile) {
            let mut args = FluentArgs::new();
            args.set("reason", err.to_string());
            self.status_error = Some(self.i18n.t_args("error-pdf-failed", &args));
        } else {
            self.status_error = None;
        }
    }

    pub(crate) fn import_fadein(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Fade In", &["fadein"])
            .pick_file()
        else {
            return;
        };
        match load_fadein(&path, self.doc.settings.script_language) {
            Ok(loaded) => {
                self.doc = loaded;
                self.undo.clear();
                self.editor = EditorState::default();
                self.current_path = None;
                self.mark_dirty();
                self.status_error = None;
            }
            Err(err) => {
                let mut args = FluentArgs::new();
                args.set("reason", err.to_string());
                self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
            }
        }
    }

    pub(crate) fn export_fadein(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("Fade In", &["fadein"])
            .set_file_name("senaryo.fadein")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("fadein");
        }
        if let Err(err) = save_fadein(&path, &self.doc) {
            let mut args = FluentArgs::new();
            args.set("reason", err.to_string());
            self.status_error = Some(self.i18n.t_args("error-save-failed", &args));
        } else {
            self.status_error = None;
        }
    }

    pub(crate) fn import_fdx(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Final Draft", &["fdx"])
            .pick_file()
        else {
            return;
        };
        match load_fdx(&path, self.doc.settings.script_language) {
            Ok(loaded) => {
                self.doc = loaded;
                self.undo.clear();
                self.editor = EditorState::default();
                self.current_path = None;
                self.mark_dirty();
                self.status_error = None;
            }
            Err(err) => {
                let mut args = FluentArgs::new();
                args.set("reason", err.to_string());
                self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
            }
        }
    }

    pub(crate) fn export_fdx(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("Final Draft", &["fdx"])
            .set_file_name("senaryo.fdx")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("fdx");
        }
        if let Err(err) = save_fdx(&path, &self.doc) {
            let mut args = FluentArgs::new();
            args.set("reason", err.to_string());
            self.status_error = Some(self.i18n.t_args("error-save-failed", &args));
        } else {
            self.status_error = None;
        }
    }
}
