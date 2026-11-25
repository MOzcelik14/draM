//! Crash-recovery coordination, snapshot validation, and unsaved close prompts.

use std::fs;
use std::path::PathBuf;

use dram_format::load_dram;
use eframe::egui;
use fluent_bundle::FluentArgs;

use crate::app::DramApp;
use crate::editor::EditorState;

impl DramApp {
    pub(crate) fn wait_for_autosave_and_clear_recovery(&mut self) {
        let base = self.active_recovery_base_path();
        let unsaved = std::env::temp_dir().join("dram_unsaved.dram");
        let _ = self.recovery.invalidate_and_remove(&base);
        let _ = self.recovery.invalidate_and_remove(&unsaved);
        if let Some(handle) = self.autosave_handle.take() {
            let _ = handle.join();
        }
        let _ = self.recovery.invalidate_and_remove(&base);
        let _ = self.recovery.invalidate_and_remove(&unsaved);
    }

    pub(crate) fn active_recovery_base_path(&self) -> PathBuf {
        self.current_path
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("dram_unsaved.dram"))
    }

    pub(crate) fn show_unsaved_close_window(&mut self, ctx: &egui::Context) {
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
                        self.wait_for_autosave_and_clear_recovery();
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

    pub(crate) fn show_recovery_window(&mut self, ctx: &egui::Context) {
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
}
