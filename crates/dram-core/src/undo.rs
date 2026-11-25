//! Bounded undo/redo command stack with UUID-addressable commands, checkpoints, and coalescing.

use std::collections::VecDeque;

use uuid::Uuid;

use crate::error::CoreError;
use crate::model::{Document, Element, ElementType, TextRun};

/// Reversible editing command operating on a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditCommand {
    /// Insert an element at `index`.
    InsertElement {
        /// Target insertion index.
        index: usize,
        /// Element to insert.
        element: Element,
    },
    /// Insert an element immediately after `after_id` (or at index 0 if `None`).
    InsertElementAfterId {
        /// Anchor element UUID (`None` inserts at the start of the document).
        after_id: Option<Uuid>,
        /// Element to insert.
        element: Element,
    },
    /// Delete the element at `index`.
    DeleteElement {
        /// Index of the deleted element.
        index: usize,
        /// Removed element (captured on execution for undo).
        removed: Element,
    },
    /// Delete the element identified by `id`.
    DeleteElementById {
        /// Target element UUID.
        id: Uuid,
        /// Original index (recorded for exact restoration on revert).
        removed_index: usize,
        /// Removed element snapshot.
        removed: Element,
    },
    /// Replace the text runs of the element at `index`.
    UpdateRuns {
        /// Target element index.
        index: usize,
        /// Previous runs before the edit.
        old_runs: Vec<TextRun>,
        /// New runs after the edit.
        new_runs: Vec<TextRun>,
    },
    /// Replace the text runs of the element identified by `id`.
    UpdateRunsById {
        /// Target element UUID.
        id: Uuid,
        /// Previous runs before the edit.
        old_runs: Vec<TextRun>,
        /// New runs after the edit.
        new_runs: Vec<TextRun>,
    },
    /// Change the [`ElementType`] of the element at `index`.
    ChangeElementType {
        /// Target element index.
        index: usize,
        /// Previous element type.
        old_kind: ElementType,
        /// New element type.
        new_kind: ElementType,
    },
    /// Change the [`ElementType`] of the element identified by `id`.
    ChangeElementTypeById {
        /// Target element UUID.
        id: Uuid,
        /// Previous element type.
        old_kind: ElementType,
        /// New element type.
        new_kind: ElementType,
    },
    /// Move an element from `from` to `to`.
    MoveElement {
        /// Original index.
        from: usize,
        /// Destination index.
        to: usize,
    },
    /// Group of commands executed and undone atomically as one step.
    Batch(Vec<EditCommand>),
}

