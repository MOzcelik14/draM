//! Custom screenplay editor widget with Final Draft-style Tab/Enter flow.

use dram_core::text::{cmp_tr, to_lower_for_lang, to_upper_for_lang};
use dram_core::{Document, DualSide, EditCommand, Element, ElementType, TextRun, UndoManager};
use eframe::egui;

use crate::i18n::I18n;
use crate::theme::paper_palette;

/// All selectable screenplay element types for the toolbar and shortcuts.
pub const ELEMENT_TYPES: [ElementType; 14] = [
    ElementType::SceneHeading,
    ElementType::Action,
    ElementType::Character,
    ElementType::Parenthetical,
    ElementType::Dialogue,
    ElementType::Transition,
    ElementType::Shot,
    ElementType::General,
    ElementType::Centered,
    ElementType::Lyrics,
    ElementType::PageBreak,
    ElementType::Section(1),
    ElementType::Synopsis,
    ElementType::Note,
];

/// Returns the Fluent i18n key for an [`ElementType`].
#[must_use]
pub const fn element_type_i18n_key(kind: ElementType) -> &'static str {
    match kind {
        ElementType::SceneHeading => "elem-scene-heading",
        ElementType::Action => "elem-action",
        ElementType::Character => "elem-character",
        ElementType::Parenthetical => "elem-parenthetical",
        ElementType::Dialogue => "elem-dialogue",
        ElementType::Transition => "elem-transition",
        ElementType::Shot => "elem-shot",
        ElementType::General => "elem-general",
        ElementType::Centered => "elem-centered",
        ElementType::Lyrics => "elem-lyrics",
        ElementType::PageBreak => "elem-page-break",
        ElementType::Section(_) => "elem-section",
        ElementType::Synopsis => "elem-synopsis",
        ElementType::Note => "elem-note",
    }
}

/// Mutable state for the screenplay editor canvas.
pub struct EditorState {
    /// Index of the currently focused element.
    pub focused_index: usize,
    /// Element index that should receive keyboard focus on the next frame.
    pub pending_focus: Option<usize>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            focused_index: 0,
            pending_focus: Some(0),
        }
    }
}

