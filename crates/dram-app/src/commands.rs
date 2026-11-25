//! Keyboard shortcuts, find/replace, scene reordering, and dual-dialogue commands.

use dram_core::text::contains_case_insensitive;
use dram_core::{DualDialogueGroup, DualSide, EditCommand, ElementType, TextRun};
use eframe::egui;
use uuid::Uuid;

use crate::app::DramApp;

impl DramApp {
    pub(crate) fn find_match_in(&self, text: &str) -> Option<(usize, usize)> {
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

    pub(crate) fn find_next(&mut self) {
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

    pub(crate) fn replace_current(&mut self) {
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

    pub(crate) fn replace_all(&mut self) {
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

    pub(crate) fn swap_scenes(
        &mut self,
        scenes: &[(usize, usize, String)],
        from_s: usize,
        to_s: usize,
    ) {
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

    pub(crate) fn toggle_dual_dialogue(&mut self) {
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

    pub(crate) fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
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
}
