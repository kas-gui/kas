// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Abstractions over `kas-text`

pub use kas_text::*;

mod selection;
pub use selection::SelectionHelper;

mod string;
pub use string::AccelString;

pub mod util {
    use super::{format, EditableTextApi, Text, TextApi, Vec2};
    use kas::{geom::Size, TkAction};

    /// Set the text and prepare
    ///
    /// Update text and trigger a resize if necessary.
    ///
    /// The `avail` parameter is used to determine when a resize is required. If
    /// this parameter is a little bit wrong then resizes may sometimes happen
    /// unnecessarily or may not happen when text is slightly too big (e.g.
    /// spills into the margin area); this behaviour is probably acceptable.
    pub fn set_text_and_prepare<T: format::FormattableText>(
        text: &mut Text<T>,
        s: T,
        avail: Size,
    ) -> TkAction {
        text.set_text(s);
        if let Some(req) = text.prepare() {
            let avail = Vec2::from(avail);
            if !(req.0 <= avail.0 && req.1 <= avail.1) {
                return TkAction::RESIZE;
            }
        }
        TkAction::empty()
    }

    /// Set the text from a string and prepare
    ///
    /// Update text and trigger a resize if necessary.
    ///
    /// The `avail` parameter is used to determine when a resize is required. If
    /// this parameter is a little bit wrong then resizes may sometimes happen
    /// unnecessarily or may not happen when text is slightly too big (e.g.
    /// spills into the margin area); this behaviour is probably acceptable.
    pub fn set_string_and_prepare<T: format::EditableText>(
        text: &mut Text<T>,
        s: String,
        avail: Size,
    ) -> TkAction {
        text.set_string(s);
        if let Some(req) = text.prepare() {
            let avail = Vec2::from(avail);
            if !(req.0 <= avail.0 && req.1 <= avail.1) {
                return TkAction::RESIZE;
            }
        }
        TkAction::empty()
    }
}
