//! `dram-app`: Desktop GUI for `draM.` screenplay writer.

#![forbid(unsafe_code)]

mod editor;
mod i18n;
mod theme;

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use dram_core::text::{cmp_tr, contains_case_insensitive};
use dram_core::{
    breakdown_to_csv, document_from_template, generate_breakdown, paginate, BuiltinSpellChecker,
    Document, DualDialogueGroup, DualSide, EditCommand, Element, ElementType, LayoutProfile, Note,
    PageSize, PaginatedDocument, ProjectTemplate, RevisionMark, ScriptLanguage, SpellChecker,
    TextRun, UndoManager,
};
use dram_format::{
    load_dram, load_fadein, load_fdx, parse_fountain, recovery_path_for, remove_recovery,
    save_dram, save_fadein, save_fdx, serialize_fountain, write_atomic_bytes, write_recovery,
};
use dram_pdf::export_pdf_file;
use dram_print::{
    list_printers, open_pdf_in_default_viewer, print_document, render_temp_pdf, PrintError,
    PrintOptions,
};
use eframe::egui;
use fluent_bundle::FluentArgs;
use uuid::Uuid;

use crate::editor::{element_type_i18n_key, EditorState, ELEMENT_TYPES};
use crate::i18n::{I18n, UiLanguage};

