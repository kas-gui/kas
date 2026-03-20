// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditBoxCore`] and [`EditBox`] widgets, plus supporting items

mod edit_box;
mod edit_field;
pub mod editor;
mod guard;
pub mod highlight;
mod multi_part;

pub use edit_box::EditBox;
pub use edit_field::EditBoxCore;
pub use editor::{Common, Editor, Part};
pub use guard::*;
pub use multi_part::MultiPartEditor;

use kas::cast::Cast;
use kas::event::PhysicalKey;
use std::fmt::Debug;
use std::ops::Range;

/// Describes the change source of a history (undo) state
///
/// Many variants include the `part` or part `range` affected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditOp {
    /// Initial state
    Initial,
    /// Cursor movement or selection adjustment
    Cursor,
    /// Keyboard
    KeyInput(usize, usize),
    /// Input Method Editor
    Ime(usize),
    /// Deletion due to key press
    KeyDelete(usize, usize),
    /// Replacement of a range, e.g. via the clipboard. Does not merge.
    Replace(usize, usize),
}

impl EditOp {
    /// An edit may be merged with a previous one if both are equal and this method returns `true`
    fn try_merge(self, last_op: &mut Option<Self>) -> bool {
        match (self, last_op) {
            (EditOp::Cursor, Some(last)) => {
                *last = self;
                true
            }
            (EditOp::KeyInput(_, _) | EditOp::KeyDelete(_, _), Some(last)) if self == *last => true,
            _ => false,
        }
    }
}

/// Used to track ongoing incompatible actions
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum CurrentAction {
    /// No current action
    #[default]
    None,
    /// IME is enabled but no input has yet been given. This is special in that
    /// a selection may exist (which would get replaced by the pre-edit text).
    ImeStart(u32),
    /// We have some pre-edit text within the given range (if non-empty).
    ///
    /// This text should be deleted if IME is cancelled.
    ImePreedit {
        part: u32,
        /// Range of the pre-edit text
        edit_range: Range<u32>,
    },
    Selection,
}

impl CurrentAction {
    #[inline]
    fn is_none(&self) -> bool {
        *self == CurrentAction::None
    }

    /// Returns `Some(part)` when IME is enabled using the given `part`.
    ///
    /// This does not imply a pre-edit (or any IME input).
    #[inline]
    fn ime_part(&self) -> Option<usize> {
        match self {
            CurrentAction::None | CurrentAction::Selection => None,
            CurrentAction::ImeStart(part) | CurrentAction::ImePreedit { part, .. } => {
                Some((*part).cast())
            }
        }
    }

    /// Check whether IME is enabled
    ///
    /// This does not imply a pre-edit (or any IME input).
    #[inline]
    fn is_ime_enabled(&self) -> bool {
        self.ime_part().is_some()
    }
}