impl EditorState {
    /// Renders the screenplay page and handles Tab/Enter/Backspace/Ctrl+1..8 editing flow.
    ///
    /// Returns `true` if the document was modified.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut Document,
        undo: &mut UndoManager,
        i18n: &I18n,
        dark_theme: bool,
    ) -> bool {
        let palette = paper_palette(dark_theme);
        let mut modified = false;
        if doc.elements.is_empty() {
            doc.elements
                .push(Element::new(ElementType::SceneHeading, ""));
        }
        if self.focused_index >= doc.elements.len() {
            self.focused_index = doc.elements.len().saturating_sub(1);
        }

        // Handle Ctrl+1..8 quick element type shortcuts
        if let Some(new_kind) = detect_ctrl_digit_shortcut(ui.ctx()) {
            if let Some(el) = doc.elements.get(self.focused_index) {
                if el.kind != new_kind {
                    let old_kind = el.kind;
                    if undo
                        .execute(
                            doc,
                            EditCommand::ChangeElementType {
                                index: self.focused_index,
                                old_kind,
                                new_kind,
                            },
                        )
                        .is_ok()
                    {
                        modified = true;
                    }
                }
            }
        }

        // Autocomplete suggestion bar for Character, SceneHeading, and Transition
        let suggestions = autocomplete_candidates(doc, self.focused_index);
        if !suggestions.is_empty() {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(i18n.t("autocomplete-suggestions"))
                                .small()
                                .weak(),
                        );
                        for cand in suggestions.into_iter().take(6) {
                            if ui.small_button(&cand).clicked() {
                                let old_runs = doc.elements[self.focused_index].runs.clone();
                                let new_runs = vec![TextRun::plain(cand)];
                                if old_runs != new_runs
                                    && undo
                                        .execute(
                                            doc,
                                            EditCommand::UpdateRuns {
                                                index: self.focused_index,
                                                old_runs,
                                                new_runs,
                                            },
                                        )
                                        .is_ok()
                                {
                                    self.pending_focus = Some(self.focused_index);
                                    modified = true;
                                }
                            }
                        }
                    });
                });
            ui.separator();
        }

        let mut action_after_loop: Option<EditorAction> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail_width = ui.available_width();
                let sheet_width = avail_width.clamp(320.0, 820.0);
                let outer_pad = ((avail_width - sheet_width) * 0.5).max(12.0);
                let inner_pad_x = 44.0_f32;
                let content_width = (sheet_width - inner_pad_x * 2.0).max(200.0);

                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.add_space(outer_pad);
                    egui::Frame::new()
                        .fill(palette.paper_bg)
                        .stroke(egui::Stroke::new(1.0_f32, palette.paper_border))
                        .corner_radius(egui::CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(inner_pad_x as i8, 36))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(content_width);
                                ui.set_min_height(680.0);

                                for idx in 0..doc.elements.len() {
                                    let el = &doc.elements[idx];
                                    let el_id = el.id;
                                    let kind = el.kind;
                                    let is_focused = idx == self.focused_index;
                                    let dual_side = el.dual_dialogue.map(|d| d.side);
                                    let rev_color = el.revision.as_ref().map(|r| r.color.clone());
                                    let scene_num = if doc.settings.show_scene_numbers
                                        && kind == ElementType::SceneHeading
                                    {
                                        el.scene_number.as_ref().map(|s| s.value.clone())
                                    } else {
                                        None
                                    };
                                    let mut text = el.plain_text();
                                    let script_lang = doc.settings.script_language;

                                    let (left_indent, right_indent, align) = element_layout(kind);
                                    let row_fill = if is_focused {
                                        palette.active_row_bg
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };

                                    egui::Frame::new()
                                        .fill(row_fill)
                                        .corner_radius(egui::CornerRadius::same(3))
                                        .inner_margin(egui::Margin::symmetric(6, 3))
                                        .show(ui, |ui| {
                                            ui.set_width(content_width - 12.0);
                                            ui.horizontal(|ui| {
                                                let gutter_w = 34.0;
                                                ui.allocate_ui_with_layout(
                                                    egui::vec2(gutter_w, 18.0),
                                                    egui::Layout::left_to_right(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if let Some(color_name) = &rev_color {
                                                            let c = revision_egui_color(color_name);
                                                            let (rect, _) = ui.allocate_exact_size(
                                                                egui::vec2(3.0, 16.0),
                                                                egui::Sense::hover(),
                                                            );
                                                            ui.painter().rect_filled(rect, 1.0, c);
                                                        }
                                                        if let Some(sn) = &scene_num {
                                                            ui.label(
                                                                egui::RichText::new(sn)
                                                                    .monospace()
                                                                    .small()
                                                                    .color(palette.muted_text),
                                                            );
                                                        }
                                                    },
                                                );

                                                ui.add_space(left_indent);
                                                if kind == ElementType::Character {
                                                    if let Some(side) = dual_side {
                                                        let badge_key = match side {
                                                            DualSide::Left => "dual-left-badge",
                                                            DualSide::Right => "dual-right-badge",
                                                        };
                                                        ui.label(
                                                            egui::RichText::new(i18n.t(badge_key))
                                                                .small()
                                                                .color(palette.muted_text),
                                                        );
                                                    }
                                                }
                                                let field_width = (content_width
                                                    - gutter_w
                                                    - 20.0
                                                    - left_indent
                                                    - right_indent)
                                                    .max(120.0);

                                                if kind == ElementType::PageBreak {
                                                    ui.vertical(|ui| {
                                                        ui.set_width(field_width);
                                                        ui.separator();
                                                        ui.label(
                                                            egui::RichText::new(
                                                                i18n.t(element_type_i18n_key(kind)),
                                                            )
                                                            .small()
                                                            .color(palette.muted_text),
                                                        );
                                                    });
                                                    return;
                                                }

                                                let font_id = if matches!(
                                                    kind,
                                                    ElementType::SceneHeading
                                                        | ElementType::Section(_)
                                                ) {
                                                    egui::FontId::monospace(15.5)
                                                } else {
                                                    egui::FontId::monospace(15.0)
                                                };

                                                let widget_id =
                                                    ui.make_persistent_id(("dram_elem", el_id));
                                                let text_color = if kind == ElementType::Synopsis
                                                    || kind == ElementType::Note
                                                {
                                                    palette.muted_text
                                                } else {
                                                    palette.paper_ink
                                                };
                                                let hint = if is_focused {
                                                    i18n.t(element_type_i18n_key(kind))
                                                } else {
                                                    String::new()
                                                };
                                                let edit = egui::TextEdit::multiline(&mut text)
                                                    .id(widget_id)
                                                    .desired_rows(1)
                                                    .desired_width(field_width)
                                                    .frame(false)
                                                    .font(font_id)
                                                    .horizontal_align(align)
                                                    .text_color(text_color)
                                                    .hint_text(
                                                        egui::RichText::new(hint)
                                                            .color(palette.muted_text),
                                                    );

                                                let response = ui.add(edit);

                                                if self.pending_focus == Some(idx) {
                                                    response.request_focus();
                                                    response
                                                        .scroll_to_me(Some(egui::Align::Center));
                                                    self.pending_focus = None;
                                                }

                                                if response.has_focus() {
                                                    self.focused_index = idx;

                                                    let (
                                                        enter_pressed,
                                                        tab_pressed,
                                                        backspace_empty,
                                                    ) = ui.input_mut(|input| {
                                                        let enter = input.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::Enter,
                                                        );
                                                        let tab = input.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::Tab,
                                                        );
                                                        let back = text.is_empty()
                                                            && input
                                                                .key_pressed(egui::Key::Backspace);
                                                        (enter, tab, back)
                                                    });

                                                    if enter_pressed {
                                                        action_after_loop =
                                                            Some(EditorAction::InsertAfter(idx));
                                                    } else if tab_pressed {
                                                        action_after_loop =
                                                            Some(EditorAction::CycleType(idx));
                                                    } else if backspace_empty
                                                        && doc.elements.len() > 1
                                                    {
                                                        action_after_loop =
                                                            Some(EditorAction::DeleteEmpty(idx));
                                                    }
                                                }

                                                if response.changed() {
                                                    let cleaned = text.replace('\n', "");
                                                    let final_text =
                                                        if kind.is_uppercase_convention() {
                                                            to_upper_for_lang(&cleaned, script_lang)
                                                        } else {
                                                            cleaned
                                                        };
                                                    let old_runs = doc.elements[idx].runs.clone();
                                                    let new_runs = if final_text.is_empty() {
                                                        Vec::new()
                                                    } else {
                                                        vec![TextRun::plain(final_text)]
                                                    };
                                                    if old_runs != new_runs
                                                        && undo
                                                            .execute(
                                                                doc,
                                                                EditCommand::UpdateRuns {
                                                                    index: idx,
                                                                    old_runs,
                                                                    new_runs,
                                                                },
                                                            )
                                                            .is_ok()
                                                    {
                                                        modified = true;
                                                    }
                                                }
                                            });
                                        });
                                    ui.add_space(4.0);
                                }
                            });
                        });
                });
                ui.add_space(40.0);
            });

        if let Some(act) = action_after_loop {
            match act {
                EditorAction::InsertAfter(idx) => {
                    undo.break_coalescing();
                    let next_kind = doc.elements[idx].kind.next_on_enter();
                    let insert_idx = idx + 1;
                    if undo
                        .execute(
                            doc,
                            EditCommand::InsertElement {
                                index: insert_idx,
                                element: Element::new(next_kind, ""),
                            },
                        )
                        .is_ok()
                    {
                        self.focused_index = insert_idx;
                        self.pending_focus = Some(insert_idx);
                        modified = true;
                    }
                }
                EditorAction::CycleType(idx) => {
                    undo.break_coalescing();
                    let old_kind = doc.elements[idx].kind;
                    let new_kind = old_kind.next_on_tab();
                    if old_kind != new_kind
                        && undo
                            .execute(
                                doc,
                                EditCommand::ChangeElementType {
                                    index: idx,
                                    old_kind,
                                    new_kind,
                                },
                            )
                            .is_ok()
                    {
                        modified = true;
                    }
                }
                EditorAction::DeleteEmpty(idx) => {
                    undo.break_coalescing();
                    let removed = doc.elements[idx].clone();
                    if undo
                        .execute(
                            doc,
                            EditCommand::DeleteElement {
                                index: idx,
                                removed,
                            },
                        )
                        .is_ok()
                    {
                        let target = idx.saturating_sub(1);
                        self.focused_index = target;
                        self.pending_focus = Some(target);
                        modified = true;
                    }
                }
            }
        }

        modified
    }
}

