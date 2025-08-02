// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Special window widgets

mod popup;
mod window;

#[cfg(feature = "accesskit")]
#[doc(inline)]
pub(crate) use popup::POPUP_INNER_INDEX;
#[doc(inline)] pub use popup::Popup;
#[doc(inline)] pub(crate) use popup::PopupDescriptor;
pub use window::*;

use std::num::NonZeroU32;

/// Available decoration modes
///
/// See [`Window::decorations`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Decorations {
    /// No decorations
    ///
    /// The root widget is drawn as a simple rectangle with no borders.
    None,
    /// Add a simple themed border to the widget
    ///
    /// Probably looks better if [`Window::transparent`] is true.
    Border,
    /// Toolkit-drawn decorations
    ///
    /// Decorations will match the toolkit theme, not the platform theme.
    /// These decorations may not have all the same capabilities.
    ///
    /// Probably looks better if [`Window::transparent`] is true.
    Toolkit,
    /// Server-side decorations
    ///
    /// Decorations are drawn by the window manager, if available.
    Server,
}

/// Identifier for a window or pop-up
///
/// Identifiers should always be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WindowId(NonZeroU32);

impl WindowId {
    pub(crate) fn get(self) -> u32 {
        self.0.get()
    }
}

/// Constructor for [`WindowId`]
#[derive(Default)]
pub(crate) struct WindowIdFactory(u32);

impl WindowIdFactory {
    /// Get the next identifier
    ///
    /// TODO(opt): this should recycle used identifiers since Id does not
    /// efficiently represent large numbers.
    pub(crate) fn make_next(&mut self) -> WindowId {
        let id = self.0 + 1;
        self.0 = id;
        WindowId(NonZeroU32::new(id).unwrap())
    }
}
