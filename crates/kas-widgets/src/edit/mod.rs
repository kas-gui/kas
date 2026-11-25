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
use kas::data_transfer::DataTransferId;
pub use multi_part::MultiPartEditor;

use kas::cast::Cast;
use kas::event::{FocusSource, PhysicalKey};
use std::fmt::Debug;
use std::ops::Range;

/// Describes the change source of a history (undo) state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditOp {
    /// Initial state
    ///
    /// This is a distinct "operation" to ensure that the initial state gets
    /// saved on the first edit.
    Initial,
    /// Cursor movement or selection adjustment
    ///
    /// This is a special "operation": it does not by itself cause a new state
    /// to be saved, but does ensure that other operations such as `KeyDelete`
    /// cannot be merged when separated by a `Cursor` operation.
    Cursor,
    /// Keyboard input
    ///
    /// This is mergeable with itself.
    KeyInput,
    /// Input Method Editor
    ///
    /// This is mergeable with itself.
    Ime,
    /// Deletion due to key press
    ///
    /// This is mergeable with itself.
    KeyDelete,
    /// Replacement of a range, e.g. via the clipboard.
    ///
    /// This is never mergeable.
    Replace,
}

impl EditOp {
    /// Whether the operation is mergeable with itself
    fn is_mergeable(self) -> bool {
        matches!(self, EditOp::KeyInput | EditOp::Ime | EditOp::KeyDelete)
    }
}

/// Used to track ongoing incompatible actions
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum CurrentAction {
    /// No current action
    #[default]
    None,
    /// A paste was requested; awaiting data transfer
    Paste {
        id: DataTransferId,
        source: FocusSource,
    },
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
    /// Returns `Some(part)` when IME is enabled using the given `part`.
    ///
    /// This does not imply a pre-edit (or any IME input).
    #[inline]
    fn ime_part(&self) -> Option<u32> {
        match self {
            CurrentAction::None | CurrentAction::Paste { .. } | CurrentAction::Selection => None,
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

    /// True on `Selection`
    #[inline]
    fn is_selecting(&self) -> bool {
        matches!(self, CurrentAction::Selection)
    }
}
