// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout, Tile and TileExt traits

use crate::geom::Rect;
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx};
use kas_macros::autoimpl;

#[allow(unused)] use super::{Events, Tile, Widget};
#[allow(unused)] use crate::layout::{self};
#[allow(unused)] use kas_macros as macros;

/// Positioning and drawing routines for [`Widget`]s
///
/// `Layout` is used to implement [`Widget`] sizing and drawing operations
/// ("layout").
/// See [`Widget`] documentation and the [`#widget`] macro.
/// `Layout` may not be implemented independently.
///
/// # Implementation
///
/// The [`#widget`] macro will, when its `layout` property is specified,
/// generate an implementation of this trait (if omitted from the surrounding
/// `#[impl_self]`) or provide default implementations of its methods (if an
/// explicit impl of `Layout` is found but some methods are missing).
///
/// # Call order
///
/// Widgets must be [**configured**](Events#configuration) before any
/// `Layout` methods are called. This is not applicable to non-widgets.
///
/// ### Sizing
///
/// Sizing involves calling the following in order:
///
/// 1.  [`Layout::size_rules`] for the horizontal axis
/// 2.  [`Layout::size_rules`] for the vertical axis
/// 3.  [`Layout::set_rect`]
///
/// This order is required initially. Resizing may start at any of the above
/// steps but must then proceed in-order for all remaining steps. If the scale
/// factor is changed, then resizing must start from step 1.
///
/// Typically parent widgets call these methods from their own implementations
/// of [`Layout::size_rules`] and [`Layout::set_rect`]. When calling these
/// methods at other times, be sure to respect the call order.
///
/// Other `Layout` methods may only be called once sizing is complete.
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Layout {
    /// Get the widget's region
    ///
    /// Coordinates are relative to the parent's coordinate space.
    ///
    /// This method is usually implemented by the `#[widget]` macro.
    /// See also [`widget_set_rect!()`](crate::widget_set_rect).
    fn rect(&self) -> Rect;

    /// Calculate size requirements for an `axis`
    ///
    /// This method is used both to initialize a widget at a given scale factor
    /// and to assess size requirements.
    ///
    /// # Calling
    ///
    /// This method is called during sizing (see [above](Layout#call-order)).
    ///
    /// # Implementation
    ///
    /// For a description of the widget size model, see [`SizeRules`].
    ///
    /// For row/column/grid layouts, a [`crate::layout::RulesSolver`] engine
    /// may be useful.
    ///
    /// ## Default implementation
    ///
    /// The [`#[layout]`](macro@crate::layout) macro may be used to
    /// provide a default implementation.
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    ///
    /// # Calling
    ///
    /// This method is called to finalize sizing (see
    /// [above](Layout#call-order)).
    ///
    /// # Implementation
    ///
    /// The size of the assigned `rect` is normally at least the minimum size
    /// requested by [`Self::size_rules`], but this is not guaranteed. In case
    /// this minimum is not met, it is permissible for the widget to draw
    /// outside of its assigned `rect` and to not function as normal.
    ///
    /// The assigned `rect` may be larger than the widget's size requirements,
    /// regardless of the [`Stretch`] policy used: containers divide up space
    /// based on children's [`SizeRules`] but do not attempt to align content
    /// when excess space is available. Instead, content is responsible for
    /// aligning itself using the provided `hints` and/or local information.
    ///
    /// ## Default implementation
    ///
    /// The [`#[layout]`](macro@crate::layout) macro may be used to
    /// provide a default implementation.
    ///
    /// [`Stretch`]: crate::layout::Stretch
    fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints);

    /// Draw a widget and its children
    ///
    /// # Calling
    ///
    /// This method is invoked each frame to draw visible widgets. It should
    /// draw itself and recurse into all visible children.
    ///
    /// # Implementation
    ///
    /// ## Default implementation
    ///
    /// The [`#[layout]`](macro@crate::layout) macro may be used to
    /// provide a default implementation.
    ///
    /// ## Method modification
    ///
    /// The `#[widget]` macro injects a call to [`DrawCx::set_id`] into this
    /// method where possible, allowing correct detection of disabled and
    /// highlight states.
    ///
    /// This method modification should never cause issues (besides the implied
    /// limitation that widgets cannot easily detect a parent's state while
    /// being drawn).
    fn draw(&self, draw: DrawCx);
}

/// Macro-defined layout
///
/// This trait is a copy of [`Layout`], implemented by the
/// [`#[layout]`](macro@crate::layout) macro. In some cases it is useful to
/// invoke `kas::MacroDefinedLayout::set_rect` (or other method) from the
/// corresponding [`Layout`] method to perform some other action before using
/// the default implementation.
///
/// [`#[layout]`]: kas::layout
pub trait MacroDefinedLayout {
    /// Get the widget's region
    fn rect(&self) -> Rect;

    /// Get size rules for the given axis
    fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints);

    /// Draw a widget and its children
    fn draw(&self, draw: DrawCx);
}
