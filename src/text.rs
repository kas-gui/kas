// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

pub use kas_text::*;

mod string;
pub use string::AccelString;

pub mod util {
    use super::{format, EditableTextApi, Text, TextApi};
    use kas::TkAction;

    /// Set the text and prepare
    ///
    /// This is a convenience function to (1) set the text, (2) call prepare
    /// and (3) return `TkAction` to trigger a redraw.
    ///
    /// This calls [`Text::prepare`] internally, then returns
    /// [`TkAction::Redraw`]. (This does not force a resize.)
    pub fn set_text_and_prepare<T: format::FormattableText>(text: &mut Text<T>, s: T) -> TkAction {
        text.set_text(s);
        text.prepare();
        TkAction::Redraw
    }

    /// Set the text from a string and prepare
    ///
    /// This is a convenience function to (1) set the text, (2) call prepare
    /// and (3) return `TkAction` to trigger a redraw.
    ///
    /// This calls [`Text::prepare`] internally, then returns
    /// [`TkAction::Redraw`]. (This does not force a resize.)
    pub fn set_string_and_prepare<T: format::EditableText>(
        text: &mut Text<T>,
        s: String,
    ) -> TkAction {
        text.set_string(s);
        text.prepare();
        TkAction::Redraw
    }
}
