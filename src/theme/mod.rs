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

use std::ops::DerefMut;

use kas::Align;

mod theme_handle;

pub use theme_handle::{DrawHandle, SizeHandle};

#[cfg(feature = "stack_dst")]
/// Fixed-size object of `Unsized` type
///
/// This is a re-export of
/// [`stack_dst::ValueA`](https://docs.rs/stack_dst/0.6.0/stack_dst/struct.ValueA.html)
/// with a custom size. The `new` and `new_or_boxed` methods provide a
/// convenient API.
pub type StackDst<T> = stack_dst::ValueA<T, [usize; 8]>;

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

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`ThemeAction`] to enable correct action when a theme
/// is updated via [`Manager::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
///
/// [`Manager::adjust_theme`]: crate::event::Manager::adjust_theme
pub trait ThemeApi {
    /// Set font size. Default is 18. Units are unknown.
    fn set_font_size(&mut self, size: f32) -> ThemeAction;

    /// Change the colour scheme
    ///
    /// If no theme by this name is found, the theme is unchanged.
    // TODO: revise scheme identification and error handling?
    fn set_colours(&mut self, _scheme: &str) -> ThemeAction;

    /// Change the theme itself
    ///
    /// Themes may do nothing, or may react according to their own
    /// interpretation of this method.
    fn set_theme(&mut self, _theme: &str) -> ThemeAction {
        ThemeAction::None
    }
}

impl<T: ThemeApi> ThemeApi for Box<T> {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        self.deref_mut().set_font_size(size)
    }
    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        self.deref_mut().set_colours(scheme)
    }
    fn set_theme(&mut self, theme: &str) -> ThemeAction {
        self.deref_mut().set_theme(theme)
    }
}
