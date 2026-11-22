//! Custom screenplay editor widget with Final Draft-style Tab/Enter flow.

use std::collections::HashMap;

use dram_core::text::{cmp_tr, to_lower_for_lang, to_upper_for_lang};
use dram_core::{
    BuiltinSpellChecker, Document, DualSide, EditCommand, Element, ElementType, InlineStyle,
    PaginatedDocument, RevisionMark, ScriptLanguage, SpellChecker, TextRun, UndoManager,
};
use eframe::egui;

use crate::i18n::I18n;
use crate::theme::{font_family_for_style, paper_palette};

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
    /// Last known `egui::Id` of the focused element's `TextEdit`, used for selection-range formatting.
    focused_widget_id: Option<egui::Id>,
    /// Highlighted suggestion index in the inline autocomplete popup.
    autocomplete_selected: usize,
    /// Element index for which the user explicitly dismissed the autocomplete popup via `Escape`.
    autocomplete_dismissed_for: Option<usize>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            focused_index: 0,
            pending_focus: Some(0),
            focused_widget_id: None,
            autocomplete_selected: 0,
            autocomplete_dismissed_for: None,
        }
    }
}

impl EditorState {
    /// Renders the screenplay page and handles Tab/Enter/Backspace/Ctrl+1..8 editing flow.
    ///
    /// Returns `true` if the document was modified.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut Document,
        undo: &mut UndoManager,
        paged: &PaginatedDocument,
        active_revision: Option<&RevisionMark>,
        spell: Option<&BuiltinSpellChecker>,
        added_dict_word: &mut Option<String>,
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
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(egui::Margin::symmetric(14, 7))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(
                            egui::RichText::new(i18n.t("autocomplete-suggestions"))
                                .size(13.0)
                                .color(ui.visuals().text_color()),
                        );
                        for cand in suggestions.into_iter().take(6) {
                            let btn = egui::Button::new(
                                egui::RichText::new(&cand).monospace().strong().size(13.0),
                            )
                            .min_size(egui::vec2(0.0, 26.0));
                            if ui.add(btn).clicked() {
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

        if let Some(attr) = detect_inline_format_shortcut(ui.ctx()) {
            let char_range = self
                .focused_widget_id
                .and_then(|id| egui::TextEdit::load_state(ui.ctx(), id))
                .and_then(|state| state.cursor.char_range())
                .and_then(|range| {
                    let start = range.primary.index.min(range.secondary.index);
                    let end = range.primary.index.max(range.secondary.index);
                    (start < end).then_some(start..end)
                });
            if toggle_element_inline_style_range(doc, undo, self.focused_index, attr, char_range) {
                modified = true;
            }
        }

        let mut page_starts: HashMap<usize, usize> = HashMap::new();
        for page in &paged.pages {
            if page.page_number > 1 {
                if let Some(first_idx) = page.lines.iter().find_map(|l| l.element_index) {
                    page_starts.entry(first_idx).or_insert(page.page_number);
                }
            }
        }

        let total_elements = doc.elements.len();
        let (window_start, window_end) = compute_element_window(total_elements, self.focused_index);
        const EST_ROW_HEIGHT: f32 = 28.0;

        let mut action_after_loop: Option<EditorAction> = None;
        let mut focused_rect: Option<egui::Rect> = None;

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

                                if window_start > 0 {
                                    ui.add_space(window_start as f32 * EST_ROW_HEIGHT);
                                }

                                for idx in window_start..window_end {
                                    if let Some(&page_num) = page_starts.get(&idx) {
                                        draw_page_break_divider(
                                            ui,
                                            content_width,
                                            page_num,
                                            palette.muted_text,
                                        );
                                    }

                                    let el = &doc.elements[idx];
                                    let el_id = el.id;
                                    let el_runs = el.runs.clone();
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

                                                let default_bold = matches!(
                                                    kind,
                                                    ElementType::SceneHeading
                                                        | ElementType::Section(_)
                                                );
                                                let font_size =
                                                    if default_bold { 15.5 } else { 15.0 };

                                                let widget_id =
                                                    ui.make_persistent_id(("dram_elem", el_id));
                                                if is_focused {
                                                    self.focused_widget_id = Some(widget_id);
                                                }
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
                                                let check_spelling = matches!(
                                                    kind,
                                                    ElementType::Action
                                                        | ElementType::Dialogue
                                                        | ElementType::Parenthetical
                                                        | ElementType::General
                                                        | ElementType::Synopsis
                                                        | ElementType::Note
                                                );
                                                let mut layouter =
                                                    |ui: &egui::Ui, buf: &str, wrap_width: f32| {
                                                        let job = build_element_layout_job(
                                                            &el_runs,
                                                            buf,
                                                            font_size,
                                                            default_bold,
                                                            text_color,
                                                            wrap_width,
                                                            if check_spelling { spell } else { None },
                                                            script_lang,
                                                        );
                                                        ui.fonts(|f| f.layout_job(job))
                                                    };
                                                let edit = egui::TextEdit::multiline(&mut text)
                                                    .id(widget_id)
                                                    .desired_rows(1)
                                                    .desired_width(field_width)
                                                    .frame(false)
                                                    .layouter(&mut layouter)
                                                    .horizontal_align(align)
                                                    .hint_text(
                                                        egui::RichText::new(hint)
                                                            .color(palette.muted_text),
                                                    );

                                                let response = ui.add(edit);

                                                if check_spelling
                                                    && (response.secondary_clicked()
                                                        || response.context_menu_opened())
                                                {
                                                    if let Some(checker) = spell {
                                                        let mut replace_req: Option<(String, String)> =
                                                            None;
                                                        let misspelled =
                                                            collect_misspelled_words(
                                                                &text,
                                                                checker,
                                                                script_lang,
                                                            );
                                                        if !misspelled.is_empty() {
                                                            response.context_menu(|ui| {
                                                                for bad in misspelled.iter().take(3)
                                                                {
                                                                    ui.strong(bad);
                                                                    let suggs = checker
                                                                        .suggestions(
                                                                            bad,
                                                                            script_lang,
                                                                        );
                                                                    if suggs.is_empty() {
                                                                        ui.label(
                                                                            egui::RichText::new(
                                                                                i18n.t(
                                                                                    "spell-no-suggestions",
                                                                                ),
                                                                            )
                                                                            .small()
                                                                            .weak(),
                                                                        );
                                                                    } else {
                                                                        for s in suggs {
                                                                            if ui
                                                                                .button(&s)
                                                                                .clicked()
                                                                            {
                                                                                replace_req = Some(
                                                                                    (
                                                                                        bad.clone(),
                                                                                        s,
                                                                                    ),
                                                                                );
                                                                                ui.close_menu();
                                                                            }
                                                                        }
                                                                    }
                                                                    if ui
                                                                        .button(
                                                                            i18n.t(
                                                                                "spell-add-dict",
                                                                            ),
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        *added_dict_word =
                                                                            Some(bad.clone());
                                                                        ui.close_menu();
                                                                    }
                                                                    ui.separator();
                                                                }
                                                            });
                                                        }
                                                        if let Some((bad, good)) = replace_req {
                                                            let updated =
                                                                text.replacen(&bad, &good, 1);
                                                            let old_runs =
                                                                doc.elements[idx].runs.clone();
                                                            let new_runs =
                                                                update_runs_preserving_styles(
                                                                    &old_runs,
                                                                    &updated,
                                                                );
                                                            undo.break_coalescing();
                                                            if undo
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
                                                                if let Some(rev) = active_revision {
                                                                    doc.elements[idx].revision =
                                                                        Some(rev.clone());
                                                                }
                                                                modified = true;
                                                            }
                                                        }
                                                    }
                                                }

                                                if self.pending_focus == Some(idx) {
                                                    response.request_focus();
                                                    response
                                                        .scroll_to_me(Some(egui::Align::Center));
                                                    ui.ctx().request_repaint();
                                                    self.pending_focus = None;
                                                }

                                                if response.has_focus() {
                                                    if self.focused_index != idx {
                                                        self.autocomplete_selected = 0;
                                                        self.autocomplete_dismissed_for = None;
                                                    }
                                                    self.focused_index = idx;
                                                    self.focused_widget_id = Some(widget_id);
                                                    focused_rect = Some(response.rect);

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
                                                    self.autocomplete_dismissed_for = None;
                                                    let cleaned = text.replace('\n', "");
                                                    let final_text =
                                                        if kind.is_uppercase_convention() {
                                                            to_upper_for_lang(&cleaned, script_lang)
                                                        } else {
                                                            cleaned
                                                        };
                                                    let old_runs = doc.elements[idx].runs.clone();
                                                    let new_runs = update_runs_preserving_styles(
                                                        &old_runs,
                                                        &final_text,
                                                    );
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
                                                        if let Some(rev) = active_revision {
                                                            doc.elements[idx].revision =
                                                                Some(rev.clone());
                                                        }
                                                        modified = true;
                                                    }
                                                }
                                            });
                                        });
                                    ui.add_space(4.0);
                                }

                                if window_end < total_elements {
                                    ui.add_space(
                                        (total_elements - window_end) as f32 * EST_ROW_HEIGHT,
                                    );
                                }
                            });
                        });
                });
                ui.add_space(40.0);
            });

        // Inline autocomplete popup directly beneath the focused Character / SceneHeading / Transition
        if let Some(anchor_rect) = focused_rect {
            if self.autocomplete_dismissed_for != Some(self.focused_index) {
                let current_text = doc
                    .elements
                    .get(self.focused_index)
                    .map(|e| e.plain_text())
                    .unwrap_or_default();
                if !current_text.trim().is_empty() {
                    let candidates = autocomplete_candidates(doc, self.focused_index);
                    let visible: Vec<String> = candidates.into_iter().take(6).collect();
                    if !visible.is_empty() {
                        if self.autocomplete_selected >= visible.len() {
                            self.autocomplete_selected = 0;
                        }
                        let (down, up, esc, accept_key) = ui.ctx().input_mut(|input| {
                            let d = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
                            let u = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
                            let e = input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                            let a = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Space);
                            (d, u, e, a)
                        });
                        if esc {
                            self.autocomplete_dismissed_for = Some(self.focused_index);
                        } else {
                            if down {
                                self.autocomplete_selected =
                                    (self.autocomplete_selected + 1) % visible.len();
                            }
                            if up {
                                self.autocomplete_selected = if self.autocomplete_selected == 0 {
                                    visible.len() - 1
                                } else {
                                    self.autocomplete_selected - 1
                                };
                            }

                            let mut chosen: Option<String> = if accept_key {
                                Some(visible[self.autocomplete_selected].clone())
                            } else {
                                None
                            };

                            egui::Area::new(ui.make_persistent_id("dram_inline_autocomplete"))
                                .order(egui::Order::Foreground)
                                .fixed_pos(anchor_rect.left_bottom() + egui::vec2(0.0, 4.0))
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style())
                                        .inner_margin(egui::Margin::same(4))
                                        .show(ui, |ui| {
                                            ui.set_min_width(220.0);
                                            for (c_idx, cand) in visible.iter().enumerate() {
                                                let selected = c_idx == self.autocomplete_selected;
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        egui::RichText::new(cand).monospace(),
                                                    )
                                                    .clicked()
                                                {
                                                    chosen = Some(cand.clone());
                                                }
                                            }
                                        });
                                });

                            if let Some(cand) = chosen {
                                let idx = self.focused_index;
                                if let Some(el) = doc.elements.get(idx) {
                                    let old_runs = el.runs.clone();
                                    let new_runs = vec![TextRun::plain(cand)];
                                    undo.break_coalescing();
                                    if undo
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
                                        if let Some(rev) = active_revision {
                                            doc.elements[idx].revision = Some(rev.clone());
                                        }
                                        self.pending_focus = Some(idx);
                                        self.autocomplete_dismissed_for = Some(idx);
                                        modified = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(act) = action_after_loop {
            match act {
                EditorAction::InsertAfter(idx) => {
                    undo.break_coalescing();
                    let next_kind = doc.elements[idx].kind.next_on_enter();
                    let insert_idx = idx + 1;
                    let mut new_el = Element::new(next_kind, "");
                    if let Some(rev) = active_revision {
                        new_el.revision = Some(rev.clone());
                    }
                    if undo
                        .execute(
                            doc,
                            EditCommand::InsertElement {
                                index: insert_idx,
                                element: new_el,
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

/// Inline text attribute toggled via keyboard shortcuts (`Ctrl+B`, `Ctrl+I`, `Ctrl+U`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineFormatAttr {
    Bold,
    Italic,
    Underline,
}

fn detect_inline_format_shortcut(ctx: &egui::Context) -> Option<InlineFormatAttr> {
    ctx.input_mut(|input| {
        if !input.modifiers.command || input.modifiers.alt || input.modifiers.shift {
            return None;
        }
        if input.consume_key(egui::Modifiers::COMMAND, egui::Key::B) {
            Some(InlineFormatAttr::Bold)
        } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::I) {
            Some(InlineFormatAttr::Italic)
        } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::U) {
            Some(InlineFormatAttr::Underline)
        } else {
            None
        }
    })
}

fn draw_page_break_divider(
    ui: &mut egui::Ui,
    content_width: f32,
    page_num: usize,
    muted_color: egui::Color32,
) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.set_width(content_width);
        let label_text = format!("{page_num}.");
        let (line_rect, _) = ui.allocate_exact_size(
            egui::vec2((content_width - 36.0).max(40.0), 14.0),
            egui::Sense::hover(),
        );
        let y = line_rect.center().y;
        let stroke = egui::Stroke::new(1.0_f32, muted_color.gamma_multiply(0.45));
        let mut x = line_rect.left();
        while x < line_rect.right() {
            let seg_end = (x + 6.0).min(line_rect.right());
            ui.painter()
                .line_segment([egui::pos2(x, y), egui::pos2(seg_end, y)], stroke);
            x += 10.0;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(label_text)
                    .monospace()
                    .small()
                    .color(muted_color),
            );
        });
    });
    ui.add_space(6.0);
}

