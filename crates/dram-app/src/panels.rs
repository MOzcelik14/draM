//! Right-docked inspector panel and tabs (Format, Characters, Locations, Notes).

use std::fs;

use chrono::Utc;
use dram_core::text::cmp_tr;
use dram_core::{EditCommand, ElementType, Note, TextRun};
use eframe::egui;
use fluent_bundle::FluentArgs;
use uuid::Uuid;

use crate::app::{DramApp, InspectorTab};
use crate::editor::{element_type_i18n_key, ELEMENT_TYPES};
use crate::theme;
use crate::workspace::element_page_number;

impl DramApp {
    pub(crate) fn show_right_dock(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn render_format_tab(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn render_characters_tab(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn render_locations_tab(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn render_notes_tab(&mut self, ui: &mut egui::Ui) {
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
}
