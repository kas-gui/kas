// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface
//!
//! TODO: "toolkit" is no longer an apt description of this internal API, but
//! rather "theme + renderer".

use crate::widget::{Size, SizePref, Widget, WidgetId};

/// Common widget properties. Implemented by the toolkit.
///
/// Users interact with this trait in a few cases, such as implementing widget
/// event handling. In these cases the user is *always* given an existing
/// reference to a `TkWidget`. Mostly this trait is only used internally.
///
/// Note that it is not necessary for toolkits to implement all of these
/// methods, depending on which functionality from the library is used.
pub trait TkWidget {
    /// Get the widget's size preferences
    fn size_pref(&self, widget: &dyn Widget, pref: SizePref) -> Size;

    /// Set the widget under the mouse
    fn set_hover(&mut self, hover: Option<WidgetId>);
}