#[allow(clippy::too_many_arguments)]
fn build_element_layout_job(
    runs: &[TextRun],
    text: &str,
    font_size: f32,
    default_bold: bool,
    text_color: egui::Color32,
    wrap_width: f32,
    spell: Option<&BuiltinSpellChecker>,
    lang: ScriptLanguage,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if text.is_empty() {
        return job;
    }

    let styled_runs = update_runs_preserving_styles(runs, text);
    for run in styled_runs {
        let mut style = run.style;
        if default_bold {
            style.bold = true;
        }
        let base_format = egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family_for_style(style)),
            color: text_color,
            underline: if style.underline {
                egui::Stroke::new(1.0_f32, text_color)
            } else {
                egui::Stroke::NONE
            },
            ..Default::default()
        };

        if let Some(checker) = spell {
            for (chunk, is_word) in split_word_chunks(&run.text) {
                let mut fmt = base_format.clone();
                if is_word && !checker.check_word(chunk, lang) {
                    fmt.underline =
                        egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(220, 60, 60));
                }
                job.append(chunk, 0.0, fmt);
            }
        } else {
            job.append(&run.text, 0.0, base_format);
        }
    }

    job
}

fn split_word_chunks(text: &str) -> Vec<(&str, bool)> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut in_word: Option<bool> = None;

    for (idx, ch) in text.char_indices() {
        let is_w = ch.is_alphabetic() || ch == '\'';
        if let Some(cur) = in_word {
            if cur != is_w {
                chunks.push((&text[start..idx], cur));
                start = idx;
                in_word = Some(is_w);
            }
        } else {
            in_word = Some(is_w);
        }
    }
    if start < text.len() {
        chunks.push((&text[start..], in_word.unwrap_or(false)));
    }
    chunks
}