impl EditCommand {
    /// Returns an approximate heap+stack memory footprint for undo budget accounting.
    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        let base = std::mem::size_of::<Self>();
        match self {
            Self::InsertElement { element, .. } | Self::InsertElementAfterId { element, .. } => {
                base + estimate_element_bytes(element)
            }
            Self::DeleteElement { removed, .. } | Self::DeleteElementById { removed, .. } => {
                base + estimate_element_bytes(removed)
            }
            Self::UpdateRuns {
                old_runs, new_runs, ..
            }
            | Self::UpdateRunsById {
                old_runs, new_runs, ..
            } => base + estimate_runs_bytes(old_runs) + estimate_runs_bytes(new_runs),
            Self::ChangeElementType { .. }
            | Self::ChangeElementTypeById { .. }
            | Self::MoveElement { .. } => base,
            Self::Batch(cmds) => base + cmds.iter().map(Self::estimated_bytes).sum::<usize>(),
        }
    }

    /// Applies the command forward onto `doc`.
    pub fn apply(&self, doc: &mut Document) -> Result<(), CoreError> {
        match self {
            Self::InsertElement { index, element } => {
                if *index > doc.elements.len() {
                    return Err(CoreError::IndexOutOfBounds {
                        index: *index,
                        len: doc.elements.len(),
                    });
                }
                doc.elements.insert(*index, element.clone());
            }
            Self::InsertElementAfterId { after_id, element } => {
                let insert_idx = match after_id {
                    None => 0,
                    Some(aid) => {
                        let pos = find_element_index(doc, *aid)?;
                        pos + 1
                    }
                };
                doc.elements.insert(insert_idx, element.clone());
            }
            Self::DeleteElement { index, .. } => {
                if *index >= doc.elements.len() {
                    return Err(CoreError::IndexOutOfBounds {
                        index: *index,
                        len: doc.elements.len(),
                    });
                }
                doc.elements.remove(*index);
            }
            Self::DeleteElementById { id, .. } => {
                let idx = find_element_index(doc, *id)?;
                doc.elements.remove(idx);
            }
            Self::UpdateRuns {
                index, new_runs, ..
            } => {
                let len = doc.elements.len();
                let el = doc
                    .elements
                    .get_mut(*index)
                    .ok_or(CoreError::IndexOutOfBounds { index: *index, len })?;
                el.runs = new_runs.clone();
            }
            Self::UpdateRunsById { id, new_runs, .. } => {
                let idx = find_element_index(doc, *id)?;
                doc.elements[idx].runs = new_runs.clone();
            }
            Self::ChangeElementType {
                index, new_kind, ..
            } => {
                let len = doc.elements.len();
                let el = doc
                    .elements
                    .get_mut(*index)
                    .ok_or(CoreError::IndexOutOfBounds { index: *index, len })?;
                el.kind = *new_kind;
            }
            Self::ChangeElementTypeById { id, new_kind, .. } => {
                let idx = find_element_index(doc, *id)?;
                doc.elements[idx].kind = *new_kind;
            }
            Self::MoveElement { from, to } => {
                let len = doc.elements.len();
                if *from >= len {
                    return Err(CoreError::IndexOutOfBounds { index: *from, len });
                }
                if *to >= len {
                    return Err(CoreError::IndexOutOfBounds { index: *to, len });
                }
                let el = doc.elements.remove(*from);
                doc.elements.insert(*to, el);
            }
            Self::Batch(cmds) => {
                for (applied, cmd) in cmds.iter().enumerate() {
                    if let Err(err) = cmd.apply(doc) {
                        for rollback_cmd in cmds[..applied].iter().rev() {
                            let _ = rollback_cmd.revert(doc);
                        }
                        return Err(err);
                    }
                }
            }
        }
        doc.touch();
        Ok(())
    }

    /// Reverts the command on `doc`.
    pub fn revert(&self, doc: &mut Document) -> Result<(), CoreError> {
        match self {
            Self::InsertElement { index, .. } => {
                if *index >= doc.elements.len() {
                    return Err(CoreError::IndexOutOfBounds {
                        index: *index,
                        len: doc.elements.len(),
                    });
                }
                doc.elements.remove(*index);
            }
            Self::InsertElementAfterId { element, .. } => {
                let idx = find_element_index(doc, element.id)?;
                doc.elements.remove(idx);
            }
            Self::DeleteElement { index, removed } => {
                if *index > doc.elements.len() {
                    return Err(CoreError::IndexOutOfBounds {
                        index: *index,
                        len: doc.elements.len(),
                    });
                }
                doc.elements.insert(*index, removed.clone());
            }
            Self::DeleteElementById {
                removed_index,
                removed,
                ..
            } => {
                let idx = (*removed_index).min(doc.elements.len());
                doc.elements.insert(idx, removed.clone());
            }
            Self::UpdateRuns {
                index, old_runs, ..
            } => {
                let len = doc.elements.len();
                let el = doc
                    .elements
                    .get_mut(*index)
                    .ok_or(CoreError::IndexOutOfBounds { index: *index, len })?;
                el.runs = old_runs.clone();
            }
            Self::UpdateRunsById { id, old_runs, .. } => {
                let idx = find_element_index(doc, *id)?;
                doc.elements[idx].runs = old_runs.clone();
            }
            Self::ChangeElementType {
                index, old_kind, ..
            } => {
                let len = doc.elements.len();
                let el = doc
                    .elements
                    .get_mut(*index)
                    .ok_or(CoreError::IndexOutOfBounds { index: *index, len })?;
                el.kind = *old_kind;
            }
            Self::ChangeElementTypeById { id, old_kind, .. } => {
                let idx = find_element_index(doc, *id)?;
                doc.elements[idx].kind = *old_kind;
            }
            Self::MoveElement { from, to } => {
                let len = doc.elements.len();
                if *to >= len {
                    return Err(CoreError::IndexOutOfBounds { index: *to, len });
                }
                if *from >= len {
                    return Err(CoreError::IndexOutOfBounds { index: *from, len });
                }
                let el = doc.elements.remove(*to);
                doc.elements.insert(*from, el);
            }
            Self::Batch(cmds) => {
                for (reverted, cmd) in cmds.iter().rev().enumerate() {
                    if let Err(err) = cmd.revert(doc) {
                        let start = cmds.len() - reverted;
                        for rollback_cmd in &cmds[start..] {
                            let _ = rollback_cmd.apply(doc);
                        }
                        return Err(err);
                    }
                }
            }
        }
        doc.touch();
        Ok(())
    }
}

