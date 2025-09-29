// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

mod edit_box;
mod edit_field;
mod guard;

pub use edit_box::EditBox;
pub use edit_field::EditField;
pub use guard::*;

use std::fmt::Debug;

#[derive(Clone, Debug, Default, PartialEq)]
enum LastEdit {
    #[default]
    None,
    Insert,
    Delete,
    Paste,
}

enum EditAction {
    None,
    Activate,
    Edit,
}

/// Used to track ongoing incompatible actions
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CurrentAction {
    #[default]
    None,
    DragSelect,
    ImeStart,
    ImeEdit,
}

impl CurrentAction {
    fn is_select(self) -> bool {
        matches!(self, CurrentAction::DragSelect)
    }

    fn is_ime(self) -> bool {
        matches!(self, CurrentAction::ImeStart | CurrentAction::ImeEdit)
    }

    fn is_active_ime(self) -> bool {
        false
    }

    fn clear_active(&mut self) {
        if matches!(self, CurrentAction::DragSelect | CurrentAction::ImeEdit) {
            *self = CurrentAction::None;
        }
    }

    fn clear_selection(&mut self) {
        if matches!(self, CurrentAction::DragSelect) {
            *self = CurrentAction::None;
        }
    }
}
