//! Core `DramApp` state and `eframe::App` frame loop.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dram_core::{
    paginate, BuiltinSpellChecker, Document, EditCommand, ElementType, LayoutProfile, PageSize,
    PaginatedDocument, ProjectTemplate, RevisionMark, ScriptLanguage, SpellChecker, UndoManager,
};
use dram_format::{recovery_path_for, RecoveryCoordinator};
use dram_print::PrintOptions;
use eframe::egui;
use fluent_bundle::FluentArgs;

use crate::editor::{element_type_i18n_key, EditorState, ELEMENT_TYPES};
use crate::i18n::{I18n, UiLanguage};
use crate::settings::{load_recent_files, load_user_dict, save_user_dict};
use crate::theme;
use crate::workspace::normalize_linux_ime_events;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    Format,
    Characters,
    Locations,
    Notes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    Editor,
    BeatBoard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportTab {
    Characters,
    Locations,
}

pub(crate) struct DramApp {
    pub(crate) doc: Document,
    pub(crate) undo: UndoManager,
    pub(crate) editor: EditorState,
    pub(crate) i18n: I18n,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) dirty: bool,
    pub(crate) status_error: Option<String>,
    pub(crate) show_title_page_editor: bool,
    pub(crate) show_shortcuts_dialog: bool,
    pub(crate) show_about_dialog: bool,
    pub(crate) show_unsaved_dialog: bool,
    pub(crate) allow_close: bool,
    pub(crate) show_navigator: bool,
    pub(crate) inspector_tab: InspectorTab,
    pub(crate) view_mode: ViewMode,
    pub(crate) revision_mode: bool,
    pub(crate) active_revision_color: String,
    pub(crate) show_reports_dialog: bool,
    pub(crate) report_tab: ReportTab,
    pub(crate) report_status: Option<String>,
    pub(crate) dark_theme: bool,
    pub(crate) applied_dark_theme: Option<bool>,
    pub(crate) focus_mode: bool,
    pub(crate) show_find_replace: bool,
    pub(crate) find_query: String,
    pub(crate) replace_text: String,
    pub(crate) find_case_sensitive: bool,
    pub(crate) find_status: Option<String>,
    pub(crate) show_print_dialog: bool,
    pub(crate) available_printers: Vec<String>,
    pub(crate) print_options: PrintOptions,
    pub(crate) print_page_range_input: String,
    pub(crate) last_print_pdf: Option<PathBuf>,
    pub(crate) print_status: Option<String>,
    pub(crate) cached_pagination: PaginatedDocument,
    pub(crate) pagination_dirty: bool,
    pub(crate) last_autosave: Instant,
    pub(crate) autosave_in_flight: Arc<AtomicBool>,
    pub(crate) autosave_handle: Option<std::thread::JoinHandle<()>>,
    pub(crate) recovery: Arc<RecoveryCoordinator>,
    pub(crate) pending_recovery_path: Option<PathBuf>,
    pub(crate) recent_files: Vec<PathBuf>,
    pub(crate) spellcheck_enabled: bool,
    pub(crate) spellchecker: BuiltinSpellChecker,
    pub(crate) asset_textures: HashMap<String, egui::TextureHandle>,
    pub(crate) preview_asset_path: Option<String>,
}

impl DramApp {
    pub(crate) fn new(i18n: I18n, initial_file: Option<PathBuf>) -> Self {
        let script_lang = match i18n.active() {
            UiLanguage::Tr => ScriptLanguage::Tr,
            UiLanguage::En => ScriptLanguage::En,
        };
        let doc = Document::new(script_lang);
        let profile = LayoutProfile::for_page_size(doc.settings.page_size);
        let cached_pagination = paginate(&doc, &profile);
        let unsaved_rec = recovery_path_for(&std::env::temp_dir().join("dram_unsaved.dram"));
        let pending_recovery_path = (unsaved_rec.exists()
            && RecoveryCoordinator::validate_recovery_file(&unsaved_rec, None))
        .then_some(unsaved_rec);
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
            autosave_handle: None,
            recovery: Arc::new(RecoveryCoordinator::new()),
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

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
        self.pagination_dirty = true;
        self.recovery.bump_generation();
    }

    pub(crate) fn ensure_pagination(&mut self) {
        if self.pagination_dirty {
            let profile = LayoutProfile::for_page_size(self.doc.settings.page_size);
            self.cached_pagination = paginate(&self.doc, &profile);
            self.spellchecker.sync_document_terms(&self.doc);
            self.pagination_dirty = false;
        }
    }
}

impl eframe::App for DramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self
            .autosave_handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            if let Some(handle) = self.autosave_handle.take() {
                let _ = handle.join();
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.dirty && !self.allow_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_unsaved_dialog = true;
            } else {
                self.wait_for_autosave_and_clear_recovery();
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
            let snapshot_gen = self.recovery.current_generation();
            let recovery = Arc::clone(&self.recovery);
            let flag = Arc::clone(&self.autosave_in_flight);
            self.last_autosave = Instant::now();
            self.autosave_handle = Some(std::thread::spawn(move || {
                if let Err(err) = recovery.try_write_recovery(&base, &doc_snapshot, snapshot_gen) {
                    eprintln!("[autosave] failed to write recovery: {err}");
                }
                flag.store(false, Ordering::Release);
            }));
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
