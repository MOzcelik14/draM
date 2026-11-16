//! `dram-app`: Desktop GUI for `draM.` screenplay writer.

#![forbid(unsafe_code)]

mod editor;
mod i18n;
mod theme;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;
use dram_core::text::contains_case_insensitive;
use dram_core::{
    paginate, Document, DualDialogueGroup, DualSide, EditCommand, ElementType, LayoutProfile, Note,
    PageSize, RevisionMark, ScriptLanguage, TextRun, UndoManager,
};
use dram_format::{
    load_dram, load_fadein, load_fdx, parse_fountain, remove_recovery, save_dram, save_fadein,
    save_fdx, serialize_fountain,
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

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        native_options,
        Box::new(|cc| {
            theme::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(DramApp::new(i18n)))
        }),
    )
    .map_err(|err| anyhow::anyhow!("eframe error: {err}"))
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
    show_navigator: bool,
    show_notes_panel: bool,
    dark_theme: bool,
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
}

impl DramApp {
    fn new(i18n: I18n) -> Self {
        let script_lang = match i18n.active() {
            UiLanguage::Tr => ScriptLanguage::Tr,
            UiLanguage::En => ScriptLanguage::En,
        };
        Self {
            doc: Document::new(script_lang),
            undo: UndoManager::new(),
            editor: EditorState::default(),
            i18n,
            current_path: None,
            dirty: false,
            status_error: None,
            show_title_page_editor: false,
            show_navigator: true,
            show_notes_panel: false,
            dark_theme: true,
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
        }
    }

    fn new_document(&mut self) {
        self.doc = Document::new(self.doc.settings.script_language);
        self.undo.clear();
        self.editor = EditorState::default();
        self.current_path = None;
        self.dirty = false;
        self.status_error = None;
    }

    fn open_dram(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("draM", &["dram"])
            .pick_file()
        else {
            return;
        };
        match load_dram(&path) {
            Ok(loaded) => {
                self.doc = loaded;
                self.undo.clear();
                self.editor = EditorState::default();
                self.current_path = Some(path);
                self.dirty = false;
                self.status_error = None;
            }
            Err(err) => {
                let mut args = FluentArgs::new();
                args.set("reason", err.to_string());
                self.status_error = Some(self.i18n.t_args("error-open-failed", &args));
            }
        }
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
                self.current_path = Some(path.to_path_buf());
                self.dirty = false;
                self.status_error = None;
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
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.doc = parse_fountain(&content, self.doc.settings.script_language);
                self.undo.clear();
                self.editor = EditorState::default();
                self.current_path = None;
                self.dirty = true;
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
        if let Err(err) = fs::write(&path, content) {
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
                self.dirty = true;
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
                        let path_res = if let Some(existing) = &self.last_print_pdf {
                            Ok(existing.clone())
                        } else {
                            render_temp_pdf(&self.doc, &profile)
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
                    }
                });
            });
        if !open {
            self.show_print_dialog = false;
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
                    self.dirty = true;
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
                self.dirty = true;
                self.find_status = None;
            }
        } else {
            self.find_status = Some(self.i18n.t("find-no-match"));
        }
    }

    fn show_scene_navigator(&mut self, ctx: &egui::Context) {
        if !self.show_navigator {
            return;
        }
        egui::SidePanel::left("scene_navigator")
            .resizable(true)
            .default_width(230.0)
            .show(ctx, |ui| {
                // Collect scene ranges (start_idx, end_idx_exclusive, label)
                let mut scenes: Vec<(usize, usize, String)> = Vec::new();
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
                        scenes.push((idx, self.doc.elements.len(), label));
                    }
                }

                ui.horizontal(|ui| {
                    ui.heading(self.i18n.t("nav-heading"));
                });
                let mut scene_args = FluentArgs::new();
                scene_args.set("count", scenes.len());
                ui.label(
                    egui::RichText::new(self.i18n.t_args("status-scenes-count", &scene_args))
                        .small()
                        .weak(),
                );
                ui.separator();

                let mut move_req: Option<(usize, usize)> = None;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (s_i, (start, _end, label)) in scenes.iter().enumerate() {
                        let is_current = self.editor.focused_index >= *start
                            && scenes
                                .get(s_i + 1)
                                .is_none_or(|next| self.editor.focused_index < next.0);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_current, label).clicked() {
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
                    }
                });

                if let Some((from_s, to_s)) = move_req {
                    self.swap_scenes(&scenes, from_s, to_s);
                }
            });
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
            self.dirty = true;
        }
    }

    fn show_title_page_window(&mut self, ctx: &egui::Context) {
        if !self.show_title_page_editor {
            return;
        }
        let mut open = self.show_title_page_editor;
        egui::Window::new(self.i18n.t("title-page-heading"))
            .open(&mut open)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                egui::Grid::new("title_page_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        let tp = &mut self.doc.title_page;
                        let mut row = |label_key: &str, val: &mut String, multiline: bool| {
                            ui.label(self.i18n.t(label_key));
                            let changed = if multiline {
                                ui.add(
                                    egui::TextEdit::multiline(val)
                                        .desired_rows(2)
                                        .desired_width(260.0),
                                )
                                .changed()
                            } else {
                                ui.add(egui::TextEdit::singleline(val).desired_width(260.0))
                                    .changed()
                            };
                            if changed {
                                self.dirty = true;
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
                self.dirty = true;
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
            self.dirty = true;
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
            self.dirty = true;
        }
    }

    fn show_notes_sidebar(&mut self, ctx: &egui::Context) {
        if !self.show_notes_panel {
            return;
        }
        egui::SidePanel::right("notes_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(self.i18n.t("notes-heading"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                                created_at: Utc::now(),
                            });
                            self.dirty = true;
                        }
                    });
                });
                ui.separator();

                let mut delete_idx: Option<usize> = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, note) in self.doc.notes.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut note.title)
                                            .hint_text(self.i18n.t("notes-title-placeholder"))
                                            .desired_width(160.0),
                                    )
                                    .changed()
                                {
                                    self.dirty = true;
                                }
                                if ui.small_button(self.i18n.t("notes-delete")).clicked() {
                                    delete_idx = Some(idx);
                                }
                            });
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut note.body)
                                        .hint_text(self.i18n.t("notes-body-placeholder"))
                                        .desired_rows(3)
                                        .desired_width(220.0),
                                )
                                .changed()
                            {
                                self.dirty = true;
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
                if let Some(i) = delete_idx {
                    self.doc.notes.remove(i);
                    self.dirty = true;
                }
            });
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
            self.dirty = true;
        }
        if redo_pressed && self.undo.redo(&mut self.doc).is_ok() {
            self.dirty = true;
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
}