enum EditorAction {
    InsertAfter(usize),
    CycleType(usize),
    DeleteEmpty(usize),
}

fn element_layout(kind: ElementType) -> (f32, f32, egui::Align) {
    match kind {
        ElementType::SceneHeading
        | ElementType::Action
        | ElementType::General
        | ElementType::Shot => (0.0, 0.0, egui::Align::Min),
        ElementType::Character => (200.0, 80.0, egui::Align::Min),
        ElementType::Parenthetical => (150.0, 140.0, egui::Align::Min),
        ElementType::Dialogue => (100.0, 100.0, egui::Align::Min),
        ElementType::Transition => (300.0, 0.0, egui::Align::Max),
        ElementType::Centered => (40.0, 40.0, egui::Align::Center),
        ElementType::Lyrics => (100.0, 100.0, egui::Align::Min),
        ElementType::PageBreak => (0.0, 0.0, egui::Align::Center),
        ElementType::Section(_) | ElementType::Synopsis | ElementType::Note => {
            (16.0, 16.0, egui::Align::Min)
        }
    }
}

fn detect_ctrl_digit_shortcut(ctx: &egui::Context) -> Option<ElementType> {
    ctx.input(|input| {
        if !input.modifiers.command {
            return None;
        }
        if input.key_pressed(egui::Key::Num1) {
            Some(ElementType::SceneHeading)
        } else if input.key_pressed(egui::Key::Num2) {
            Some(ElementType::Action)
        } else if input.key_pressed(egui::Key::Num3) {
            Some(ElementType::Character)
        } else if input.key_pressed(egui::Key::Num4) {
            Some(ElementType::Parenthetical)
        } else if input.key_pressed(egui::Key::Num5) {
            Some(ElementType::Dialogue)
        } else if input.key_pressed(egui::Key::Num6) {
            Some(ElementType::Transition)
        } else if input.key_pressed(egui::Key::Num7) {
            Some(ElementType::Shot)
        } else if input.key_pressed(egui::Key::Num8) {
            Some(ElementType::General)
        } else {
            None
        }
    })
}

