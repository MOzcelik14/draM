//! Modal and floating utility windows (print, find/replace, title page, reports, help).

use std::fs;

use dram_core::{breakdown_to_csv, generate_breakdown, LayoutProfile};
use dram_print::{
    list_printers, open_pdf_in_default_viewer, print_document, render_temp_pdf, PrintError,
};
use eframe::egui;
use fluent_bundle::FluentArgs;

use crate::app::{DramApp, ReportTab};
use crate::theme;

impl DramApp {
    pub(crate) fn open_print_dialog(&mut self) {
        self.available_printers = list_printers();
        self.print_status = None;
        self.last_print_pdf = None;
        self.show_print_dialog = true;
    }

    pub(crate) fn show_print_window(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn show_find_replace_window(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn ensure_asset_texture(
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

    pub(crate) fn show_asset_preview_window(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn show_title_page_window(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn show_shortcuts_window(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn show_about_window(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn show_reports_window(&mut self, ctx: &egui::Context) {
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
