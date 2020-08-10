// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

use kas::TkAction;
pub use kas_text::*;

#[doc(no_inline)]
pub use rich::Text as RichText;

#[doc(no_inline)]
pub use prepared::Text as PreparedText;

#[doc(no_inline)]
pub use prepared::Prepare as PrepareAction;

/// Extension trait over [`prepared::Text`]
pub trait PreparedTextExt {
    /// Set the text
    ///
    /// This calls [`PreparedText::prepare`] internally, then returns
    /// [`TkAction::Redraw`]. (This does not force a resize.)
    fn set_and_prepare<T: Into<RichText>>(&mut self, text: T) -> TkAction;
}

impl PreparedTextExt for PreparedText {
    fn set_and_prepare<T: Into<RichText>>(&mut self, text: T) -> TkAction {
        if self.set_text(text.into()).prepare() {
            self.prepare();
            TkAction::Redraw
        } else {
            TkAction::None
        }
    }
}