fn main() -> Result<()> {
    let initial_ui_lang = UiLanguage::detect_system();
    let i18n = I18n::new(initial_ui_lang);
    let title = i18n.t("app-title");
    let initial_file = std::env::args_os().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([720.0, 480.0])
            .with_icon(theme::app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        native_options,
        Box::new(|cc| {
            theme::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(DramApp::new(i18n, initial_file)))
        }),
    )
    .map_err(|err| anyhow::anyhow!("eframe error: {err}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorTab {
    Format,
    Characters,
    Locations,
    Notes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Editor,
    BeatBoard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportTab {
    Characters,
    Locations,
}

struct DramApp {
    doc: Document,
    undo: UndoManager,
    editor: EditorState,
    i18n: I18n,
    current_path: Option<PathBuf>,
    dirty: bool,
    status_error: Option<String>,
    show_title_page_editor: bool,
    show_shortcuts_dialog: bool,
    show_about_dialog: bool,
    show_unsaved_dialog: bool,
    allow_close: bool,
    show_navigator: bool,
    inspector_tab: InspectorTab,
    view_mode: ViewMode,
    revision_mode: bool,
    active_revision_color: String,
    show_reports_dialog: bool,
    report_tab: ReportTab,
    report_status: Option<String>,
    dark_theme: bool,
    applied_dark_theme: Option<bool>,
    focus_mode: bool,
    show_find_replace: bool,
    find_query: String,
    replace_text: String,
    find_case_sensitive: bool,
    find_status: Option<String>,
    show_print_dialog: bool,
    available_printers: Vec<String>,
    print_options: PrintOptions,
    print_page_range_input: String,
    last_print_pdf: Option<PathBuf>,
    print_status: Option<String>,
    cached_pagination: PaginatedDocument,
    pagination_dirty: bool,
    last_autosave: Instant,
    autosave_in_flight: Arc<AtomicBool>,
    pending_recovery_path: Option<PathBuf>,
    recent_files: Vec<PathBuf>,
    spellcheck_enabled: bool,
    spellchecker: BuiltinSpellChecker,
    asset_textures: HashMap<String, egui::TextureHandle>,
    preview_asset_path: Option<String>,
}

impl DramApp {
    fn new(i18n: I18n, initial_file: Option<PathBuf>) -> Self {
        let script_lang = match i18n.active() {
            UiLanguage::Tr => ScriptLanguage::Tr,
            UiLanguage::En => ScriptLanguage::En,
        };
        let doc = Document::new(script_lang);
        let profile = LayoutProfile::for_page_size(doc.settings.page_size);
        let cached_pagination = paginate(&doc, &profile);
        let unsaved_rec = recovery_path_for(&std::env::temp_dir().join("dram_unsaved.dram"));
        let pending_recovery_path = unsaved_rec.exists().then_some(unsaved_rec);
        let recent_files = load_recent_files();
        let mut spellchecker = BuiltinSpellChecker::new();
        spellchecker.load_user_words(&load_user_dict());
        spellchecker.sync_document_terms(&doc);
        let mut app = Self {
            doc,
            undo: UndoManager::new(),
            editor: EditorState::default(),
            i18n,
            current_path: None,
            dirty: false,
            status_error: None,
            show_title_page_editor: false,
            show_shortcuts_dialog: false,
            show_about_dialog: false,
            show_unsaved_dialog: false,
            allow_close: false,
            show_navigator: true,
            inspector_tab: InspectorTab::Format,
            view_mode: ViewMode::Editor,
            revision_mode: false,
            active_revision_color: "blue".to_string(),
            show_reports_dialog: false,
            report_tab: ReportTab::Characters,
            report_status: None,
            dark_theme: true,
            applied_dark_theme: None,
            focus_mode: false,
            show_find_replace: false,
            find_query: String::new(),
            replace_text: String::new(),
            find_case_sensitive: false,
            find_status: None,
            show_print_dialog: false,
            available_printers: Vec::new(),
            print_options: PrintOptions::default(),
            print_page_range_input: String::new(),
            last_print_pdf: None,
            print_status: None,
            cached_pagination,
            pagination_dirty: false,
            last_autosave: Instant::now(),
            autosave_in_flight: Arc::new(AtomicBool::new(false)),
            pending_recovery_path,
            recent_files,
            spellcheck_enabled: true,
            spellchecker,
            asset_textures: HashMap::new(),
            preview_asset_path: None,
        };
        if let Some(path) = initial_file {
            app.open_path(&path);
        }
        if let Ok(view) = std::env::var("DRAM_INITIAL_VIEW") {
            match view.as_str() {
                "beat-board" => app.view_mode = ViewMode::BeatBoard,
                "title-page" => app.show_title_page_editor = true,
                "reports" => app.show_reports_dialog = true,
                _ => {}
            }
        }
        app
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.pagination_dirty = true;
    }

    fn ensure_pagination(&mut self) {
        if self.pagination_dirty {
            let profile = LayoutProfile::for_page_size(self.doc.settings.page_size);
            self.cached_pagination = paginate(&self.doc, &profile);
            self.spellchecker.sync_document_terms(&self.doc);
            self.pagination_dirty = false;
        }
    }

    fn active_recovery_base_path(&self) -> PathBuf {
        self.current_path
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("dram_unsaved.dram"))
    }

    fn push_recent_file(&mut self, path: &Path) {
        let pb = path.to_path_buf();
        self.recent_files.retain(|p| p != &pb);
        self.recent_files.insert(0, pb);
        self.recent_files.truncate(8);
        save_recent_files(&self.recent_files);
    }

    fn clear_recent_files(&mut self) {
        self.recent_files.clear();
        save_recent_files(&self.recent_files);
    }

    fn new_document(&mut self) {
        self.doc = Document::new(self.doc.settings.script_language);
        self.undo.clear();
        self.editor = EditorState::default();
        self.current_path = None;
        self.dirty = false;
        self.pagination_dirty = true;
        self.status_error = None;
    }

    fn new_from_template(&mut self, template: ProjectTemplate) {
        self.doc = document_from_template(template, self.doc.settings.script_language);
        self.undo.clear();
        self.editor = EditorState::default();
        self.current_path = None;
        self.mark_dirty();
        self.status_error = None;
    }

    fn open_path(&mut self, path: &Path) {
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
                    self.pending_recovery_path = rec.exists().then_some(rec);
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

    fn open_dram(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("draM", &["dram"])
            .pick_file()
        else {
            return;
        };
        self.open_path(&path);
    }

    fn save_current(&mut self) {
        if let Some(path) = self.current_path.clone() {
            self.write_to_path(&path);
        } else {
            self.save_as();
        }
    }

    fn save_as(&mut self) {
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

    fn write_to_path(&mut self, path: &Path) {
        match save_dram(path, &self.doc) {
            Ok(()) => {
                let _ = remove_recovery(path);
                let _ = remove_recovery(&std::env::temp_dir().join("dram_unsaved.dram"));
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

    fn import_fountain(&mut self) {
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

    fn export_fountain(&mut self) {
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

    fn export_pdf(&mut self) {
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

    fn import_fadein(&mut self) {
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

    fn export_fadein(&mut self) {
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

    fn open_print_dialog(&mut self) {
        self.available_printers = list_printers();
        self.print_status = None;
        self.last_print_pdf = None;
        self.show_print_dialog = true;
    }

    fn show_print_window(&mut self, ctx: &egui::Context) {
        if !self.show_print_dialog {
            return;
        }
        let mut open = self.show_print_dialog;
        egui::Window::new(self.i18n.t("print-dialog-title"))
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.i18n.t("print-printer"));
                    let selected_label = self
                        .print_options
                        .printer
                        .clone()
                        .unwrap_or_else(|| self.i18n.t("print-default-printer"));
                    egui::ComboBox::from_id_salt("printer_combo")
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    self.print_options.printer.is_none(),
                                    self.i18n.t("print-default-printer"),
                                )
                                .clicked()
                            {
                                self.print_options.printer = None;
                            }
                            for printer in &self.available_printers {
                                let is_sel =
                                    self.print_options.printer.as_deref() == Some(printer.as_str());
                                if ui.selectable_label(is_sel, printer).clicked() {
                                    self.print_options.printer = Some(printer.clone());
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(self.i18n.t("print-copies"));
                    ui.add(egui::DragValue::new(&mut self.print_options.copies).range(1..=99));
                });

                ui.horizontal(|ui| {
                    ui.label(self.i18n.t("print-page-range"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.print_page_range_input)
                            .desired_width(140.0),
                    );
                });

                if let Some(msg) = &self.print_status {
                    ui.label(msg);
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(self.i18n.t("print-btn")).clicked() {
                        self.print_options.page_range =
                            if self.print_page_range_input.trim().is_empty() {
                                None
                            } else {
                                Some(self.print_page_range_input.trim().to_string())
                            };
                        let profile = LayoutProfile::for_page_size(self.doc.settings.page_size);
                        if let Some(old) = self.last_print_pdf.take() {
                            let _ = fs::remove_file(old);
                        }
                        match print_document(&self.doc, &profile, &self.print_options) {
                            Ok(pdf_path) => {
                                self.last_print_pdf = Some(pdf_path);
                                self.print_status = Some(self.i18n.t("print-success"));
                            }
                            Err(PrintError::PrintCommand { message, pdf_path }) => {
                                self.last_print_pdf = Some(pdf_path);
                                self.print_status = Some(message);
                            }
                            Err(err) => {
                                self.print_status = Some(err.to_string());
                            }
                        }
                    }

                    if ui.button(self.i18n.t("print-open-viewer")).clicked() {
                        let profile = LayoutProfile::for_page_size(self.doc.settings.page_size);
                        let path_res = if let Some(existing) =
                            self.last_print_pdf.as_ref().filter(|p| p.exists())
                        {
                            Ok(existing.clone())
                        } else {
                            let res = render_temp_pdf(&self.doc, &profile);
                            if let Ok(ref p) = res {
                                self.last_print_pdf = Some(p.clone());
                            }
                            res
                        };
                        match path_res {
                            Ok(path) => {
                                if let Err(err) = open_pdf_in_default_viewer(&path) {
                                    self.print_status = Some(err.to_string());
                                }
                            }
                            Err(err) => self.print_status = Some(err.to_string()),
                        }
                    }

                    if ui.button(self.i18n.t("btn-close")).clicked() {
                        self.show_print_dialog = false;
                        if let Some(old) = self.last_print_pdf.take() {
                            let _ = fs::remove_file(old);
                        }
                    }
                });
            });
        if !open {
            self.show_print_dialog = false;
            if let Some(old) = self.last_print_pdf.take() {
                let _ = fs::remove_file(old);
            }
        }
    }

    fn show_find_replace_window(&mut self, ctx: &egui::Context) {
        if !self.show_find_replace {
            return;
        }
        let mut open = self.show_find_replace;
        egui::Window::new(self.i18n.t("menu-find-replace"))
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                egui::Grid::new("find_replace_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(self.i18n.t("find-label"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.find_query).desired_width(220.0),
                        );
                        ui.end_row();

                        ui.label(self.i18n.t("replace-label"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.replace_text).desired_width(220.0),
                        );
                        ui.end_row();
                    });

                ui.checkbox(
                    &mut self.find_case_sensitive,
                    self.i18n.t("find-case-sensitive"),
                );

                if let Some(status) = &self.find_status {
                    ui.label(status);
                }

                ui.horizontal(|ui| {
                    if ui.button(self.i18n.t("find-next-btn")).clicked() {
                        self.find_next();
                    }
                    if ui.button(self.i18n.t("replace-btn")).clicked() {
                        self.replace_current();
                    }
                    if ui.button(self.i18n.t("replace-all-btn")).clicked() {
                        self.replace_all();
                    }
                    if ui.button(self.i18n.t("btn-close")).clicked() {
                        self.show_find_replace = false;
                    }
                });
            });
        if !open {
            self.show_find_replace = false;
        }
    }

    fn find_match_in(&self, text: &str) -> Option<(usize, usize)> {
        if self.find_query.is_empty() {
            return None;
        }
        if self.find_case_sensitive {
            text.find(&self.find_query)
                .map(|s| (s, s + self.find_query.len()))
        } else {
            contains_case_insensitive(text, &self.find_query, self.doc.settings.script_language)
        }
    }

    fn find_next(&mut self) {
        if self.find_query.is_empty() || self.doc.elements.is_empty() {
            return;
        }
        let len = self.doc.elements.len();
        let start = (self.editor.focused_index + 1) % len;
        for offset in 0..len {
            let idx = (start + offset) % len;
            let text = self.doc.elements[idx].plain_text();
            if self.find_match_in(&text).is_some() {
                self.editor.focused_index = idx;
                self.editor.pending_focus = Some(idx);
                self.find_status = None;
                return;
            }
        }
        self.find_status = Some(self.i18n.t("find-no-match"));
    }

    fn replace_current(&mut self) {
        if self.find_query.is_empty() {
            return;
        }
        let idx = self.editor.focused_index;
        if let Some(el) = self.doc.elements.get(idx) {
            let text = el.plain_text();
            if let Some((start, end)) = self.find_match_in(&text) {
                let mut updated = String::with_capacity(text.len());
                updated.push_str(&text[..start]);
                updated.push_str(&self.replace_text);
                updated.push_str(&text[end..]);
                let old_runs = el.runs.clone();
                let new_runs = if updated.is_empty() {
                    Vec::new()
                } else {
                    vec![TextRun::plain(updated)]
                };
                self.undo.break_coalescing();
                if self
                    .undo
                    .execute(
                        &mut self.doc,
                        EditCommand::UpdateRuns {
                            index: idx,
                            old_runs,
                            new_runs,
                        },
                    )
                    .is_ok()
                {
                    self.mark_dirty();
                    self.find_next();
                }
                return;
            }
        }
        self.find_next();
    }

    fn replace_all(&mut self) {
        if self.find_query.is_empty() {
            return;
        }
        let mut cmds = Vec::new();
        for (idx, el) in self.doc.elements.iter().enumerate() {
            let mut text = el.plain_text();
            let mut changed = false;
            let mut search_from = 0;
            while search_from <= text.len() {
                let Some((rel_s, rel_e)) = self.find_match_in(&text[search_from..]) else {
                    break;
                };
                let abs_s = search_from + rel_s;
                let abs_e = search_from + rel_e;
                text.replace_range(abs_s..abs_e, &self.replace_text);
                search_from = abs_s + self.replace_text.len();
                changed = true;
                if rel_s == rel_e {
                    break;
                }
            }
            if changed {
                let new_runs = if text.is_empty() {
                    Vec::new()
                } else {
                    vec![TextRun::plain(text)]
                };
                cmds.push(EditCommand::UpdateRuns {
                    index: idx,
                    old_runs: el.runs.clone(),
                    new_runs,
                });
            }
        }
        if !cmds.is_empty() {
            self.undo.break_coalescing();
            if self
                .undo
                .execute(&mut self.doc, EditCommand::Batch(cmds))
                .is_ok()
            {
                self.mark_dirty();
                self.find_status = None;
            }
        } else {
            self.find_status = Some(self.i18n.t("find-no-match"));
        }
    }

    fn show_right_dock(&mut self, ctx: &egui::Context) {
        if !self.show_navigator {
            return;
        }
        self.ensure_pagination();
        let total_pages = self.cached_pagination.page_count();

        let dock_frame =
            egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::same(10));

        egui::SidePanel::right("fadein_dock")
            .frame(dock_frame)
            .resizable(true)
            .default_width(330.0)
            .min_width(305.0)
            .show(ctx, |ui| {
                // Top section: single-line 4-tab bar (Format | Characters | Locations | Notes)
                let tabs = [
                    (InspectorTab::Format, "tab-format"),
                    (InspectorTab::Characters, "tab-characters"),
                    (InspectorTab::Locations, "tab-locations"),
                    (InspectorTab::Notes, "tab-notes"),
                ];
                ui.horizontal(|ui| {
                    let gap = 5.0_f32;
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.spacing_mut().button_padding = egui::vec2(6.0, 5.0);
                    let total_w = ui.available_width();
                    let usable = (total_w - gap * 3.0).max(200.0);
                    let labels: Vec<String> =
                        tabs.iter().map(|(_, key)| self.i18n.t(key)).collect();
                    let weights: Vec<f32> = labels
                        .iter()
                        .map(|l| l.chars().count() as f32 + 3.0)
                        .collect();
                    let weight_sum: f32 = weights.iter().sum();
                    let mut used_w = 0.0_f32;

                    for (i, ((tab, _), label)) in tabs.iter().zip(labels.iter()).enumerate() {
                        let w = if i + 1 == tabs.len() {
                            (usable - used_w).max(40.0)
                        } else {
                            let part = (usable * (weights[i] / weight_sum)).floor();
                            used_w += part;
                            part
                        };
                        let active = self.inspector_tab == *tab;
                        let mut text = egui::RichText::new(label).size(12.5);
                        if active {
                            text = text.strong().color(egui::Color32::WHITE);
                        }
                        let btn = egui::Button::new(text)
                            .wrap_mode(egui::TextWrapMode::Extend)
                            .min_size(egui::vec2(w, 28.0))
                            .selected(active);
                        if ui.add(btn).clicked() {
                            self.inspector_tab = *tab;
                        }
                    }
                });
                ui.add_space(6.0);

                let top_h = (ui.available_height() * 0.42).clamp(210.0, 300.0);
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_min_height(top_h);
                        egui::ScrollArea::vertical()
                            .id_salt("inspector_scroll")
                            .min_scrolled_height(top_h)
                            .max_height(top_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.inspector_tab {
                                InspectorTab::Format => self.render_format_tab(ui),
                                InspectorTab::Characters => self.render_characters_tab(ui),
                                InspectorTab::Locations => self.render_locations_tab(ui),
                                InspectorTab::Notes => self.render_notes_tab(ui),
                            });
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // Bottom section: Fade In-style Scene Navigator
                ui.horizontal(|ui| {
                    ui.strong(self.i18n.t("nav-heading"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut scene_args = FluentArgs::new();
                        let count = self
                            .doc
                            .elements
                            .iter()
                            .filter(|e| e.kind == ElementType::SceneHeading)
                            .count();
                        scene_args.set("count", count);
                        ui.label(
                            egui::RichText::new(
                                self.i18n.t_args("status-scenes-count", &scene_args),
                            )
                            .small()
                            .weak(),
                        );
                    });
                });
                ui.add_space(4.0);

                // Collect scene ranges + first action snippet + page number
                let mut scenes: Vec<(usize, usize, String, String, usize)> = Vec::new();
                for (idx, el) in self.doc.elements.iter().enumerate() {
                    if el.kind == ElementType::SceneHeading {
                        if let Some(prev) = scenes.last_mut() {
                            prev.1 = idx;
                        }
                        let num = el
                            .scene_number
                            .as_ref()
                            .map(|n| format!("{}. ", n.value))
                            .unwrap_or_default();
                        let heading = el.plain_text();
                        let label = if heading.trim().is_empty() {
                            format!("{num}...")
                        } else {
                            format!("{num}{heading}")
                        };
                        let page_num = element_page_number(&self.cached_pagination, idx);
                        scenes.push((idx, self.doc.elements.len(), label, String::new(), page_num));
                    } else if el.kind == ElementType::Action {
                        if let Some(cur) = scenes.last_mut() {
                            if cur.3.is_empty() {
                                cur.3 = el.plain_text();
                            }
                        }
                    }
                }

                let mut move_req: Option<(usize, usize)> = None;
                let mut active_page = 1usize;

                let nav_h = (ui.available_height() - 26.0).max(80.0);
                egui::ScrollArea::vertical()
                    .id_salt("navigator_scroll")
                    .max_height(nav_h)
                    .show(ui, |ui| {
                        for (s_i, (start, _end, label, snippet, page_num)) in
                            scenes.iter().enumerate()
                        {
                            let is_current = self.editor.focused_index >= *start
                                && scenes
                                    .get(s_i + 1)
                                    .is_none_or(|next| self.editor.focused_index < next.0);
                            if is_current {
                                active_page = *page_num;
                            }

                            let card_fill = if is_current {
                                theme::ACCENT_CRIMSON_DARK
                            } else {
                                ui.visuals().faint_bg_color
                            };

                            let resp = egui::Frame::new()
                                .fill(card_fill)
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let title_color = if is_current {
                                            egui::Color32::WHITE
                                        } else {
                                            ui.visuals().text_color()
                                        };
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new(label)
                                                        .strong()
                                                        .size(13.0)
                                                        .color(title_color),
                                                )
                                                .frame(false),
                                            )
                                            .clicked()
                                        {
                                            self.editor.focused_index = *start;
                                            self.editor.pending_focus = Some(*start);
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if s_i + 1 < scenes.len()
                                                    && ui
                                                        .small_button("v")
                                                        .on_hover_text(self.i18n.t("nav-move-down"))
                                                        .clicked()
                                                {
                                                    move_req = Some((s_i, s_i + 1));
                                                }
                                                if s_i > 0
                                                    && ui
                                                        .small_button("^")
                                                        .on_hover_text(self.i18n.t("nav-move-up"))
                                                        .clicked()
                                                {
                                                    move_req = Some((s_i, s_i - 1));
                                                }
                                            },
                                        );
                                    });

                                    let mut page_args = FluentArgs::new();
                                    page_args.set("current", *page_num);
                                    page_args.set("total", total_pages);
                                    let sub_color = if is_current {
                                        egui::Color32::from_rgb(240, 222, 222)
                                    } else {
                                        ui.visuals().weak_text_color()
                                    };
                                    ui.label(
                                        egui::RichText::new(
                                            self.i18n.t_args("page-x-of-y", &page_args),
                                        )
                                        .size(11.5)
                                        .color(sub_color),
                                    );
                                    if !snippet.trim().is_empty() {
                                        let truncated: String =
                                            snippet.trim().chars().take(54).collect();
                                        ui.label(
                                            egui::RichText::new(truncated)
                                                .size(11.5)
                                                .color(sub_color),
                                        );
                                    }
                                });
                            if resp.response.clicked() {
                                self.editor.focused_index = *start;
                                self.editor.pending_focus = Some(*start);
                            }
                            ui.add_space(4.0);
                        }
                    });

                ui.separator();
                let mut page_args = FluentArgs::new();
                page_args.set("current", active_page);
                page_args.set("total", total_pages);
                ui.label(
                    egui::RichText::new(self.i18n.t_args("page-x-of-y", &page_args))
                        .small()
                        .weak(),
                );

                if let Some((from_s, to_s)) = move_req {
                    let simple_scenes: Vec<(usize, usize, String)> = scenes
                        .into_iter()
                        .map(|(s, e, l, _, _)| (s, e, l))
                        .collect();
                    self.swap_scenes(&simple_scenes, from_s, to_s);
                }
            });
    }

    fn render_format_tab(&mut self, ui: &mut egui::Ui) {
        let current_kind = self
            .doc
            .elements
            .get(self.editor.focused_index)
            .map_or(ElementType::Action, |e| e.kind);

        for (idx, &kind) in ELEMENT_TYPES.iter().enumerate() {
            let is_active = current_kind == kind;
            let row_fill = if is_active {
                theme::ACCENT_CRIMSON
            } else {
                egui::Color32::TRANSPARENT
            };
            let text_color = if is_active {
                egui::Color32::WHITE
            } else {
                ui.visuals().text_color()
            };
            let num_color = if is_active {
                egui::Color32::from_rgb(245, 220, 220)
            } else {
                ui.visuals().weak_text_color()
            };

            let resp = egui::Frame::new()
                .fill(row_fill)
                .corner_radius(egui::CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.set_width((ui.available_width() - 4.0).max(80.0));
                    ui.spacing_mut().interact_size.y = 18.0;
                    ui.horizontal(|ui| {
                        let shortcut = if idx < 8 {
                            format!("{}", idx + 1)
                        } else {
                            String::new()
                        };
                        ui.add_sized(
                            egui::vec2(18.0, 18.0),
                            egui::Label::new(
                                egui::RichText::new(shortcut)
                                    .monospace()
                                    .size(12.5)
                                    .color(num_color),
                            ),
                        );
                        let mut label =
                            egui::RichText::new(self.i18n.t(element_type_i18n_key(kind)))
                                .size(13.5)
                                .color(text_color);
                        if is_active {
                            label = label.strong();
                        }
                        ui.label(label);
                    });
                })
                .response
                .interact(egui::Sense::click());

            if resp.clicked()
                && !is_active
                && self
                    .undo
                    .execute(
                        &mut self.doc,
                        EditCommand::ChangeElementType {
                            index: self.editor.focused_index,
                            old_kind: current_kind,
                            new_kind: kind,
                        },
                    )
                    .is_ok()
            {
                self.mark_dirty();
                self.editor.pending_focus = Some(self.editor.focused_index);
            }
        }
    }

    fn render_characters_tab(&mut self, ui: &mut egui::Ui) {
        let mut chars: Vec<String> = Vec::new();
        for el in &self.doc.elements {
            if el.kind == ElementType::Character {
                let t = el.plain_text().trim().to_string();
                if !t.is_empty() && !chars.contains(&t) {
                    chars.push(t);
                }
            }
        }
        chars.sort_by(|a, b| cmp_tr(a, b));
        if chars.is_empty() {
            ui.label(
                egui::RichText::new(self.i18n.t("empty-list"))
                    .weak()
                    .small(),
            );
            return;
        }
        for name in chars {
            if ui.selectable_label(false, &name).clicked() {
                let idx = self.editor.focused_index;
                if let Some(el) = self.doc.elements.get(idx) {
                    let old_runs = el.runs.clone();
                    if self
                        .undo
                        .execute(
                            &mut self.doc,
                            EditCommand::UpdateRuns {
                                index: idx,
                                old_runs,
                                new_runs: vec![TextRun::plain(name)],
                            },
                        )
                        .is_ok()
                    {
                        self.mark_dirty();
                        self.editor.pending_focus = Some(idx);
                    }
                }
            }
        }
    }

    fn render_locations_tab(&mut self, ui: &mut egui::Ui) {
        let mut locs: Vec<(usize, String)> = Vec::new();
        for (i, el) in self.doc.elements.iter().enumerate() {
            if el.kind == ElementType::SceneHeading {
                let t = el.plain_text().trim().to_string();
                if !t.is_empty() && !locs.iter().any(|(_, existing)| existing == &t) {
                    locs.push((i, t));
                }
            }
        }
        locs.sort_by(|a, b| cmp_tr(&a.1, &b.1));
        if locs.is_empty() {
            ui.label(
                egui::RichText::new(self.i18n.t("empty-list"))
                    .weak()
                    .small(),
            );
            return;
        }
        for (idx, loc) in locs {
            if ui.selectable_label(false, &loc).clicked() {
                self.editor.focused_index = idx;
                self.editor.pending_focus = Some(idx);
            }
        }
    }

    fn ensure_asset_texture(
        &mut self,
        ctx: &egui::Context,
        asset_path: &str,
    ) -> Option<egui::TextureId> {
        if let Some(tex) = self.asset_textures.get(asset_path) {
            return Some(tex.id());
        }
        let bytes = self.doc.assets.get(asset_path)?;
        let img = image::load_from_memory(bytes).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        let handle = ctx.load_texture(
            format!("dram_asset_{asset_path}"),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        let id = handle.id();
        self.asset_textures.insert(asset_path.to_string(), handle);
        Some(id)
    }

    fn show_asset_preview_window(&mut self, ctx: &egui::Context) {
        let Some(asset_path) = self.preview_asset_path.clone() else {
            return;
        };
        let _ = self.ensure_asset_texture(ctx, &asset_path);
        let mut open = true;
        egui::Window::new(self.i18n.t("notes-preview-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(540.0)
            .show(ctx, |ui| {
                if let Some(tex) = self.asset_textures.get(&asset_path) {
                    let orig = tex.size_vec2();
                    let max_w = ui.available_width().clamp(240.0, 760.0);
                    let scale = (max_w / orig.x.max(1.0))
                        .min(520.0 / orig.y.max(1.0))
                        .min(1.0);
                    ui.image((tex.id(), orig * scale));
                }
            });
        if !open {
            self.preview_asset_path = None;
        }
    }

    fn render_notes_tab(&mut self, ui: &mut egui::Ui) {
        if ui.small_button(self.i18n.t("notes-add")).clicked() {
            self.doc.notes.push(Note {
                id: Uuid::new_v4(),
                element_id: self
                    .doc
                    .elements
                    .get(self.editor.focused_index)
                    .map(|e| e.id),
                title: String::new(),
                body: String::new(),
                asset_path: None,
                created_at: Utc::now(),
            });
            self.mark_dirty();
        }
        ui.add_space(6.0);

        // Preload textures for all attached note images.
        let paths_to_load: Vec<String> = self
            .doc
            .notes
            .iter()
            .filter_map(|n| n.asset_path.clone())
            .collect();
        for path in &paths_to_load {
            let _ = self.ensure_asset_texture(ui.ctx(), path);
        }

        let mut delete_idx: Option<usize> = None;
        let mut attach_idx: Option<usize> = None;
        let mut remove_img_idx: Option<usize> = None;
        let mut preview_req: Option<String> = None;
        let mut notes_changed = false;
        for (idx, note) in self.doc.notes.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut note.title)
                                .hint_text(self.i18n.t("notes-title-placeholder"))
                                .desired_width(170.0),
                        )
                        .changed()
                    {
                        notes_changed = true;
                    }
                    if ui.small_button(self.i18n.t("notes-delete")).clicked() {
                        delete_idx = Some(idx);
                    }
                });
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut note.body)
                            .hint_text(self.i18n.t("notes-body-placeholder"))
                            .desired_rows(2)
                            .desired_width(220.0),
                    )
                    .changed()
                {
                    notes_changed = true;
                }

                if let Some(ref asset_path) = note.asset_path {
                    if let Some(tex) = self.asset_textures.get(asset_path) {
                        ui.add_space(4.0);
                        let orig = tex.size_vec2();
                        let scale = (210.0 / orig.x.max(1.0))
                            .min(120.0 / orig.y.max(1.0))
                            .min(1.0);
                        let img_resp = ui.add(
                            egui::Image::new((tex.id(), orig * scale)).sense(egui::Sense::click()),
                        );
                        if img_resp.clicked() {
                            preview_req = Some(asset_path.clone());
                        }
                    }
                    if ui.small_button(self.i18n.t("notes-remove-image")).clicked() {
                        remove_img_idx = Some(idx);
                    }
                } else if ui.small_button(self.i18n.t("notes-attach-image")).clicked() {
                    attach_idx = Some(idx);
                }
            });
            ui.add_space(4.0);
        }
        if notes_changed {
            self.mark_dirty();
        }
        if let Some(path) = preview_req {
            self.preview_asset_path = Some(path);
        }
        if let Some(idx) = remove_img_idx {
            if let Some(old_path) = self.doc.notes[idx].asset_path.take() {
                let still_used = self
                    .doc
                    .notes
                    .iter()
                    .any(|n| n.asset_path.as_deref() == Some(old_path.as_str()));
                if !still_used {
                    self.doc.assets.remove(&old_path);
                    self.asset_textures.remove(&old_path);
                }
                self.mark_dirty();
            }
        }
        if let Some(idx) = attach_idx {
            if let Some(file_path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
                match fs::read(&file_path) {
                    Ok(bytes) if !bytes.is_empty() && bytes.len() <= MAX_IMAGE_BYTES => {
                        let ext = file_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.to_ascii_lowercase())
                            .unwrap_or_else(|| "png".to_string());
                        let key = format!("assets/{}.{ext}", Uuid::new_v4());
                        self.doc.assets.insert(key.clone(), bytes);
                        self.doc.notes[idx].asset_path = Some(key);
                        self.mark_dirty();
                    }
                    Ok(_) => {}
                    Err(err) => {
                        let mut args = FluentArgs::new();
                        args.set("reason", err.to_string());
                        self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
                    }
                }
            }
        }
        if let Some(i) = delete_idx {
            let removed = self.doc.notes.remove(i);
            if let Some(old_path) = removed.asset_path {
                let still_used = self
                    .doc
                    .notes
                    .iter()
                    .any(|n| n.asset_path.as_deref() == Some(old_path.as_str()));
                if !still_used {
                    self.doc.assets.remove(&old_path);
                    self.asset_textures.remove(&old_path);
                }
            }
            self.mark_dirty();
        }
    }

    fn swap_scenes(&mut self, scenes: &[(usize, usize, String)], from_s: usize, to_s: usize) {
        let (first_s, second_s) = if from_s < to_s {
            (from_s, to_s)
        } else {
            (to_s, from_s)
        };
        let (s1, e1, _) = scenes[first_s];
        let (s2, e2, _) = scenes[second_s];
        if e1 != s2 {
            return;
        }
        // Move elements [s2..e2] before s1 one by one so UndoManager can revert cleanly.
        let count2 = e2 - s2;
        let mut cmds = Vec::with_capacity(count2);
        for offset in 0..count2 {
            cmds.push(EditCommand::MoveElement {
                from: s2 + offset,
                to: s1 + offset,
            });
        }
        self.undo.break_coalescing();
        if self
            .undo
            .execute(&mut self.doc, EditCommand::Batch(cmds))
            .is_ok()
        {
            if self.doc.settings.show_scene_numbers {
                self.doc.renumber_scenes();
            }
            self.mark_dirty();
        }
    }

    fn show_title_page_window(&mut self, ctx: &egui::Context) {
        if !self.show_title_page_editor {
            return;
        }
        let mut open = self.show_title_page_editor;
        let palette = theme::paper_palette(self.dark_theme);
        egui::Window::new(self.i18n.t("title-page-heading"))
            .open(&mut open)
            .resizable(true)
            .default_width(720.0)
            .show(ctx, |ui| {
                let mut tp_changed = false;
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        egui::Grid::new("title_page_grid")
                            .num_columns(2)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                let tp = &mut self.doc.title_page;
                                let mut row =
                                    |label_key: &str, val: &mut String, multiline: bool| {
                                        ui.label(self.i18n.t(label_key));
                                        let changed = if multiline {
                                            ui.add(
                                                egui::TextEdit::multiline(val)
                                                    .desired_rows(2)
                                                    .desired_width(240.0),
                                            )
                                            .changed()
                                        } else {
                                            ui.add(
                                                egui::TextEdit::singleline(val)
                                                    .desired_width(240.0),
                                            )
                                            .changed()
                                        };
                                        if changed {
                                            tp_changed = true;
                                        }
                                        ui.end_row();
                                    };
                                row("tp-title", &mut tp.title, false);
                                row("tp-credit", &mut tp.credit, false);
                                row("tp-author", &mut tp.author, false);
                                row("tp-source", &mut tp.source, false);
                                row("tp-draft-date", &mut tp.draft_date, false);
                                row("tp-contact", &mut tp.contact, true);
                                row("tp-copyright", &mut tp.copyright, false);
                            });
                    });

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(self.i18n.t("tp-preview-heading"))
                                .small()
                                .weak(),
                        );
                        ui.add_space(4.0);
                        let tp = &self.doc.title_page;
                        egui::Frame::new()
                            .fill(palette.paper_bg)
                            .stroke(egui::Stroke::new(1.0_f32, palette.paper_border))
                            .corner_radius(egui::CornerRadius::same(4))
                            .inner_margin(egui::Margin::same(16))
                            .show(ui, |ui| {
                                ui.set_width(250.0);
                                ui.set_min_height(320.0);
                                ui.vertical_centered(|ui| {
                                    ui.add_space(56.0);
                                    if !tp.title.trim().is_empty() {
                                        ui.label(
                                            egui::RichText::new(tp.title.trim())
                                                .monospace()
                                                .strong()
                                                .size(14.0)
                                                .color(palette.paper_ink),
                                        );
                                        ui.add_space(10.0);
                                    }
                                    let credit = if tp.credit.trim().is_empty()
                                        && !tp.author.trim().is_empty()
                                    {
                                        self.doc.settings.dictionary.label_written_by.as_str()
                                    } else {
                                        tp.credit.trim()
                                    };
                                    if !credit.is_empty() {
                                        ui.label(
                                            egui::RichText::new(credit)
                                                .monospace()
                                                .size(11.0)
                                                .color(palette.paper_ink),
                                        );
                                        ui.add_space(6.0);
                                    }
                                    if !tp.author.trim().is_empty() {
                                        ui.label(
                                            egui::RichText::new(tp.author.trim())
                                                .monospace()
                                                .size(11.5)
                                                .color(palette.paper_ink),
                                        );
                                        ui.add_space(8.0);
                                    }
                                    if !tp.source.trim().is_empty() {
                                        ui.label(
                                            egui::RichText::new(tp.source.trim())
                                                .monospace()
                                                .italics()
                                                .size(10.5)
                                                .color(palette.muted_text),
                                        );
                                    }
                                });
                                ui.add_space(80.0);
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        if !tp.contact.trim().is_empty() {
                                            ui.label(
                                                egui::RichText::new(tp.contact.trim())
                                                    .monospace()
                                                    .size(9.5)
                                                    .color(palette.paper_ink),
                                            );
                                        }
                                        if !tp.copyright.trim().is_empty() {
                                            ui.label(
                                                egui::RichText::new(tp.copyright.trim())
                                                    .monospace()
                                                    .size(9.5)
                                                    .color(palette.muted_text),
                                            );
                                        }
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::BOTTOM),
                                        |ui| {
                                            if !tp.draft_date.trim().is_empty() {
                                                ui.label(
                                                    egui::RichText::new(tp.draft_date.trim())
                                                        .monospace()
                                                        .size(9.5)
                                                        .color(palette.paper_ink),
                                                );
                                            }
                                        },
                                    );
                                });
                            });
                    });
                });
                if tp_changed {
                    self.mark_dirty();
                }
                ui.add_space(8.0);
                if ui.button(self.i18n.t("btn-close")).clicked() {
                    self.show_title_page_editor = false;
                }
            });
        if !open {
            self.show_title_page_editor = false;
        }
    }

    fn import_fdx(&mut self) {
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

    fn export_fdx(&mut self) {
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

    fn toggle_dual_dialogue(&mut self) {
        let idx = self.editor.focused_index;
        if idx >= self.doc.elements.len() {
            return;
        }
        // Find the Character block containing `idx`
        let mut char_idx = None;
        for i in (0..=idx).rev() {
            match self.doc.elements[i].kind {
                ElementType::Character => {
                    char_idx = Some(i);
                    break;
                }
                ElementType::Dialogue | ElementType::Parenthetical => {}
                _ => break,
            }
        }
        let Some(right_start) = char_idx else {
            return;
        };

        // If already dual, clear it; otherwise link with the immediately preceding Character block.
        if self.doc.elements[right_start].dual_dialogue.is_some() {
            let group_id = self.doc.elements[right_start]
                .dual_dialogue
                .map(|d| d.group_id);
            for el in &mut self.doc.elements {
                if el.dual_dialogue.map(|d| d.group_id) == group_id {
                    el.dual_dialogue = None;
                }
            }
            self.mark_dirty();
            return;
        }

        // Find previous Character block before `right_start`
        let mut left_start = None;
        for i in (0..right_start).rev() {
            match self.doc.elements[i].kind {
                ElementType::Character => {
                    left_start = Some(i);
                    break;
                }
                ElementType::Dialogue | ElementType::Parenthetical => {}
                _ => break,
            }
        }
        if let Some(ls) = left_start {
            let group_id = Uuid::new_v4();
            let right_char_id = self.doc.elements[right_start].id;
            for el in &mut self.doc.elements[ls..right_start] {
                el.dual_dialogue = Some(DualDialogueGroup {
                    group_id,
                    side: DualSide::Left,
                });
            }
            for el in &mut self.doc.elements[right_start..] {
                if el.kind == ElementType::Character && el.id != right_char_id {
                    break;
                }
                if matches!(
                    el.kind,
                    ElementType::Character | ElementType::Dialogue | ElementType::Parenthetical
                ) {
                    el.dual_dialogue = Some(DualDialogueGroup {
                        group_id,
                        side: DualSide::Right,
                    });
                } else {
                    break;
                }
            }
            self.mark_dirty();
        }
    }

    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        let (save, undo_pressed, redo_pressed, find_pressed, print_pressed, focus_pressed) = ctx
            .input_mut(|input| {
                let save = input.consume_key(egui::Modifiers::COMMAND, egui::Key::S);
                let undo = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
                let redo = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
                    || input.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::Z,
                    );
                let find = input.consume_key(egui::Modifiers::COMMAND, egui::Key::F);
                let print = input.consume_key(egui::Modifiers::COMMAND, egui::Key::P);
                let focus = input.consume_key(egui::Modifiers::NONE, egui::Key::F11);
                (save, undo, redo, find, print, focus)
            });

        if save {
            self.save_current();
        }
        if undo_pressed && self.undo.undo(&mut self.doc).is_ok() {
            self.mark_dirty();
        }
        if redo_pressed && self.undo.redo(&mut self.doc).is_ok() {
            self.mark_dirty();
        }
        if find_pressed {
            self.show_find_replace = true;
        }
        if print_pressed {
            self.open_print_dialog();
        }
        if focus_pressed {
            self.focus_mode = !self.focus_mode;
        }
    }
    fn show_shortcuts_window(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts_dialog {
            return;
        }
        let mut open = self.show_shortcuts_dialog;
        egui::Window::new(self.i18n.t("shortcuts-title"))
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(self.i18n.t("shortcuts-body"));
                ui.add_space(10.0);
                if ui.button(self.i18n.t("btn-close")).clicked() {
                    self.show_shortcuts_dialog = false;
                }
            });
        if !open {
            self.show_shortcuts_dialog = false;
        }
    }

    fn show_about_window(&mut self, ctx: &egui::Context) {
        if !self.show_about_dialog {
            return;
        }
        let mut open = self.show_about_dialog;
        egui::Window::new(self.i18n.t("about-title"))
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    theme::draw_logo_mark(ui, 48.0);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.heading("draM.");
                        let mut ver_args = FluentArgs::new();
                        ver_args.set("version", env!("CARGO_PKG_VERSION"));
                        ui.label(
                            egui::RichText::new(self.i18n.t_args("about-version", &ver_args))
                                .weak(),
                        );
                    });
                });
                ui.add_space(10.0);
                ui.label(self.i18n.t("about-desc"));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(self.i18n.t("about-suite")).weak());
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(self.i18n.t("about-license"))
                        .small()
                        .weak(),
                );
                ui.add_space(12.0);
                if ui.button(self.i18n.t("btn-close")).clicked() {
                    self.show_about_dialog = false;
                }
            });
        if !open {
            self.show_about_dialog = false;
        }
    }

    fn show_unsaved_close_window(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_dialog {
            return;
        }
        let mut open = self.show_unsaved_dialog;
        egui::Window::new(self.i18n.t("unsaved-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.label(self.i18n.t("unsaved-message"));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(self.i18n.t("btn-save-and-close")).clicked() {
                        self.save_current();
                        if !self.dirty {
                            self.show_unsaved_dialog = false;
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                    if ui.button(self.i18n.t("btn-discard-close")).clicked() {
                        let _ = remove_recovery(&self.active_recovery_base_path());
                        let _ = remove_recovery(&std::env::temp_dir().join("dram_unsaved.dram"));
                        self.show_unsaved_dialog = false;
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button(self.i18n.t("btn-cancel")).clicked() {
                        self.show_unsaved_dialog = false;
                    }
                });
            });
        if !open {
            self.show_unsaved_dialog = false;
        }
    }

    fn show_recovery_window(&mut self, ctx: &egui::Context) {
        let Some(rec_path) = self.pending_recovery_path.clone() else {
            return;
        };
        let mut open = true;
        egui::Window::new(self.i18n.t("recovery-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(self.i18n.t("recovery-message"));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(self.i18n.t("btn-recover")).clicked() {
                        match load_dram(&rec_path) {
                            Ok(loaded) => {
                                self.doc = loaded;
                                self.undo.clear();
                                self.editor = EditorState::default();
                                self.mark_dirty();
                                self.status_error = None;
                            }
                            Err(err) => {
                                let mut args = FluentArgs::new();
                                args.set("reason", err.to_string());
                                self.status_error =
                                    Some(self.i18n.t_args("error-open-failed", &args));
                            }
                        }
                        let _ = fs::remove_file(&rec_path);
                        self.pending_recovery_path = None;
                    }
                    if ui.button(self.i18n.t("btn-discard-recovery")).clicked() {
                        let _ = fs::remove_file(&rec_path);
                        self.pending_recovery_path = None;
                    }
                });
            });
        if !open {
            self.pending_recovery_path = None;
        }
    }

    fn show_beat_board(&mut self, ui: &mut egui::Ui) {
        self.ensure_pagination();

        // Collect scenes: (start_idx, end_idx, heading, synopsis_idx, synopsis_text, page_num)
        struct CardInfo {
            start: usize,
            end: usize,
            heading: String,
            synopsis_idx: Option<usize>,
            synopsis: String,
            page_num: usize,
        }

        let mut cards: Vec<CardInfo> = Vec::new();
        for (idx, el) in self.doc.elements.iter().enumerate() {
            if el.kind == ElementType::SceneHeading {
                if let Some(prev) = cards.last_mut() {
                    prev.end = idx;
                }
                let num = el
                    .scene_number
                    .as_ref()
                    .map(|n| format!("{}. ", n.value))
                    .unwrap_or_default();
                let heading = el.plain_text();
                let label = if heading.trim().is_empty() {
                    format!("{num}...")
                } else {
                    format!("{num}{heading}")
                };
                let page_num = element_page_number(&self.cached_pagination, idx);
                cards.push(CardInfo {
                    start: idx,
                    end: self.doc.elements.len(),
                    heading: label,
                    synopsis_idx: None,
                    synopsis: String::new(),
                    page_num,
                });
            } else if el.kind == ElementType::Synopsis {
                if let Some(cur) = cards.last_mut() {
                    if cur.synopsis_idx.is_none() {
                        cur.synopsis_idx = Some(idx);
                        cur.synopsis = el.plain_text();
                    }
                }
            }
        }

        let mut move_req: Option<(usize, usize)> = None;
        let mut jump_to: Option<usize> = None;
        let mut synopsis_update: Option<(usize, Option<usize>, String)> = None;
        let total_pages = self.cached_pagination.page_count();
        let palette = theme::paper_palette(self.dark_theme);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.heading(self.i18n.t("beat-board-title"));
                });
                ui.add_space(12.0);

                if cards.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(self.i18n.t("beat-board-empty")).weak());
                    });
                    return;
                }

                let avail_w = (ui.available_width() - 40.0).max(280.0);
                let card_w = 280.0_f32;
                let cols = ((avail_w / (card_w + 14.0)).floor() as usize).max(1);
                let total_cards = cards.len();

                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    egui::Grid::new("beat_board_grid")
                        .num_columns(cols)
                        .spacing([14.0, 14.0])
                        .show(ui, |ui| {
                            for (c_i, card) in cards.iter_mut().enumerate() {
                                let is_active = self.editor.focused_index >= card.start
                                    && self.editor.focused_index < card.end;
                                let border = if is_active {
                                    egui::Stroke::new(1.5_f32, theme::ACCENT_CRIMSON)
                                } else {
                                    egui::Stroke::new(1.0_f32, palette.paper_border)
                                };

                                egui::Frame::new()
                                    .fill(palette.paper_bg)
                                    .stroke(border)
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .inner_margin(egui::Margin::same(12))
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width(card_w - 24.0);
                                            ui.set_min_height(155.0);

                                            ui.label(
                                                egui::RichText::new(&card.heading)
                                                    .strong()
                                                    .size(13.0)
                                                    .color(palette.paper_ink),
                                            );

                                            let mut page_args = FluentArgs::new();
                                            page_args.set("current", card.page_num);
                                            page_args.set("total", total_pages);
                                            ui.label(
                                                egui::RichText::new(
                                                    self.i18n.t_args("page-x-of-y", &page_args),
                                                )
                                                .size(11.5)
                                                .color(palette.muted_text),
                                            );
                                            ui.add_space(6.0);

                                            let resp = ui.add(
                                                egui::TextEdit::multiline(&mut card.synopsis)
                                                    .hint_text(
                                                        self.i18n
                                                            .t("beat-board-synopsis-placeholder"),
                                                    )
                                                    .desired_rows(3)
                                                    .desired_width(card_w - 28.0)
                                                    .background_color(egui::Color32::from_rgb(
                                                        244, 246, 249,
                                                    ))
                                                    .text_color(palette.paper_ink),
                                            );
                                            if resp.changed() {
                                                synopsis_update = Some((
                                                    card.start,
                                                    card.synopsis_idx,
                                                    card.synopsis.clone(),
                                                ));
                                            }

                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                if ui
                                                    .small_button(self.i18n.t("beat-board-open"))
                                                    .clicked()
                                                {
                                                    jump_to = Some(card.start);
                                                }
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if c_i + 1 < total_cards
                                                            && ui
                                                                .small_button(">")
                                                                .on_hover_text(
                                                                    self.i18n
                                                                        .t("beat-board-move-next"),
                                                                )
                                                                .clicked()
                                                        {
                                                            move_req = Some((c_i, c_i + 1));
                                                        }
                                                        if c_i > 0
                                                            && ui
                                                                .small_button("<")
                                                                .on_hover_text(
                                                                    self.i18n
                                                                        .t("beat-board-move-prev"),
                                                                )
                                                                .clicked()
                                                        {
                                                            move_req = Some((c_i, c_i - 1));
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    });

                                if (c_i + 1) % cols == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
                ui.add_space(24.0);
            });

        if let Some((scene_start, syn_idx, new_text)) = synopsis_update {
            if let Some(idx) = syn_idx {
                let old_runs = self.doc.elements[idx].runs.clone();
                let new_runs = if new_text.is_empty() {
                    Vec::new()
                } else {
                    vec![TextRun::plain(new_text)]
                };
                if self
                    .undo
                    .execute(
                        &mut self.doc,
                        EditCommand::UpdateRuns {
                            index: idx,
                            old_runs,
                            new_runs,
                        },
                    )
                    .is_ok()
                {
                    self.mark_dirty();
                }
            } else if !new_text.is_empty() {
                let insert_idx = (scene_start + 1).min(self.doc.elements.len());
                if self
                    .undo
                    .execute(
                        &mut self.doc,
                        EditCommand::InsertElement {
                            index: insert_idx,
                            element: Element::new(ElementType::Synopsis, new_text),
                        },
                    )
                    .is_ok()
                {
                    self.mark_dirty();
                }
            }
        }

        if let Some((from_s, to_s)) = move_req {
            let simple_scenes: Vec<(usize, usize, String)> = cards
                .into_iter()
                .map(|c| (c.start, c.end, c.heading))
                .collect();
            self.swap_scenes(&simple_scenes, from_s, to_s);
        }

        if let Some(target_idx) = jump_to {
            self.editor.focused_index = target_idx;
            self.editor.pending_focus = Some(target_idx);
            self.view_mode = ViewMode::Editor;
        }
    }

    fn show_reports_window(&mut self, ctx: &egui::Context) {
        if !self.show_reports_dialog {
            return;
        }
        let breakdown = generate_breakdown(&self.doc);
        let mut open = self.show_reports_dialog;

        egui::Window::new(self.i18n.t("reports-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(540.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                let mut sum_args = FluentArgs::new();
                sum_args.set("scenes", breakdown.total_scenes);
                sum_args.set("int", breakdown.interior_scenes);
                sum_args.set("ext", breakdown.exterior_scenes);
                sum_args.set("day", breakdown.day_scenes);
                sum_args.set("night", breakdown.night_scenes);
                ui.label(
                    egui::RichText::new(self.i18n.t_args("reports-summary", &sum_args)).strong(),
                );
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.report_tab == ReportTab::Characters,
                            self.i18n.t("reports-tab-characters"),
                        )
                        .clicked()
                    {
                        self.report_tab = ReportTab::Characters;
                    }
                    if ui
                        .selectable_label(
                            self.report_tab == ReportTab::Locations,
                            self.i18n.t("reports-tab-locations"),
                        )
                        .clicked()
                    {
                        self.report_tab = ReportTab::Locations;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(self.i18n.t("reports-export-csv")).clicked() {
                            if let Some(mut path) = rfd::FileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_file_name("senaryo_rapor.csv")
                                .save_file()
                            {
                                if path.extension().is_none() {
                                    path.set_extension("csv");
                                }
                                let csv = breakdown_to_csv(&breakdown);
                                match fs::write(&path, csv) {
                                    Ok(()) => {
                                        self.report_status = Some(self.i18n.t("reports-exported"));
                                    }
                                    Err(err) => {
                                        let mut args = FluentArgs::new();
                                        args.set("reason", err.to_string());
                                        self.report_status =
                                            Some(self.i18n.t_args("error-save-failed", &args));
                                    }
                                }
                            }
                        }
                    });
                });

                if let Some(status) = &self.report_status {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(status).small().weak());
                }

                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.report_tab {
                        ReportTab::Characters => {
                            egui::Grid::new("reports_char_grid")
                                .num_columns(4)
                                .striped(true)
                                .spacing([18.0, 6.0])
                                .show(ui, |ui| {
                                    ui.strong(self.i18n.t("reports-col-name"));
                                    ui.strong(self.i18n.t("reports-col-scenes"));
                                    ui.strong(self.i18n.t("reports-col-lines"));
                                    ui.strong(self.i18n.t("reports-col-words"));
                                    ui.end_row();
                                    for ch in &breakdown.characters {
                                        ui.label(&ch.name);
                                        ui.label(ch.scene_count.to_string());
                                        ui.label(ch.dialogue_blocks.to_string());
                                        ui.label(ch.word_count.to_string());
                                        ui.end_row();
                                    }
                                });
                        }
                        ReportTab::Locations => {
                            egui::Grid::new("reports_loc_grid")
                                .num_columns(4)
                                .striped(true)
                                .spacing([18.0, 6.0])
                                .show(ui, |ui| {
                                    ui.strong(self.i18n.t("reports-col-location"));
                                    ui.strong(self.i18n.t("reports-col-scenes"));
                                    ui.strong(self.i18n.t("reports-col-int-ext"));
                                    ui.strong(self.i18n.t("reports-col-day-night"));
                                    ui.end_row();
                                    for loc in &breakdown.locations {
                                        ui.label(&loc.location);
                                        ui.label(loc.scene_count.to_string());
                                        ui.label(format!(
                                            "{} / {}",
                                            loc.interior_count, loc.exterior_count
                                        ));
                                        ui.label(format!(
                                            "{} / {}",
                                            loc.day_count, loc.night_count
                                        ));
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });

        if !open {
            self.show_reports_dialog = false;
            self.report_status = None;
        }
    }
}

impl eframe::App for DramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.dirty && !self.allow_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_unsaved_dialog = true;
            } else {
                let _ = remove_recovery(&self.active_recovery_base_path());
                let _ = remove_recovery(&std::env::temp_dir().join("dram_unsaved.dram"));
                if let Some(old) = self.last_print_pdf.take() {
                    let _ = fs::remove_file(old);
                }
            }
        }

        if self.dirty
            && self.last_autosave.elapsed() >= Duration::from_secs(15)
            && !self.autosave_in_flight.swap(true, Ordering::AcqRel)
        {
            let base = self.active_recovery_base_path();
            let doc_snapshot = self.doc.clone();
            let flag = Arc::clone(&self.autosave_in_flight);
            self.last_autosave = Instant::now();
            std::thread::spawn(move || {
                if let Err(err) = write_recovery(&base, &doc_snapshot) {
                    eprintln!("[autosave] failed to write recovery: {err}");
                }
                flag.store(false, Ordering::Release);
            });
        }

        if self.applied_dark_theme != Some(self.dark_theme) {
            theme::apply_theme(ctx, self.dark_theme);
            self.applied_dark_theme = Some(self.dark_theme);
        }
        normalize_linux_ime_events(ctx);

        self.handle_global_shortcuts(ctx);

        // Top menu bar & element toolbar (hidden in focus mode)
        if !self.focus_mode {
            let bar_frame = egui::Frame::side_top_panel(&ctx.style())
                .inner_margin(egui::Margin::symmetric(12, 6));
            egui::TopBottomPanel::top("top_panel")
                .frame(bar_frame)
                .show(ctx, |ui| {
                    egui::menu::bar(ui, |ui| {
                        theme::draw_logo_mark(ui, 18.0);
                        ui.add_space(4.0);
                        ui.menu_button(self.i18n.t("menu-file"), |ui| {
                            if ui.button(self.i18n.t("menu-new")).clicked() {
                                self.new_document();
                                ui.close_menu();
                            }
                            ui.menu_button(self.i18n.t("menu-new-template"), |ui| {
                                if ui.button(self.i18n.t("template-feature")).clicked() {
                                    self.new_from_template(ProjectTemplate::FeatureFilm);
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("template-short")).clicked() {
                                    self.new_from_template(ProjectTemplate::ShortFilm);
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("template-series")).clicked() {
                                    self.new_from_template(ProjectTemplate::TvEpisode);
                                    ui.close_menu();
                                }
                            });
                            if ui.button(self.i18n.t("menu-open")).clicked() {
                                self.open_dram();
                                ui.close_menu();
                            }
                            let recent_snapshot = self.recent_files.clone();
                            let mut open_recent: Option<PathBuf> = None;
                            let mut clear_recent = false;
                            ui.menu_button(self.i18n.t("menu-recent-files"), |ui| {
                                if recent_snapshot.is_empty() {
                                    ui.label(
                                        egui::RichText::new(self.i18n.t("menu-recent-empty"))
                                            .weak(),
                                    );
                                } else {
                                    for p in &recent_snapshot {
                                        let label = p
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or_default()
                                            .to_string();
                                        if ui
                                            .button(label)
                                            .on_hover_text(p.display().to_string())
                                            .clicked()
                                        {
                                            open_recent = Some(p.clone());
                                            ui.close_menu();
                                        }
                                    }
                                    ui.separator();
                                    if ui.button(self.i18n.t("menu-recent-clear")).clicked() {
                                        clear_recent = true;
                                        ui.close_menu();
                                    }
                                }
                            });
                            if let Some(p) = open_recent {
                                self.open_path(&p);
                            }
                            if clear_recent {
                                self.clear_recent_files();
                            }
                            if ui.button(self.i18n.t("menu-save")).clicked() {
                                self.save_current();
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.t("menu-save-as")).clicked() {
                                self.save_as();
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.menu_button(self.i18n.t("menu-import"), |ui| {
                                if ui.button(self.i18n.t("menu-import-fountain")).clicked() {
                                    self.import_fountain();
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("menu-import-fadein")).clicked() {
                                    self.import_fadein();
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("menu-import-fdx")).clicked() {
                                    self.import_fdx();
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button(self.i18n.t("menu-export"), |ui| {
                                if ui.button(self.i18n.t("menu-export-pdf")).clicked() {
                                    self.export_pdf();
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("menu-export-fountain")).clicked() {
                                    self.export_fountain();
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("menu-export-fadein")).clicked() {
                                    self.export_fadein();
                                    ui.close_menu();
                                }
                                if ui.button(self.i18n.t("menu-export-fdx")).clicked() {
                                    self.export_fdx();
                                    ui.close_menu();
                                }
                            });
                            ui.separator();
                            if ui.button(self.i18n.t("menu-print")).clicked() {
                                self.open_print_dialog();
                                ui.close_menu();
                            }
                        });

                        ui.menu_button(self.i18n.t("menu-edit"), |ui| {
                            let can_undo = self.undo.can_undo();
                            if ui
                                .add_enabled(can_undo, egui::Button::new(self.i18n.t("menu-undo")))
                                .clicked()
                            {
                                if self.undo.undo(&mut self.doc).is_ok() {
                                    self.mark_dirty();
                                }
                                ui.close_menu();
                            }
                            let can_redo = self.undo.can_redo();
                            if ui
                                .add_enabled(can_redo, egui::Button::new(self.i18n.t("menu-redo")))
                                .clicked()
                            {
                                if self.undo.redo(&mut self.doc).is_ok() {
                                    self.mark_dirty();
                                }
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(self.i18n.t("menu-find-replace")).clicked() {
                                self.show_find_replace = true;
                                ui.close_menu();
                            }
                        });

                        ui.menu_button(self.i18n.t("menu-view"), |ui| {
                            if ui
                                .selectable_label(
                                    self.view_mode == ViewMode::Editor,
                                    self.i18n.t("view-editor"),
                                )
                                .clicked()
                            {
                                self.view_mode = ViewMode::Editor;
                                ui.close_menu();
                            }
                            if ui
                                .selectable_label(
                                    self.view_mode == ViewMode::BeatBoard,
                                    self.i18n.t("view-beat-board"),
                                )
                                .clicked()
                            {
                                self.view_mode = ViewMode::BeatBoard;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .checkbox(&mut self.show_navigator, self.i18n.t("nav-toggle"))
                                .changed()
                            {
                                ui.close_menu();
                            }
                            if ui
                                .checkbox(&mut self.focus_mode, self.i18n.t("focus-mode-toggle"))
                                .changed()
                            {
                                ui.close_menu();
                            }
                            if ui
                                .checkbox(
                                    &mut self.spellcheck_enabled,
                                    self.i18n.t("spellcheck-toggle"),
                                )
                                .changed()
                            {
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.label(self.i18n.t("theme-label"));
                            if ui
                                .selectable_label(self.dark_theme, self.i18n.t("theme-dark"))
                                .clicked()
                            {
                                self.dark_theme = true;
                            }
                            if ui
                                .selectable_label(!self.dark_theme, self.i18n.t("theme-light"))
                                .clicked()
                            {
                                self.dark_theme = false;
                            }
                        });

                        ui.menu_button(self.i18n.t("menu-format"), |ui| {
                            let current_kind = self
                                .doc
                                .elements
                                .get(self.editor.focused_index)
                                .map_or(ElementType::Action, |e| e.kind);
                            for (idx, &kind) in ELEMENT_TYPES.iter().take(8).enumerate() {
                                let label = format!(
                                    "{}  {}",
                                    idx + 1,
                                    self.i18n.t(element_type_i18n_key(kind))
                                );
                                if ui.selectable_label(current_kind == kind, label).clicked()
                                    && current_kind != kind
                                    && self
                                        .undo
                                        .execute(
                                            &mut self.doc,
                                            EditCommand::ChangeElementType {
                                                index: self.editor.focused_index,
                                                old_kind: current_kind,
                                                new_kind: kind,
                                            },
                                        )
                                        .is_ok()
                                {
                                    self.mark_dirty();
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui.button(self.i18n.t("dual-dialogue-btn")).clicked() {
                                self.toggle_dual_dialogue();
                                ui.close_menu();
                            }
                            ui.checkbox(&mut self.revision_mode, self.i18n.t("rev-mode-toggle"));
                            if ui.button(self.i18n.t("rev-clear-all")).clicked() {
                                for el in &mut self.doc.elements {
                                    el.revision = None;
                                }
                                self.mark_dirty();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(self.i18n.t("menu-reports")).clicked() {
                                self.show_reports_dialog = true;
                                ui.close_menu();
                            }
                        });

                        ui.menu_button(self.i18n.t("menu-document"), |ui| {
                            if ui.button(self.i18n.t("menu-title-page")).clicked() {
                                self.show_title_page_editor = true;
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.t("menu-renumber-scenes")).clicked() {
                                self.doc.renumber_scenes();
                                self.mark_dirty();
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.label(self.i18n.t("page-size-label"));
                            if ui
                                .selectable_label(
                                    self.doc.settings.page_size == PageSize::A4,
                                    self.i18n.t("page-size-a4"),
                                )
                                .clicked()
                            {
                                self.doc.settings.page_size = PageSize::A4;
                                self.mark_dirty();
                            }
                            if ui
                                .selectable_label(
                                    self.doc.settings.page_size == PageSize::UsLetter,
                                    self.i18n.t("page-size-us-letter"),
                                )
                                .clicked()
                            {
                                self.doc.settings.page_size = PageSize::UsLetter;
                                self.mark_dirty();
                            }
                            ui.separator();
                            if ui
                                .checkbox(
                                    &mut self.doc.settings.show_scene_numbers,
                                    self.i18n.t("show-scene-numbers"),
                                )
                                .changed()
                            {
                                if self.doc.settings.show_scene_numbers {
                                    self.doc.renumber_scenes();
                                }
                                self.mark_dirty();
                            }
                        });

                        ui.menu_button(self.i18n.t("menu-settings"), |ui| {
                            ui.label(self.i18n.t("ui-language-label"));
                            if ui
                                .selectable_label(
                                    self.i18n.active() == UiLanguage::Tr,
                                    self.i18n.t("ui-language-tr"),
                                )
                                .clicked()
                            {
                                self.i18n.set_language(UiLanguage::Tr);
                            }
                            if ui
                                .selectable_label(
                                    self.i18n.active() == UiLanguage::En,
                                    self.i18n.t("ui-language-en"),
                                )
                                .clicked()
                            {
                                self.i18n.set_language(UiLanguage::En);
                            }

                            ui.separator();
                            ui.label(self.i18n.t("script-language-label"));
                            if ui
                                .selectable_label(
                                    self.doc.settings.script_language == ScriptLanguage::Tr,
                                    self.i18n.t("script-language-tr"),
                                )
                                .clicked()
                            {
                                self.doc.settings.set_script_language(ScriptLanguage::Tr);
                                self.mark_dirty();
                            }
                            if ui
                                .selectable_label(
                                    self.doc.settings.script_language == ScriptLanguage::En,
                                    self.i18n.t("script-language-en"),
                                )
                                .clicked()
                            {
                                self.doc.settings.set_script_language(ScriptLanguage::En);
                                self.mark_dirty();
                            }
                        });

                        ui.menu_button(self.i18n.t("menu-help"), |ui| {
                            if ui.button(self.i18n.t("menu-shortcuts")).clicked() {
                                self.show_shortcuts_dialog = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(self.i18n.t("menu-about")).clicked() {
                                self.show_about_dialog = true;
                                ui.close_menu();
                            }
                        });
                    });

                    ui.separator();

                    // Element type selector toolbar
                    ui.add_space(1.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(self.i18n.t("toolbar-element-type"));
                            let current_kind = self
                                .doc
                                .elements
                                .get(self.editor.focused_index)
                                .map_or(ElementType::Action, |e| e.kind);

                            egui::ComboBox::from_id_salt("element_type_combo")
                                .selected_text(self.i18n.t(element_type_i18n_key(current_kind)))
                                .show_ui(ui, |ui| {
                                    for &kind in &ELEMENT_TYPES {
                                        let label = self.i18n.t(element_type_i18n_key(kind));
                                        if ui
                                            .selectable_label(current_kind == kind, label)
                                            .clicked()
                                            && current_kind != kind
                                            && self
                                                .undo
                                                .execute(
                                                    &mut self.doc,
                                                    EditCommand::ChangeElementType {
                                                        index: self.editor.focused_index,
                                                        old_kind: current_kind,
                                                        new_kind: kind,
                                                    },
                                                )
                                                .is_ok()
                                        {
                                            self.mark_dirty();
                                        }
                                    }
                                });

                            ui.separator();
                            if ui.button(self.i18n.t("dual-dialogue-btn")).clicked() {
                                self.toggle_dual_dialogue();
                            }

                            ui.separator();
                            ui.label(self.i18n.t("revision-label"));
                            let cur_rev = self
                                .doc
                                .elements
                                .get(self.editor.focused_index)
                                .and_then(|e| e.revision.as_ref().map(|r| r.color.clone()));
                            let rev_label = match cur_rev.as_deref() {
                                Some("blue") => self.i18n.t("rev-blue"),
                                Some("pink") => self.i18n.t("rev-pink"),
                                Some("yellow") => self.i18n.t("rev-yellow"),
                                Some("green") => self.i18n.t("rev-green"),
                                _ => self.i18n.t("rev-none"),
                            };
                            egui::ComboBox::from_id_salt("revision_combo")
                                .selected_text(rev_label)
                                .show_ui(ui, |ui| {
                                    let options = [
                                        (None, "rev-none"),
                                        (Some("blue"), "rev-blue"),
                                        (Some("pink"), "rev-pink"),
                                        (Some("yellow"), "rev-yellow"),
                                        (Some("green"), "rev-green"),
                                    ];
                                    for (val, key) in options {
                                        let is_sel = cur_rev.as_deref() == val;
                                        if ui.selectable_label(is_sel, self.i18n.t(key)).clicked() {
                                            if let Some(c) = val {
                                                self.active_revision_color = c.to_string();
                                            }
                                            if let Some(el) =
                                                self.doc.elements.get_mut(self.editor.focused_index)
                                            {
                                                el.revision = val.map(|c| RevisionMark {
                                                    color: c.to_string(),
                                                    label: String::new(),
                                                });
                                                self.mark_dirty();
                                            }
                                        }
                                    }
                                });

                            let rev_btn_label = if self.revision_mode {
                                self.i18n.t("rev-mode-active")
                            } else {
                                self.i18n.t("rev-mode-toggle")
                            };
                            let mut rev_btn = egui::Button::new(rev_btn_label);
                            if self.revision_mode {
                                rev_btn = rev_btn.fill(theme::ACCENT_CRIMSON);
                            }
                            if ui.add(rev_btn).clicked() {
                                self.revision_mode = !self.revision_mode;
                            }

                            ui.separator();
                            let toggle_btn =
                                |ui: &mut egui::Ui, active: bool, label: String| -> bool {
                                    let mut btn = egui::Button::new(label);
                                    if active {
                                        btn = btn.fill(ui.visuals().widgets.hovered.bg_fill);
                                    }
                                    ui.add(btn).clicked()
                                };
                            if toggle_btn(
                                ui,
                                self.view_mode == ViewMode::BeatBoard,
                                self.i18n.t("view-beat-board"),
                            ) {
                                self.view_mode = match self.view_mode {
                                    ViewMode::Editor => ViewMode::BeatBoard,
                                    ViewMode::BeatBoard => ViewMode::Editor,
                                };
                            }
                            if toggle_btn(ui, self.show_navigator, self.i18n.t("nav-toggle")) {
                                self.show_navigator = !self.show_navigator;
                            }
                            if toggle_btn(
                                ui,
                                self.show_navigator && self.inspector_tab == InspectorTab::Notes,
                                self.i18n.t("notes-toggle"),
                            ) {
                                self.show_navigator = true;
                                self.inspector_tab = InspectorTab::Notes;
                            }
                            if toggle_btn(ui, self.focus_mode, self.i18n.t("focus-mode-toggle")) {
                                self.focus_mode = !self.focus_mode;
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let doc_name = self
                                        .current_path
                                        .as_ref()
                                        .and_then(|p| p.file_name())
                                        .and_then(|n| n.to_str())
                                        .map(ToString::to_string)
                                        .unwrap_or_else(|| self.i18n.t("untitled-document"));
                                    let tab_label = if self.dirty {
                                        format!("{doc_name} *")
                                    } else {
                                        doc_name
                                    };
                                    ui.label(egui::RichText::new(tab_label).size(12.5).weak());
                                },
                            );
                        },
                    );
                    ui.add_space(1.0);
                });

            // Bottom status bar
            let status_frame = egui::Frame::side_top_panel(&ctx.style())
                .inner_margin(egui::Margin::symmetric(12, 6));
            egui::TopBottomPanel::bottom("status_bar")
                .frame(status_frame)
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        let scene_count = self
                            .doc
                            .elements
                            .iter()
                            .filter(|e| e.kind == ElementType::SceneHeading)
                            .count();
                        let mut scene_args = FluentArgs::new();
                        scene_args.set("count", scene_count);
                        ui.label(self.i18n.t_args("status-scenes-count", &scene_args));

                        ui.separator();

                        let mut line_args = FluentArgs::new();
                        line_args.set("count", self.doc.elements.len());
                        ui.label(self.i18n.t_args("status-elements-count", &line_args));

                        ui.separator();

                        self.ensure_pagination();
                        let mut page_args = FluentArgs::new();
                        page_args.set("pages", self.cached_pagination.page_count());
                        page_args.set("minutes", self.cached_pagination.estimated_minutes());
                        ui.label(self.i18n.t_args("status-pages-runtime", &page_args));

                        ui.separator();

                        let status_key = if self.dirty {
                            "status-modified"
                        } else {
                            "status-saved"
                        };
                        ui.label(self.i18n.t(status_key));

                        if let Some(err) = &self.status_error {
                            ui.separator();
                            ui.colored_label(egui::Color32::LIGHT_RED, err);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(self.i18n.t("status-hint-flow"))
                                    .weak()
                                    .small(),
                            );
                        });
                    });
                });

            // Fade In-style right dock (Format/Characters/Locations/Notes + Navigator)
            self.show_right_dock(ctx);
        }

        // Main screenplay editor canvas or Index Cards (Beat Board)
        self.ensure_pagination();
        let active_revision = self.revision_mode.then(|| RevisionMark {
            color: self.active_revision_color.clone(),
            label: String::new(),
        });
        let mut added_dict_word: Option<String> = None;
        let desk_bg = theme::paper_palette(self.dark_theme).desk_bg;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(desk_bg))
            .show(ctx, |ui| match self.view_mode {
                ViewMode::Editor => {
                    let spell = self.spellcheck_enabled.then_some(&self.spellchecker);
                    if self.editor.show(
                        ui,
                        &mut self.doc,
                        &mut self.undo,
                        &self.cached_pagination,
                        active_revision.as_ref(),
                        spell,
                        &mut added_dict_word,
                        &self.i18n,
                        self.dark_theme,
                    ) {
                        self.mark_dirty();
                    }
                }
                ViewMode::BeatBoard => {
                    self.show_beat_board(ui);
                }
            });

        if let Some(word) = added_dict_word {
            self.spellchecker.add_to_user_dict(&word);
            save_user_dict(&self.spellchecker.user_words_sorted());
        }

        self.show_title_page_window(ctx);
        self.show_find_replace_window(ctx);
        self.show_print_window(ctx);
        self.show_reports_window(ctx);
        self.show_asset_preview_window(ctx);
        self.show_shortcuts_window(ctx);
        self.show_about_window(ctx);
        self.show_unsaved_close_window(ctx);
        self.show_recovery_window(ctx);
    }
}

/// Converts orphaned `ImeEvent::Commit` events (sent by Linux X11/ibus for Turkish keys
/// such as `ü`, `ş`, `ğ` without a preceding `Preedit`) into `Event::Text` so `egui::TextEdit`
/// never drops them.
fn normalize_linux_ime_events(ctx: &egui::Context) {
    ctx.input_mut(|input| {
        let has_preedit = input.events.iter().any(|ev| {
            matches!(
                ev,
                egui::Event::Ime(egui::ImeEvent::Preedit(s)) if !s.is_empty()
            )
        });
        if has_preedit {
            return;
        }
        for ev in &mut input.events {
            if let egui::Event::Ime(egui::ImeEvent::Commit(text)) = ev {
                let committed = std::mem::take(text);
                if !committed.is_empty() {
                    *ev = egui::Event::Text(committed);
                }
            }
        }
    });
}

fn element_page_number(paged: &dram_core::PaginatedDocument, element_idx: usize) -> usize {
    for page in &paged.pages {
        if page
            .lines
            .iter()
            .any(|l| l.element_index == Some(element_idx))
        {
            return page.page_number;
        }
    }
    1
}

fn recent_files_store_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("dram").join("recent.txt"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("dram").join("recent.txt"));
    }
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("dram")
            .join("recent.txt")
    })
}

