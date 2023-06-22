// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window trait and identifier

use super::{Icon, Widget};
use std::num::NonZeroU32;

/// Identifier for a window or pop-up
///
/// Identifiers should always be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WindowId(NonZeroU32);

impl WindowId {
    /// Construct a [`WindowId`]
    ///
    /// Only for use by the shell!
    #[allow(unused)]
    pub(crate) fn new(n: NonZeroU32) -> WindowId {
        WindowId(n)
    }
}

/// Available decoration modes
///
/// See [`Window::decorations`].
#[derive(PartialEq, Eq, Hash, Debug)]
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

/// Functionality required by a window
pub trait Window: Widget {
    /// Get the window title
    fn title(&self) -> &str;

    /// Get the window icon, if any
    ///
    /// Default: `None`
    fn icon(&self) -> Option<Icon> {
        None
    }

    /// Get the preference for window decorations
    ///
    /// "Windowing" platforms (i.e. not mobile or web) usually include a
    /// title-bar, icons and potentially side borders. These are known as
    /// **decorations**.
    ///
    /// This controls the *preferred* type of decorations on windowing
    /// platforms. It is not always followed (e.g. Wayland does not support
    /// server-side decorations by default).
    ///
    /// Default: [`Decorations::Server`].
    fn decorations(&self) -> Decorations {
        Decorations::Server
    }

    /// Whether to limit the maximum size of a window
    ///
    /// All widgets' size rules allow calculation of two sizes: the minimum
    /// size and the ideal size. Windows are initially sized to the ideal size.
    /// This option controls whether the window size is restricted by the
    /// calculated minimum size and by the ideal size.
    ///
    /// Return value is `(restrict_min, restrict_max)`. Suggested is to use
    /// `(true, true)` for simple dialog boxes and `(true, false)` for complex
    /// windows.
    ///
    /// Default: `(true, false)`
    fn restrict_dimensions(&self) -> (bool, bool) {
        (true, false)
    }

    /// Whether to allow dragging the window from the background
    ///
    /// If true, then any unhandled click+drag in the window may be used to
    /// drag the window. Probably more useful for small pop-ups than large
    /// windows.
    ///
    /// Default: `true`.
    fn drag_anywhere(&self) -> bool {
        true
    }

    /// Whether the window supports transparency
    ///
    /// If true, painting with `alpha < 1.0` makes the background visible.
    /// Additionally, window draw targets are cleared to transparent. This does
    /// not stop theme elements from drawing a solid background.
    ///
    /// Note: results may vary by platform. Current output does *not* use
    /// pre-multiplied alpha which *some* platforms expect, thus pixels with
    /// partial transparency may have incorrect appearance.
    ///
    /// Default: `false`.
    fn transparent(&self) -> bool {
        false
    }
}