fn collect_misspelled_words(
    text: &str,
    checker: &BuiltinSpellChecker,
    lang: ScriptLanguage,
) -> Vec<String> {
    let mut out = Vec::new();
    for (chunk, is_word) in split_word_chunks(text) {
        if is_word && !checker.check_word(chunk, lang) {
            let trimmed = chunk.trim_matches('\'');
            if !trimmed.is_empty() && !out.iter().any(|w| w == trimmed) {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

/// Toggles an inline style attribute either on `char_range` (if `Some(start..end)` with `start < end`)
/// or across the entire element at `index` (if `None`).
pub fn toggle_element_inline_style_range(
    doc: &mut Document,
    undo: &mut UndoManager,
    index: usize,
    attr: InlineFormatAttr,
    char_range: Option<std::ops::Range<usize>>,
) -> bool {
    let Some(range) = char_range.filter(|r| r.start < r.end) else {
        return toggle_element_inline_style(doc, undo, index, attr);
    };

    let Some(el) = doc.elements.get(index) else {
        return false;
    };
    if el.runs.is_empty() {
        return false;
    }

    let old_runs = el.runs.clone();
    // Split runs into (text, style, inside_selection) segments by char index.
    let mut segments: Vec<(String, InlineStyle, bool)> = Vec::new();
    let mut char_cursor = 0usize;

    for run in &old_runs {
        let run_chars: Vec<char> = run.text.chars().collect();
        let run_start = char_cursor;
        let run_end = run_start + run_chars.len();
        char_cursor = run_end;

        if run_end <= range.start || run_start >= range.end {
            segments.push((run.text.clone(), run.style, false));
            continue;
        }

        let sel_local_start = range.start.saturating_sub(run_start);
        let sel_local_end = range.end.min(run_end) - run_start;

        if sel_local_start > 0 {
            let pre: String = run_chars[..sel_local_start].iter().collect();
            segments.push((pre, run.style, false));
        }
        if sel_local_end > sel_local_start {
            let mid: String = run_chars[sel_local_start..sel_local_end].iter().collect();
            segments.push((mid, run.style, true));
        }
        if sel_local_end < run_chars.len() {
            let post: String = run_chars[sel_local_end..].iter().collect();
            segments.push((post, run.style, false));
        }
    }

    let has_selected = segments.iter().any(|(_, _, sel)| *sel);
    if !has_selected {
        return false;
    }

    let all_set = segments
        .iter()
        .filter(|(_, _, sel)| *sel)
        .all(|(_, style, _)| match attr {
            InlineFormatAttr::Bold => style.bold,
            InlineFormatAttr::Italic => style.italic,
            InlineFormatAttr::Underline => style.underline,
        });
    let target = !all_set;

    let mut new_runs: Vec<TextRun> = Vec::new();
    for (text, mut style, sel) in segments {
        if sel {
            match attr {
                InlineFormatAttr::Bold => style.bold = target,
                InlineFormatAttr::Italic => style.italic = target,
                InlineFormatAttr::Underline => style.underline = target,
            }
        }
        push_merged_run(&mut new_runs, &text, style);
    }

    if old_runs == new_runs {
        return false;
    }

    undo.break_coalescing();
    undo.execute(
        doc,
        EditCommand::UpdateRuns {
            index,
            old_runs,
            new_runs,
        },
    )
    .is_ok()
}

/// Toggles an inline style attribute across all runs of the element at `index`.
pub fn toggle_element_inline_style(
    doc: &mut Document,
    undo: &mut UndoManager,
    index: usize,
    attr: InlineFormatAttr,
) -> bool {
    let Some(el) = doc.elements.get(index) else {
        return false;
    };
    if el.runs.is_empty() {
        return false;
    }

    let all_set = el.runs.iter().all(|r| match attr {
        InlineFormatAttr::Bold => r.style.bold,
        InlineFormatAttr::Italic => r.style.italic,
        InlineFormatAttr::Underline => r.style.underline,
    });
    let target = !all_set;

    let old_runs = el.runs.clone();
    let new_runs: Vec<TextRun> = old_runs
        .iter()
        .map(|r| {
            let mut style = r.style;
            match attr {
                InlineFormatAttr::Bold => style.bold = target,
                InlineFormatAttr::Italic => style.italic = target,
                InlineFormatAttr::Underline => style.underline = target,
            }
            TextRun {
                text: r.text.clone(),
                style,
            }
        })
        .collect();

    if old_runs == new_runs {
        return false;
    }

    undo.break_coalescing();
    undo.execute(
        doc,
        EditCommand::UpdateRuns {
            index,
            old_runs,
            new_runs,
        },
    )
    .is_ok()
}

/// Computes a virtualized render window `(start, end)` around `focused_index` for large scripts.
#[must_use]
pub fn compute_element_window(total: usize, focused_index: usize) -> (usize, usize) {
    const VIRTUALIZE_THRESHOLD: usize = 120;
    const WINDOW_RADIUS: usize = 60;
    if total <= VIRTUALIZE_THRESHOLD {
        return (0, total);
    }
    let clamped_focus = focused_index.min(total.saturating_sub(1));
    let start = clamped_focus.saturating_sub(WINDOW_RADIUS);
    let end = (clamped_focus + WINDOW_RADIUS + 1).min(total);
    (start, end)
}

/// Updates `old_runs` with `final_text` while preserving existing [`InlineStyle`]s
/// on single-run elements and untouched prefix/suffix runs on multi-run elements.
#[must_use]
pub fn update_runs_preserving_styles(old_runs: &[TextRun], final_text: &str) -> Vec<TextRun> {
    if final_text.is_empty() {
        return Vec::new();
    }
    if old_runs.is_empty() {
        return vec![TextRun::plain(final_text)];
    }
    if old_runs.len() == 1 {
        return vec![TextRun {
            text: final_text.to_string(),
            style: old_runs[0].style,
        }];
    }

    let old_plain: String = old_runs.iter().map(|r| r.text.as_str()).collect();
    if old_plain == final_text {
        return old_runs.to_vec();
    }

    let prefix_len = common_char_prefix_bytes(&old_plain, final_text);
    let suffix_len = common_char_suffix_bytes(&old_plain[prefix_len..], &final_text[prefix_len..]);
    let old_suffix_start = old_plain.len() - suffix_len;
    let new_suffix_start = final_text.len() - suffix_len;

    let mut result: Vec<TextRun> = Vec::new();
    let mut cursor = 0usize;
    let mut mid_style = old_runs.last().map_or(InlineStyle::PLAIN, |r| r.style);

    for run in old_runs {
        let run_start = cursor;
        let run_end = cursor + run.text.len();
        cursor = run_end;

        if prefix_len >= run_start && prefix_len <= run_end {
            mid_style = run.style;
        }

        let p_end = prefix_len.min(run_end);
        if p_end > run_start {
            push_merged_run(&mut result, &old_plain[run_start..p_end], run.style);
        }
    }

    if new_suffix_start > prefix_len {
        push_merged_run(
            &mut result,
            &final_text[prefix_len..new_suffix_start],
            mid_style,
        );
    }

    cursor = 0;
    for run in old_runs {
        let run_start = cursor;
        let run_end = cursor + run.text.len();
        cursor = run_end;

        let s_start = old_suffix_start.max(run_start);
        if run_end > s_start {
            push_merged_run(&mut result, &old_plain[s_start..run_end], run.style);
        }
    }

    result
}

fn push_merged_run(out: &mut Vec<TextRun>, slice: &str, style: InlineStyle) {
    if slice.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.style == style {
            last.text.push_str(slice);
            return;
        }
    }
    out.push(TextRun {
        text: slice.to_string(),
        style,
    });
}

fn common_char_prefix_bytes(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .take_while(|((_, ca), cb)| ca == cb)
        .map(|((idx, ca), _)| idx + ca.len_utf8())
        .last()
        .unwrap_or(0)
}

fn common_char_suffix_bytes(a: &str, b: &str) -> usize {
    a.chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(ca, _)| ca.len_utf8())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_single_and_multi_run_styles_on_edit() {
        let bold = InlineStyle {
            bold: true,
            ..InlineStyle::PLAIN
        };
        let italic = InlineStyle {
            italic: true,
            ..InlineStyle::PLAIN
        };
        let old = vec![
            TextRun {
                text: "Merhaba ".to_string(),
                style: bold,
            },
            TextRun {
                text: "dünya".to_string(),
                style: italic,
            },
        ];

        let updated = update_runs_preserving_styles(&old, "Merhaba güzel dünya");
        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].text, "Merhaba ");
        assert_eq!(updated[0].style, bold);
        assert_eq!(updated[1].text, "güzel dünya");
        assert_eq!(updated[1].style, italic);
    }

    #[test]
    fn toggle_inline_style_uses_undo_and_toggles_bold_italic_underline() {
        let mut doc = Document::new(dram_core::ScriptLanguage::Tr);
        let mut undo = UndoManager::new();
        doc.elements[0] = Element::new(ElementType::Action, "Şehir uyuyor.");

        assert!(toggle_element_inline_style(
            &mut doc,
            &mut undo,
            0,
            InlineFormatAttr::Bold
        ));
        assert!(doc.elements[0].runs[0].style.bold);

        assert!(toggle_element_inline_style(
            &mut doc,
            &mut undo,
            0,
            InlineFormatAttr::Italic
        ));
        assert!(doc.elements[0].runs[0].style.bold);
        assert!(doc.elements[0].runs[0].style.italic);

        assert!(toggle_element_inline_style(
            &mut doc,
            &mut undo,
            0,
            InlineFormatAttr::Underline
        ));
        assert!(doc.elements[0].runs[0].style.underline);

        assert!(toggle_element_inline_style(
            &mut doc,
            &mut undo,
            0,
            InlineFormatAttr::Bold
        ));
        assert!(!doc.elements[0].runs[0].style.bold);
    }

    #[test]
    fn element_window_virtualizes_above_threshold() {
        assert_eq!(compute_element_window(80, 10), (0, 80));
        assert_eq!(compute_element_window(500, 200), (140, 261));
        assert_eq!(compute_element_window(500, 0), (0, 61));
    }

    #[test]
    fn toggle_inline_style_range_styles_only_selected_turkish_slice() {
        let mut doc = Document::new(dram_core::ScriptLanguage::Tr);
        let mut undo = UndoManager::new();
        doc.elements[0] = Element::new(ElementType::Action, "Şehir ışıklarla parlar.");

        // Select "ışıklarla" (char indices 6..15)
        assert!(toggle_element_inline_style_range(
            &mut doc,
            &mut undo,
            0,
            InlineFormatAttr::Bold,
            Some(6..15),
        ));
        let runs = &doc.elements[0].runs;
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "Şehir ");
        assert!(!runs[0].style.bold);
        assert_eq!(runs[1].text, "ışıklarla");
        assert!(runs[1].style.bold);
        assert_eq!(runs[2].text, " parlar.");
        assert!(!runs[2].style.bold);

        // Toggle off same range -> merges back into a single plain run
        assert!(toggle_element_inline_style_range(
            &mut doc,
            &mut undo,
            0,
            InlineFormatAttr::Bold,
            Some(6..15),
        ));
        assert_eq!(doc.elements[0].runs.len(), 1);
        assert!(!doc.elements[0].runs[0].style.bold);
    }
}
