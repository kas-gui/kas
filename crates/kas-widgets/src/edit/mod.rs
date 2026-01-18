// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

mod edit_box;
mod edit_field;
mod editor;
mod guard;

pub use edit_box::EditBox;
pub use edit_field::EditField;
pub use editor::Editor;
pub use guard::*;

use std::fmt::Debug;
use std::ops::Range;

/// Describes the change source of a history (undo) state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditOp {
    /// Initial state
    Initial,
    /// Cursor movement or selection adjustment
    Cursor,
    /// Keyboard
    KeyInput,
    /// Input Method Editor
    Ime,
    /// Deletion due to key press
    Delete,
    /// Cut to or paste from clipboard
    Clipboard,
    /// Programmatic edit
    Synthetic,
}

impl EditOp {
    /// An edit may be merged with a previous one if both are equal and this method returns `true`
    fn try_merge(self, last_op: &mut Option<Self>) -> bool {
        match (self, last_op) {
            (EditOp::Cursor, Some(last)) => {
                *last = self;
                true
            }
            (EditOp::KeyInput | EditOp::Delete, Some(last)) if self == *last => true,
            _ => false,
        }
    }
}

enum CmdAction {
    /// Key not used, no action
    Unused,
    /// Key used, no action
    Used,
    /// Cursor and/or selection changed
    Cursor,
    /// Enter key in single-line editor
    Activate,
    /// Text was edited by key command
    Edit,
}

/// Used to track ongoing incompatible actions
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum CurrentAction {
    /// No current action
    #[default]
    None,
    /// IME is enabled but no input has yet been given. This is special in that
    /// a selection may exist (which would get replaced by the pre-edit text).
    ImeStart,
    /// We have some pre-edit text within the given range (if non-empty).
    ///
    /// This text should be deleted if IME is cancelled.
    ImePreedit {
        /// Range of the pre-edit text
        edit_range: Range<u32>,
    },
    Selection,
}

impl CurrentAction {
    fn is_none(&self) -> bool {
        *self == CurrentAction::None
    }

    /// Check whether IME is enabled
    ///
    /// This does not imply a pre-edit (or any IME input).
    fn is_ime_enabled(&self) -> bool {
        matches!(
            self,
            CurrentAction::ImeStart | CurrentAction::ImePreedit { .. }
        )
    }
}
