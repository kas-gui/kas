// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout, Tile and TileExt traits

use crate::event::ConfigCx;
use crate::geom::{Coord, Rect};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx};
use crate::Id;
use kas_macros::autoimpl;

#[allow(unused)] use super::{Events, Tile, Widget};
#[allow(unused)]
use crate::layout::{self, AlignPair, LayoutVisitor};
#[allow(unused)] use kas_macros as macros;

/// Positioning and drawing routines for [`Widget`]s
///
/// `Layout` is used to implement [`Widget`] sizing and drawing operations
/// ("layout").
/// See [`Widget`] documentation and the [`#widget`] macro.
/// `Layout` may not be implemented independently.
///
/// # Widget lifecycle
///
/// 1.  The widget is configured ([`Events::configure`]) and immediately updated
///     ([`Events::update`]).
/// 2.  The widget has its size-requirements checked by calling [`Self::size_rules`]
///     for each axis (usually via recursion, sometimes via [`layout::solve_size_rules`]
///     or [`layout::SolveCache`]).
/// 3.  [`Self::set_rect`] is called to position elements. This may use data
///     cached by `size_rules`.
/// 4.  The widget is updated again after any data change (see [`ConfigCx::update`]).
/// 5.  The widget is ready for event-handling and drawing ([`Events`],
///     [`Layout::try_probe`], [`Self::draw`]).
///
/// Widgets are responsible for ensuring that their children may observe this
/// lifecycle. Usually this simply involves inclusion of the child in layout
/// operations. Steps of the lifecycle may be postponed until a widget becomes
/// visible.
///
/// # Implementation
///
/// The [`#widget`] macro will, when its `layout` property is specified,
/// generate an implementation of this trait (if omitted from the surrounding
/// `impl_scope!`) or provide default implementations of its methods (if an
/// explicit impl of `Layout` is found but some methods are missing).
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Layout {
    /// Get size rules for the given axis
    ///
    /// # Calling
    ///
    /// This method is called during sizing (see
    /// [widget lifecycle](Self#widget-lifecycle)).
    /// Typically, this method is called twice: first for the horizontal axis,
    /// second for the vertical axis (with resolved width available through
    /// the `axis` parameter allowing content wrapping).
    /// For a description of the widget size model, see [`SizeRules`].
    ///
    /// ## Call order
    ///
    /// Required: `self` is configured ([`ConfigCx::configure`]) before this
    /// method is called, and that `size_rules` is called for the
    /// horizontal axis before it is called for the vertical axis.
    /// Further, [`Self::set_rect`] must be called after this method before
    /// drawing or event handling.
    ///
    /// # Implementation
    ///
    /// This method is expected to cache any size requirements calculated from
    /// children which would be required for space allocations in
    /// [`Self::set_rect`]. As an example, the horizontal [`SizeRules`] for a
    /// row layout is the sum of the rules for each column (plus margins);
    /// these per-column [`SizeRules`] are also needed to calculate column
    /// widths in [`Self::size_rules`] once the available size is known.
    ///
    /// For row/column/grid layouts, a [`crate::layout::RulesSolver`] engine
    /// may be useful.
    ///
    /// ## Default implementation
    ///
    /// The `#[widget]` macro
    /// [may generate a default implementation](macros::widget#layout-1) by
    /// implementing [`LayoutVisitor`] for `Self`.
    /// In this case the default impl of this method is
    /// `self.layout_visitor().size_rules(/* ... */)`.
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    ///
    /// # Calling
    ///
    /// This method is called after [`Self::size_rules`] and may use values
    /// cached by `size_rules` (in the case `size_rules` is not called first,
    /// the widget may exhibit incorrect layout but should not panic). This
    /// method should not write over values cached by `size_rules` since
    /// `set_rect` may be called multiple times consecutively.
    /// After `set_rect` is called, the widget must be ready for drawing and event handling.
    ///
    /// ## Call order
    ///
    /// Required: [`Self::size_rules`] is called for both axes before this
    /// method is called, and that this method has been called *after* the last
    /// call to [`Self::size_rules`] *before* any of the following methods:
    /// [`Layout::try_probe`], [`Layout::draw`], [`Events::handle_event`].
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
    /// The `#[widget]` macro
    /// [may generate a default implementation](macros::widget#layout-1) by
    /// implementing [`LayoutVisitor`] for `Self`.
    /// In this case the default impl of this method is
    /// `self.layout_visitor().set_rect(/* ... */)`.
    ///
    /// [`Stretch`]: crate::layout::Stretch
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints);

    /// Probe a coordinate for a widget's [`Id`]
    ///
    /// Returns the [`Id`] of the widget expected to handle clicks and touch
    /// events at the given `coord`, or `None` if `self` does not occupy this
    /// `coord`. Typically the result is the lowest descendant in
    /// the widget tree at the given `coord`, but it is not required to be; e.g.
    /// a `Button` may use an inner widget as a label but return its own [`Id`]
    /// to indicate that the button (not the inner label) handles clicks.
    ///
    /// # Calling
    ///
    /// ## Call order
    ///
    /// It is expected that [`Layout::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// # Implementation
    ///
    /// Widgets should implement [`Tile::probe`] instead, in which case an
    /// implemention of this method will be provided:
    /// ```ignore
    /// self.rect().contains(coord).then(|| self.probe(coord))
    /// ```
    /// Derive-mode widgets may implement either method.
    ///
    /// ## Default implementation
    ///
    /// Non-widgets do not have an [`Id`], and therefore should use the default
    /// implementation which simply returns `None`.
    fn try_probe(&mut self, coord: Coord) -> Option<Id> {
        let _ = coord;
        None
    }

    /// Draw a widget and its children
    ///
    /// # Calling
    ///
    /// This method is invoked each frame to draw visible widgets. It should
    /// draw itself and recurse into all visible children.
    ///
    /// ## Call order
    ///
    /// It is expected that [`Self::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// # Implementation
    ///
    /// ## Default implementation
    ///
    /// The `#[widget]` macro
    /// [may generate a default implementation](macros::widget#layout-1) by
    /// implementing [`LayoutVisitor`] for `Self`.
    /// In this case the default impl of this method is
    /// `self.layout_visitor().draw(/* ... */)`.
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
    fn draw(&mut self, draw: DrawCx);
}

/// Macro-defined layout
///
/// This trait is a copy of [`Layout`], implemented automatically for custom
/// widgets with macro-defined layout. It may be useful for small hacks where
/// the macro-generated layout implementations should still be used, but with
/// some addition or modification of inputs.
///
/// TODO: add an example
pub trait MacroDefinedLayout {
    /// Get size rules for the given axis
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints);

    /// Probe a coordinate for a widget's [`Id`]
    fn try_probe(&mut self, coord: Coord) -> Option<Id>;

    /// Draw a widget and its children
    fn draw(&mut self, draw: DrawCx);
}
