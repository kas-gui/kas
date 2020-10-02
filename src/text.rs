// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

use kas::TkAction;
pub use kas_text::*;

/// Extension trait over [`Text`]
pub trait TextExt {
    /// Set the text
    ///
    /// This calls [`Text::prepare`] internally, then returns
    /// [`TkAction::Redraw`]. (This does not force a resize.)
    fn set_and_prepare<S: Into<FormattedString>>(&mut self, text: S) -> TkAction {
        self.set_and_prepare_formatted(text.into())
    }

    fn set_and_prepare_formatted(&mut self, text: FormattedString) -> TkAction;
}

impl TextExt for Text {
    fn set_and_prepare_formatted(&mut self, text: FormattedString) -> TkAction {
        if self.set_text(text).prepare() {
            self.prepare();
            TkAction::Redraw
        } else {
            TkAction::None
        }
    }
}
