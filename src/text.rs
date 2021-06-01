// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text functionality
//!
//! Most of this module is simply a re-export of the [KAS Text] API, hence the
//! lower level of integration than other parts of the library. The [`util`]
//! module is an extension providing some integration.
//!
//! [KAS Text]: https://github.com/kas-gui/kas-text/

pub use kas_text::*;

mod selection;
pub use selection::SelectionHelper;

mod string;
pub use string::AccelString;

/// Utilities integrating `kas-text` functionality
pub mod util {
    use super::{fonts, format, EditableTextApi, Text, TextApi, Vec2};
    use kas::{geom::Size, TkAction};
    use log::trace;

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
        prepare_if_needed(text, avail)
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
        prepare_if_needed(text, avail)
    }

    /// Do text preparation, if required/possible
    ///
    /// TODO(opt): this method may trigger a RESIZE even though in some cases
    /// this does nothing useful (e.g. the widget cannot be made bigger anyway).
    ///
    /// Note: an alternative approach would be to delay text preparation by
    /// adding TkAction::PREPARE and a new method, perhaps in Layout.
    fn prepare_if_needed<T: format::FormattableText>(text: &mut Text<T>, avail: Size) -> TkAction {
        if fonts::fonts().num_faces() == 0 {
            // Fonts not loaded yet: cannot prepare and can assume it will happen later anyway.
            return TkAction::empty();
        }
        if let Some(req) = text.prepare() {
            let avail = Vec2::from(avail);
            if !(req.0 <= avail.0 && req.1 <= avail.1) {
                trace!(
                    "set_text_and_prepare triggers RESIZE: req={:?}, avail={:?}",
                    req,
                    avail
                );
                return TkAction::RESIZE;
            }
        }
        TkAction::REDRAW
    }
}