/// Collects Turkish-collation-sorted autocomplete suggestions for the focused element.
#[must_use]
pub fn autocomplete_candidates(doc: &Document, focused_index: usize) -> Vec<String> {
    let Some(current) = doc.elements.get(focused_index) else {
        return Vec::new();
    };
    let lang = doc.settings.script_language;
    let query = current.plain_text();
    let query_low = to_lower_for_lang(query.trim(), lang);

    let mut pool: Vec<String> = Vec::new();
    match current.kind {
        ElementType::Character => {
            for (i, el) in doc.elements.iter().enumerate() {
                if i != focused_index && el.kind == ElementType::Character {
                    let t = el.plain_text().trim().to_string();
                    if !t.is_empty() && !pool.contains(&t) {
                        pool.push(t);
                    }
                }
            }
        }
        ElementType::SceneHeading => {
            for prefix in &doc.settings.dictionary.scene_prefixes {
                let p = format!("{prefix} ");
                if !pool.contains(&p) {
                    pool.push(p);
                }
            }
            for (i, el) in doc.elements.iter().enumerate() {
                if i != focused_index && el.kind == ElementType::SceneHeading {
                    let t = el.plain_text().trim().to_string();
                    if !t.is_empty() && !pool.contains(&t) {
                        pool.push(t);
                    }
                }
            }
        }
        ElementType::Transition => {
            for tr in &doc.settings.dictionary.transitions {
                if !pool.contains(tr) {
                    pool.push(tr.clone());
                }
            }
        }
        _ => return Vec::new(),
    }

    pool.retain(|cand| {
        let cand_low = to_lower_for_lang(cand, lang);
        cand_low != query_low && (query_low.is_empty() || cand_low.starts_with(&query_low))
    });
    pool.sort_by(|a, b| cmp_tr(a, b));
    pool
}

fn revision_egui_color(name: &str) -> egui::Color32 {
    match name {
        "blue" => egui::Color32::from_rgb(90, 160, 255),
        "pink" => egui::Color32::from_rgb(245, 130, 190),
        "yellow" => egui::Color32::from_rgb(240, 210, 75),
        "green" => egui::Color32::from_rgb(100, 210, 120),
        _ => egui::Color32::GRAY,
    }
}
