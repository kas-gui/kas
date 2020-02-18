// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! High-level drawing interface
//!
//! A [`Theme`] provides a high-level drawing interface. It may be provided by
//! the toolkit or separately (but dependent on a toolkit's drawing API).
//!
//! A theme is implemented in multiple parts: the [`Theme`] object is shared
//! by all windows and may provide shared resources (e.g. fonts and textures).
//! It is also responsible for creating a per-window [`Window`] object.
//!
//! Finally, the [`SizeHandle`] and [`DrawHandle`] traits provide actual sizing
//! and drawing information for widgets. Widgets are provided implementations of
//! these traits within calls to the appropriate [`Widget`] methods.
//!
//! [`Widget`]: crate::Widget

use kas::Align;

#[cfg(all(feature = "stack_dst", not(feature = "gat")))]
mod theme_dst;
mod theme_handle;
mod theme_traits;

#[cfg(all(feature = "stack_dst", not(feature = "gat")))]
pub use theme_dst::{StackDst, ThemeDst, WindowDst};
pub use theme_handle::{DrawHandle, SizeHandle};
pub use theme_traits::{Theme, ThemeApi, Window};

/// Class of text drawn
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum TextClass {
    /// Label text is drawn over the background colour
    Label,
    /// Button text is drawn over a button
    Button,
    /// Class of text drawn in a single-line edit box
    Edit,
    /// Class of text drawn in a multi-line edit box
    EditMulti,
}

/// Text alignment, class, etc.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextProperties {
    /// Class of text
    pub class: TextClass,
    /// Horizontal alignment
    pub horiz: Align,
    /// Vertical alignment
    pub vert: Align,
    // Note: do we want to add HighlightState?
}

/// Toolkit actions needed after theme adjustment, if any
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThemeAction {
    /// No action needed
    None,
    /// All windows require redrawing
    RedrawAll,
    /// Theme sizes changed: must call [`Theme::update_window`] and resize
    ThemeResize,
}
