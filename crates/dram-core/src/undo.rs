//! Unlimited undo/redo command stack with consecutive text-edit coalescing.

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
    /// Delete the element at `index`.
    DeleteElement {
        /// Index of the deleted element.
        index: usize,
        /// Removed element (captured on execution for undo).
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
    /// Change the [`ElementType`] of the element at `index`.
    ChangeElementType {
        /// Target element index.
        index: usize,
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
            Self::DeleteElement { index, .. } => {
                if *index >= doc.elements.len() {
                    return Err(CoreError::IndexOutOfBounds {
                        index: *index,
                        len: doc.elements.len(),
                    });
                }
                doc.elements.remove(*index);
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
                for cmd in cmds {
                    cmd.apply(doc)?;
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
            Self::DeleteElement { index, removed } => {
                if *index > doc.elements.len() {
                    return Err(CoreError::IndexOutOfBounds {
                        index: *index,
                        len: doc.elements.len(),
                    });
                }
                doc.elements.insert(*index, removed.clone());
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
                for cmd in cmds.iter().rev() {
                    cmd.revert(doc)?;
                }
            }
        }
        doc.touch();
        Ok(())
    }
}

/// Manages an unlimited undo/redo history with coalescing for consecutive typing edits.
#[derive(Debug, Default, Clone)]
pub struct UndoManager {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
    coalescing_open: bool,
}

impl UndoManager {
    /// Creates an empty [`UndoManager`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Executes `cmd` on `doc` and pushes it onto the undo stack, clearing the redo stack.
    ///
    /// Consecutive `UpdateRuns` commands on the same element index are coalesced while
    /// coalescing is open (broken by [`Self::break_coalescing`] or non-`UpdateRuns` commands).
    pub fn execute(&mut self, doc: &mut Document, cmd: EditCommand) -> Result<(), CoreError> {
        cmd.apply(doc)?;
        self.redo_stack.clear();

        if self.coalescing_open {
            if let (
                Some(EditCommand::UpdateRuns {
                    index: prev_idx,
                    new_runs: prev_new,
                    ..
                }),
                EditCommand::UpdateRuns {
                    index: cur_idx,
                    new_runs: cur_new,
                    ..
                },
            ) = (self.undo_stack.last_mut(), &cmd)
            {
                if prev_idx == cur_idx {
                    *prev_new = cur_new.clone();
                    return Ok(());
                }
            }
        }

        self.coalescing_open = matches!(cmd, EditCommand::UpdateRuns { .. });
        self.undo_stack.push(cmd);
        Ok(())
    }

    /// Prevents the next `UpdateRuns` command from merging into the top of the undo stack.
    pub fn break_coalescing(&mut self) {
        self.coalescing_open = false;
    }

    /// Undoes the most recent command on `doc`.
    pub fn undo(&mut self, doc: &mut Document) -> Result<(), CoreError> {
        let cmd = self
            .undo_stack
            .pop()
            .ok_or(CoreError::EmptyHistory("undo"))?;
        cmd.revert(doc)?;
        self.redo_stack.push(cmd);
        self.coalescing_open = false;
        Ok(())
    }

    /// Redoes the most recently undone command on `doc`.
    pub fn redo(&mut self, doc: &mut Document) -> Result<(), CoreError> {
        let cmd = self
            .redo_stack
            .pop()
            .ok_or(CoreError::EmptyHistory("redo"))?;
        cmd.apply(doc)?;
        self.undo_stack.push(cmd);
        self.coalescing_open = false;
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
}