fn find_element_index(doc: &Document, id: Uuid) -> Result<usize, CoreError> {
    doc.elements
        .iter()
        .position(|el| el.id == id)
        .ok_or(CoreError::ElementNotFound(id))
}

fn estimate_runs_bytes(runs: &[TextRun]) -> usize {
    runs.iter()
        .map(|r| std::mem::size_of::<TextRun>() + r.text.capacity())
        .sum()
}

fn estimate_element_bytes(el: &Element) -> usize {
    std::mem::size_of::<Element>() + estimate_runs_bytes(&el.runs)
}

/// Configurable history limits for [`UndoManager`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoLimits {
    /// Maximum number of undo steps retained.
    pub max_commands: usize,
    /// Maximum estimated memory footprint in bytes retained on the undo stack.
    pub max_bytes: usize,
}

impl Default for UndoLimits {
    fn default() -> Self {
        Self {
            max_commands: 1000,
            max_bytes: 32 * 1024 * 1024,
        }
    }
}

/// Manages a bounded undo/redo history with checkpoints and consecutive typing coalescing.
#[derive(Debug, Clone)]
pub struct UndoManager {
    undo_stack: VecDeque<EditCommand>,
    redo_stack: Vec<EditCommand>,
    coalescing_open: bool,
    limits: UndoLimits,
    undo_bytes: usize,
    step_counter: isize,
    checkpoint_step: Option<isize>,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::with_limits(UndoLimits::default())
    }
}