const MAX_TEXT_BYTES: u64 = 32 * 1024 * 1024;

fn read_text_bounded(path: &Path, max_bytes: u64) -> std::io::Result<String> {
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

fn load_recent_files() -> Vec<PathBuf> {
    let Some(store) = recent_files_store_path() else {
        return Vec::new();
    };
    let Ok(raw) = read_text_bounded(&store, 1024 * 1024) else {
        return Vec::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains('\0'))
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .take(8)
        .collect()
}

fn save_recent_files(paths: &[PathBuf]) {
    let Some(store) = recent_files_store_path() else {
        return;
    };
    if let Some(parent) = store.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("[config] failed to create config dir: {err}");
            return;
        }
    }
    let content = paths
        .iter()
        .filter_map(|p| p.to_str())
        .filter(|s| !s.contains('\n') && !s.contains('\r'))
        .collect::<Vec<_>>()
        .join("\n");
    if let Err(err) = write_atomic_bytes(&store, content.as_bytes()) {
        eprintln!("[config] failed to save recent files: {err}");
    }
}

fn user_dict_store_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("dram").join("user_dict.txt"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("dram").join("user_dict.txt"));
    }
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("dram")
            .join("user_dict.txt")
    })
}

fn load_user_dict() -> String {
    let Some(store) = user_dict_store_path() else {
        return String::new();
    };
    read_text_bounded(&store, 2 * 1024 * 1024).unwrap_or_default()
}

fn save_user_dict(words: &[String]) {
    let Some(store) = user_dict_store_path() else {
        return;
    };
    if let Some(parent) = store.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("[config] failed to create config dir: {err}");
            return;
        }
    }
    let cleaned: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|w| !w.is_empty() && !w.contains('\n') && !w.contains('\r'))
        .collect();
    if let Err(err) = write_atomic_bytes(&store, cleaned.join("\n").as_bytes()) {
        eprintln!("[config] failed to save user dict: {err}");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui_sources_contain_no_hardcoded_user_labels() {
        // Verify that common UI labels are routed through i18n keys rather than hardcoded literals.
        let main_src = include_str!("main.rs");
        let editor_src = include_str!("editor.rs");
        for forbidden in [
            "\"Dosya\"",
            "\"Kaydet\"",
            "\"File\"",
            "\"Save\"",
            "\"Undo\"",
            "\"Geri Al\"",
        ] {
            assert!(
                !main_src.contains(forbidden) && !editor_src.contains(forbidden),
                "found hardcoded UI string {forbidden}"
            );
        }
    }
}
