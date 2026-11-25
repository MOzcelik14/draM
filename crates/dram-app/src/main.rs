//! `dram-app`: Desktop GUI for `draM.` screenplay writer.

#![forbid(unsafe_code)]

mod app;
mod commands;
mod dialogs;
mod document;
mod editor;
mod i18n;
mod panels;
mod recovery;
mod settings;
mod theme;
mod workspace;

use std::path::PathBuf;

use anyhow::Result;
use eframe::egui;

use crate::app::DramApp;
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

#[cfg(test)]
mod tests {
    #[test]
    fn ui_sources_contain_no_hardcoded_user_labels() {
        // Verify that common UI labels are routed through i18n keys rather than hardcoded literals.
        let sources = [
            include_str!("main.rs"),
            include_str!("app.rs"),
            include_str!("commands.rs"),
            include_str!("dialogs.rs"),
            include_str!("document.rs"),
            include_str!("editor.rs"),
            include_str!("panels.rs"),
            include_str!("recovery.rs"),
            include_str!("settings.rs"),
            include_str!("workspace.rs"),
        ];
        for src in sources {
            for forbidden in [
                "\"Dosya\"",
                "\"Kaydet\"",
                "\"File\"",
                "\"Save\"",
                "\"Undo\"",
                "\"Geri Al\"",
            ] {
                assert!(
                    !src.contains(forbidden),
                    "found hardcoded UI string {forbidden}"
                );
            }
        }
    }
}