impl eframe::App for DramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx, self.dark_theme);

        self.handle_global_shortcuts(ctx);

        // Top menu bar & element toolbar (hidden in focus mode)
        if !self.focus_mode {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(self.i18n.t("menu-file"), |ui| {
                        if ui.button(self.i18n.t("menu-new")).clicked() {
                            self.new_document();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-open")).clicked() {
                            self.open_dram();
                            ui.close_menu();
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
                        if ui.button(self.i18n.t("menu-title-page")).clicked() {
                            self.show_title_page_editor = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(self.i18n.t("menu-import-fountain")).clicked() {
                            self.import_fountain();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-export-fountain")).clicked() {
                            self.export_fountain();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-export-pdf")).clicked() {
                            self.export_pdf();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-print")).clicked() {
                            self.open_print_dialog();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(self.i18n.t("menu-import-fadein")).clicked() {
                            self.import_fadein();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-export-fadein")).clicked() {
                            self.export_fadein();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-import-fdx")).clicked() {
                            self.import_fdx();
                            ui.close_menu();
                        }
                        if ui.button(self.i18n.t("menu-export-fdx")).clicked() {
                            self.export_fdx();
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
                                self.dirty = true;
                            }
                            ui.close_menu();
                        }
                        let can_redo = self.undo.can_redo();
                        if ui
                            .add_enabled(can_redo, egui::Button::new(self.i18n.t("menu-redo")))
                            .clicked()
                        {
                            if self.undo.redo(&mut self.doc).is_ok() {
                                self.dirty = true;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(self.i18n.t("menu-find-replace")).clicked() {
                            self.show_find_replace = true;
                            ui.close_menu();
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
                            self.dirty = true;
                        }
                        if ui
                            .selectable_label(
                                self.doc.settings.script_language == ScriptLanguage::En,
                                self.i18n.t("script-language-en"),
                            )
                            .clicked()
                        {
                            self.doc.settings.set_script_language(ScriptLanguage::En);
                            self.dirty = true;
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
                            self.dirty = true;
                        }
                        if ui
                            .selectable_label(
                                self.doc.settings.page_size == PageSize::UsLetter,
                                self.i18n.t("page-size-us-letter"),
                            )
                            .clicked()
                        {
                            self.doc.settings.page_size = PageSize::UsLetter;
                            self.dirty = true;
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
                            self.dirty = true;
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
                        ui.separator();
                        if ui
                            .checkbox(&mut self.focus_mode, self.i18n.t("focus-mode-toggle"))
                            .changed()
                        {
                            ui.close_menu();
                        }
                    });
                });

                ui.separator();

                // Element type selector toolbar
                ui.horizontal(|ui| {
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
                                    self.dirty = true;
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
                                    if let Some(el) =
                                        self.doc.elements.get_mut(self.editor.focused_index)
                                    {
                                        el.revision = val.map(|c| RevisionMark {
                                            color: c.to_string(),
                                            label: String::new(),
                                        });
                                        self.dirty = true;
                                    }
                                }
                            }
                        });

                    ui.separator();
                    if ui
                        .selectable_label(self.show_navigator, self.i18n.t("nav-toggle"))
                        .clicked()
                    {
                        self.show_navigator = !self.show_navigator;
                    }
                    if ui
                        .selectable_label(self.show_notes_panel, self.i18n.t("notes-toggle"))
                        .clicked()
                    {
                        self.show_notes_panel = !self.show_notes_panel;
                    }
                    if ui
                        .selectable_label(self.focus_mode, self.i18n.t("focus-mode-toggle"))
                        .clicked()
                    {
                        self.focus_mode = !self.focus_mode;
                    }
                });
            });

            // Bottom status bar
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
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

                    let profile = LayoutProfile::for_page_size(self.doc.settings.page_size);
                    let paged = paginate(&self.doc, &profile);
                    let mut page_args = FluentArgs::new();
                    page_args.set("pages", paged.page_count());
                    page_args.set("minutes", paged.estimated_minutes());
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

            // Sidebars (hidden in focus mode)
            self.show_scene_navigator(ctx);
            self.show_notes_sidebar(ctx);
        }

        // Main screenplay editor canvas
        let desk_bg = theme::paper_palette(self.dark_theme).desk_bg;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(desk_bg))
            .show(ctx, |ui| {
                if self.editor.show(
                    ui,
                    &mut self.doc,
                    &mut self.undo,
                    &self.i18n,
                    self.dark_theme,
                ) {
                    self.dirty = true;
                }
            });

        self.show_title_page_window(ctx);
        self.show_find_replace_window(ctx);
        self.show_print_window(ctx);
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