impl UndoManager {
    /// Creates an empty [`UndoManager`] with default limits.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty [`UndoManager`] enforcing custom [`UndoLimits`].
    #[must_use]
    pub fn with_limits(limits: UndoLimits) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            coalescing_open: false,
            limits,
            undo_bytes: 0,
            step_counter: 0,
            checkpoint_step: None,
        }
    }

    /// Updates [`UndoLimits`] and immediately trims excess older undo entries.
    pub fn set_limits(&mut self, limits: UndoLimits) {
        self.limits = limits;
        self.enforce_limits();
    }

    /// Returns the number of steps currently on the undo stack.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the estimated memory footprint (in bytes) of retained undo steps.
    #[must_use]
    pub fn estimated_undo_bytes(&self) -> usize {
        self.undo_bytes
    }

    /// Marks the current history position as a save/clean checkpoint.
    pub fn mark_checkpoint(&mut self) {
        self.checkpoint_step = Some(self.step_counter);
        self.coalescing_open = false;
    }

    /// Returns `true` if the current history position matches the active checkpoint.
    #[must_use]
    pub fn is_at_checkpoint(&self) -> bool {
        self.checkpoint_step == Some(self.step_counter)
    }

    /// Executes `cmd` on `doc` and pushes it onto the undo stack, clearing the redo stack.
    ///
    /// Consecutive `UpdateRuns` / `UpdateRunsById` commands on the same target are coalesced
    /// while coalescing is open (broken by [`Self::break_coalescing`] or other commands).
    pub fn execute(&mut self, doc: &mut Document, cmd: EditCommand) -> Result<(), CoreError> {
        cmd.apply(doc)?;
        self.redo_stack.clear();
        if self
            .checkpoint_step
            .is_some_and(|cp| cp > self.step_counter)
        {
            self.checkpoint_step = None;
        }

        if self.coalescing_open {
            match (self.undo_stack.back_mut(), &cmd) {
                (
                    Some(EditCommand::UpdateRuns {
                        index: prev_idx,
                        old_runs,
                        new_runs: prev_new,
                    }),
                    EditCommand::UpdateRuns {
                        index: cur_idx,
                        new_runs: cur_new,
                        ..
                    },
                ) if prev_idx == cur_idx => {
                    let old_bytes = estimate_runs_bytes(old_runs) + estimate_runs_bytes(prev_new);
                    let new_bytes = estimate_runs_bytes(old_runs) + estimate_runs_bytes(cur_new);
                    self.undo_bytes = self.undo_bytes.saturating_sub(old_bytes) + new_bytes;
                    *prev_new = cur_new.clone();
                    self.enforce_limits();
                    return Ok(());
                }
                (
                    Some(EditCommand::UpdateRunsById {
                        id: prev_id,
                        old_runs,
                        new_runs: prev_new,
                    }),
                    EditCommand::UpdateRunsById {
                        id: cur_id,
                        new_runs: cur_new,
                        ..
                    },
                ) if prev_id == cur_id => {
                    let old_bytes = estimate_runs_bytes(old_runs) + estimate_runs_bytes(prev_new);
                    let new_bytes = estimate_runs_bytes(old_runs) + estimate_runs_bytes(cur_new);
                    self.undo_bytes = self.undo_bytes.saturating_sub(old_bytes) + new_bytes;
                    *prev_new = cur_new.clone();
                    self.enforce_limits();
                    return Ok(());
                }
                _ => {}
            }
        }

        self.coalescing_open = matches!(
            cmd,
            EditCommand::UpdateRuns { .. } | EditCommand::UpdateRunsById { .. }
        );
        self.undo_bytes = self.undo_bytes.saturating_add(cmd.estimated_bytes());
        self.undo_stack.push_back(cmd);
        self.step_counter += 1;
        self.enforce_limits();
        Ok(())
    }

    fn enforce_limits(&mut self) {
        while self.undo_stack.len() > self.limits.max_commands
            || (self.undo_bytes > self.limits.max_bytes && self.undo_stack.len() > 1)
        {
            if let Some(removed) = self.undo_stack.pop_front() {
                self.undo_bytes = self.undo_bytes.saturating_sub(removed.estimated_bytes());
            } else {
                break;
            }
        }
    }

    /// Prevents the next `UpdateRuns` command from merging into the top of the undo stack.
    pub fn break_coalescing(&mut self) {
        self.coalescing_open = false;
    }

    /// Undoes the most recent command on `doc`.
    pub fn undo(&mut self, doc: &mut Document) -> Result<(), CoreError> {
        let cmd = self
            .undo_stack
            .pop_back()
            .ok_or(CoreError::EmptyHistory("undo"))?;
        if let Err(err) = cmd.revert(doc) {
            self.undo_stack.push_back(cmd);
            return Err(err);
        }
        self.undo_bytes = self.undo_bytes.saturating_sub(cmd.estimated_bytes());
        self.redo_stack.push(cmd);
        self.coalescing_open = false;
        self.step_counter -= 1;
        Ok(())
    }

    /// Redoes the most recently undone command on `doc`.
    pub fn redo(&mut self, doc: &mut Document) -> Result<(), CoreError> {
        let cmd = self
            .redo_stack
            .pop()
            .ok_or(CoreError::EmptyHistory("redo"))?;
        if let Err(err) = cmd.apply(doc) {
            self.redo_stack.push(cmd);
            return Err(err);
        }
        self.undo_bytes = self.undo_bytes.saturating_add(cmd.estimated_bytes());
        self.undo_stack.push_back(cmd);
        self.coalescing_open = false;
        self.step_counter += 1;
        self.enforce_limits();
        Ok(())
    }

    /// Returns `true` if at least one command can be undone.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if at least one command can be redone.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.coalescing_open = false;
        self.undo_bytes = 0;
        self.step_counter = 0;
        self.checkpoint_step = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ScriptLanguage;

    #[test]
    fn undo_redo_and_coalescing_work() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let mut mgr = UndoManager::new();

        // Coalesced typing on element 0
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRuns {
                index: 0,
                old_runs: vec![],
                new_runs: vec![TextRun::plain("İÇ.")],
            },
        )
        .unwrap();
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRuns {
                index: 0,
                old_runs: vec![TextRun::plain("İÇ.")],
                new_runs: vec![TextRun::plain("İÇ. KAFE - GÜN")],
            },
        )
        .unwrap();

        assert_eq!(doc.elements[0].plain_text(), "İÇ. KAFE - GÜN");

        // Single undo reverts both coalesced updates
        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[0].plain_text(), "");
        assert!(!mgr.can_undo());
        assert!(mgr.can_redo());

        mgr.redo(&mut doc).unwrap();
        assert_eq!(doc.elements[0].plain_text(), "İÇ. KAFE - GÜN");

        // Insert and change type
        mgr.execute(
            &mut doc,
            EditCommand::InsertElement {
                index: 1,
                element: Element::new(ElementType::Action, "Ahmet içeri girer."),
            },
        )
        .unwrap();
        assert_eq!(doc.elements.len(), 2);

        mgr.execute(
            &mut doc,
            EditCommand::ChangeElementType {
                index: 1,
                old_kind: ElementType::Action,
                new_kind: ElementType::Character,
            },
        )
        .unwrap();
        assert_eq!(doc.elements[1].kind, ElementType::Character);

        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[1].kind, ElementType::Action);
        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements.len(), 1);
    }

    #[test]
    fn batch_apply_rolls_back_on_partial_failure() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let original_elements = doc.elements.clone();

        let batch = EditCommand::Batch(vec![
            EditCommand::InsertElement {
                index: 1,
                element: Element::new(ElementType::Action, "Geçici satır"),
            },
            EditCommand::ChangeElementType {
                index: 1,
                old_kind: ElementType::Action,
                new_kind: ElementType::Dialogue,
            },
            // Out-of-bounds index triggers error mid-batch.
            EditCommand::ChangeElementType {
                index: 99,
                old_kind: ElementType::Action,
                new_kind: ElementType::Character,
            },
        ]);

        let err = batch.apply(&mut doc).unwrap_err();
        assert!(matches!(err, CoreError::IndexOutOfBounds { index: 99, .. }));
        assert_eq!(doc.elements, original_elements);
    }

    #[test]
    fn batch_revert_rolls_back_on_partial_failure() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements
            .push(Element::new(ElementType::Action, "İkinci satır"));
        let snapshot_before_revert = doc.elements.clone();

        let batch = EditCommand::Batch(vec![
            // Fails on revert (index 99 out of bounds) after the second command already reverted.
            EditCommand::DeleteElement {
                index: 99,
                removed: Element::new(ElementType::Note, "Yok"),
            },
            EditCommand::ChangeElementType {
                index: 1,
                old_kind: ElementType::Dialogue,
                new_kind: ElementType::Action,
            },
        ]);

        let err = batch.revert(&mut doc).unwrap_err();
        assert!(matches!(err, CoreError::IndexOutOfBounds { index: 99, .. }));
        assert_eq!(doc.elements, snapshot_before_revert);
    }

    #[test]
    fn deep_undo_redo_chain_restores_exact_document_state() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let initial = doc.elements.clone();
        let mut mgr = UndoManager::new();

        for i in 1..=60 {
            mgr.execute(
                &mut doc,
                EditCommand::InsertElement {
                    index: i,
                    element: Element::new(ElementType::Action, format!("Satır {i}")),
                },
            )
            .unwrap();
        }
        assert_eq!(doc.elements.len(), 61);
        let final_snapshot = doc.elements.clone();

        while mgr.can_undo() {
            mgr.undo(&mut doc).unwrap();
        }
        assert_eq!(doc.elements, initial);

        while mgr.can_redo() {
            mgr.redo(&mut doc).unwrap();
        }
        assert_eq!(doc.elements, final_snapshot);
    }

    #[test]
    fn empty_history_returns_error_without_panicking() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let mut mgr = UndoManager::new();
        assert!(matches!(
            mgr.undo(&mut doc),
            Err(CoreError::EmptyHistory("undo"))
        ));
        assert!(matches!(
            mgr.redo(&mut doc),
            Err(CoreError::EmptyHistory("redo"))
        ));
    }

    #[test]
    fn failed_execute_does_not_corrupt_undo_or_redo_stacks() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let mut mgr = UndoManager::new();

        mgr.execute(
            &mut doc,
            EditCommand::InsertElement {
                index: 1,
                element: Element::new(ElementType::Action, "İlk eklenen"),
            },
        )
        .unwrap();
        mgr.undo(&mut doc).unwrap();
        assert!(mgr.can_redo());
        assert!(!mgr.can_undo());

        let err = mgr
            .execute(
                &mut doc,
                EditCommand::ChangeElementType {
                    index: 42,
                    old_kind: ElementType::Action,
                    new_kind: ElementType::Dialogue,
                },
            )
            .unwrap_err();
        assert!(matches!(err, CoreError::IndexOutOfBounds { index: 42, .. }));
        assert!(mgr.can_redo());
        assert!(!mgr.can_undo());
    }

    #[test]
    fn coalescing_breaks_across_elements_and_on_break_coalescing() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements
            .push(Element::new(ElementType::Dialogue, "İkinci"));
        let mut mgr = UndoManager::new();

        mgr.execute(
            &mut doc,
            EditCommand::UpdateRuns {
                index: 0,
                old_runs: vec![TextRun::plain("")],
                new_runs: vec![TextRun::plain("A")],
            },
        )
        .unwrap();
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRuns {
                index: 0,
                old_runs: vec![TextRun::plain("A")],
                new_runs: vec![TextRun::plain("AB")],
            },
        )
        .unwrap();
        mgr.break_coalescing();
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRuns {
                index: 0,
                old_runs: vec![TextRun::plain("AB")],
                new_runs: vec![TextRun::plain("ABC")],
            },
        )
        .unwrap();
        // Editing a different index does not coalesce with index 0.
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRuns {
                index: 1,
                old_runs: vec![TextRun::plain("İkinci")],
                new_runs: vec![TextRun::plain("Değişti")],
            },
        )
        .unwrap();

        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[1].plain_text(), "İkinci");
        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[0].plain_text(), "AB");
        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[0].plain_text(), "");
    }

    #[test]
    fn move_element_and_delete_round_trip_preserve_order() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.push(Element::new(ElementType::Action, "Bir"));
        doc.elements.push(Element::new(ElementType::Action, "İki"));
        let mut mgr = UndoManager::new();

        mgr.execute(&mut doc, EditCommand::MoveElement { from: 2, to: 1 })
            .unwrap();
        assert_eq!(doc.elements[1].plain_text(), "İki");
        assert_eq!(doc.elements[2].plain_text(), "Bir");

        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[1].plain_text(), "Bir");
        assert_eq!(doc.elements[2].plain_text(), "İki");
    }

    #[test]
    fn uuid_commands_survive_element_insertions_and_coalesce() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let first_id = doc.elements[0].id;
        let mut mgr = UndoManager::new();

        // Coalesce two UpdateRunsById edits on first_id
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRunsById {
                id: first_id,
                old_runs: vec![TextRun::plain("")],
                new_runs: vec![TextRun::plain("İÇ.")],
            },
        )
        .unwrap();
        mgr.execute(
            &mut doc,
            EditCommand::UpdateRunsById {
                id: first_id,
                old_runs: vec![TextRun::plain("İÇ.")],
                new_runs: vec![TextRun::plain("İÇ. ODA")],
            },
        )
        .unwrap();
        assert_eq!(mgr.undo_len(), 1);

        // Insert a new element before first_id (after_id: None) -> shifts first_id to index 1
        let inserted = Element::new(ElementType::Section(1), "BÖLÜM 1");
        let inserted_id = inserted.id;
        mgr.execute(
            &mut doc,
            EditCommand::InsertElementAfterId {
                after_id: None,
                element: inserted,
            },
        )
        .unwrap();
        assert_eq!(doc.elements[0].id, inserted_id);
        assert_eq!(doc.elements[1].id, first_id);

        // Change type by UUID still targets first_id even after index shift
        mgr.execute(
            &mut doc,
            EditCommand::ChangeElementTypeById {
                id: first_id,
                old_kind: ElementType::SceneHeading,
                new_kind: ElementType::Action,
            },
        )
        .unwrap();
        assert_eq!(doc.elements[1].kind, ElementType::Action);

        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[1].kind, ElementType::SceneHeading);
        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].id, first_id);
        mgr.undo(&mut doc).unwrap();
        assert_eq!(doc.elements[0].plain_text(), "");
    }

    #[test]
    fn undo_limits_trim_oldest_entries_by_count_and_bytes() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let limits = UndoLimits {
            max_commands: 3,
            max_bytes: 64 * 1024,
        };
        let mut mgr = UndoManager::with_limits(limits);

        for i in 1..=5 {
            mgr.execute(
                &mut doc,
                EditCommand::InsertElement {
                    index: i,
                    element: Element::new(ElementType::Action, format!("Satır {i}")),
                },
            )
            .unwrap();
        }
        assert_eq!(mgr.undo_len(), 3);
        assert!(mgr.estimated_undo_bytes() > 0);

        // Byte-budget trimming
        mgr.set_limits(UndoLimits {
            max_commands: 100,
            max_bytes: 1,
        });
        assert_eq!(mgr.undo_len(), 1);
    }

    #[test]
    fn checkpoint_tracks_save_state_and_invalidates_on_branch() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let mut mgr = UndoManager::new();
        mgr.mark_checkpoint();
        assert!(mgr.is_at_checkpoint());

        mgr.execute(
            &mut doc,
            EditCommand::InsertElement {
                index: 1,
                element: Element::new(ElementType::Action, "Yeni"),
            },
        )
        .unwrap();
        assert!(!mgr.is_at_checkpoint());

        mgr.undo(&mut doc).unwrap();
        assert!(mgr.is_at_checkpoint());

        mgr.redo(&mut doc).unwrap();
        mgr.mark_checkpoint();
        assert!(mgr.is_at_checkpoint());

        // Undo past checkpoint and branch with a new edit -> old checkpoint must invalidate
        mgr.undo(&mut doc).unwrap();
        assert!(!mgr.is_at_checkpoint());
        mgr.execute(
            &mut doc,
            EditCommand::InsertElement {
                index: 1,
                element: Element::new(ElementType::Action, "Alternatif"),
            },
        )
        .unwrap();
        assert!(!mgr.is_at_checkpoint());
    }
}
