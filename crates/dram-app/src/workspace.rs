//! Beat Board workspace view, page lookup, and Linux IME event normalization.

use dram_core::{EditCommand, Element, ElementType, TextRun};
use eframe::egui;
use fluent_bundle::FluentArgs;

use crate::app::{DramApp, ViewMode};
use crate::theme;

impl DramApp {
    pub(crate) fn show_beat_board(&mut self, ui: &mut egui::Ui) {
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
}

/// Converts orphaned `ImeEvent::Commit` events (sent by Linux X11/ibus for Turkish keys
/// such as `ü`, `ş`, `ğ` without a preceding `Preedit`) into `Event::Text` so `egui::TextEdit`
/// never drops them.
pub(crate) fn normalize_ime_events_in_vec(events: &mut [egui::Event]) {
    let has_preedit = events.iter().any(|ev| {
        matches!(
            ev,
            egui::Event::Ime(egui::ImeEvent::Preedit(s)) if !s.is_empty()
        )
    });
    if has_preedit {
        return;
    }
    for ev in events.iter_mut() {
        if let egui::Event::Ime(egui::ImeEvent::Commit(text)) = ev {
            let committed = std::mem::take(text);
            if !committed.is_empty() {
                *ev = egui::Event::Text(committed);
            }
        }
    }
}

/// Normalizes Linux X11/ibus IME commit events on the current frame's `egui::Context`.
pub(crate) fn normalize_linux_ime_events(ctx: &egui::Context) {
    ctx.input_mut(|input| normalize_ime_events_in_vec(&mut input.events));
}

pub(crate) fn element_page_number(
    paged: &dram_core::PaginatedDocument,
    element_idx: usize,
) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphaned_turkish_ime_commit_converts_to_text_event() {
        let mut events = vec![
            egui::Event::Ime(egui::ImeEvent::Commit("ü".to_string())),
            egui::Event::Ime(egui::ImeEvent::Commit("ş".to_string())),
            egui::Event::Ime(egui::ImeEvent::Commit("ğ".to_string())),
            egui::Event::Ime(egui::ImeEvent::Commit("İ".to_string())),
        ];
        normalize_ime_events_in_vec(&mut events);
        assert_eq!(
            events,
            vec![
                egui::Event::Text("ü".to_string()),
                egui::Event::Text("ş".to_string()),
                egui::Event::Text("ğ".to_string()),
                egui::Event::Text("İ".to_string()),
            ]
        );
    }

    #[test]
    fn ime_commit_with_active_preedit_is_preserved() {
        let mut events = vec![
            egui::Event::Ime(egui::ImeEvent::Preedit("s".to_string())),
            egui::Event::Ime(egui::ImeEvent::Commit("ş".to_string())),
        ];
        normalize_ime_events_in_vec(&mut events);
        assert!(matches!(
            &events[1],
            egui::Event::Ime(egui::ImeEvent::Commit(s)) if s == "ş"
        ));
    }
}
