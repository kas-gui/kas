// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout, Tile and TileExt traits

use crate::event::ConfigCx;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx};
use crate::util::IdentifyWidget;
use crate::{HasId, Id};
use kas_macros::autoimpl;

#[allow(unused)] use super::{Events, Widget};
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

/// Positioning and drawing routines for [`Widget`]s
///
/// `Tile` is a super-trait of [`Widget`] which:
///
/// -   Has no [`Data`](Widget::Data) parameter
/// -   Supports read-only tree reflection: [`Self::get_child`]
/// -   Provides some basic operations: [`Self::id_ref`], [`Self::rect`]
/// -   Covers sizing and drawing operations from [`Layout`]
///
/// `Tile` may not be implemented directly; it will be implemented by the
/// [`#widget`] macro.
///
/// # Tree reflection
///
/// `Tile` offers a reflection API over the widget tree via
/// [`Tile::get_child`]. This is limited to read-only functions, and thus
/// cannot directly violate the widget lifecycle, however note that the
/// [`id_ref`](Self::id_ref) could be invalid or could be valid but refer to a
/// node which has not yet been sized and positioned (and thus which it is not
/// valid to send events to).
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Tile: Layout {
    /// Get as a `dyn Tile`
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn as_tile(&self) -> &dyn Tile {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Get a reference to the widget's identifier
    ///
    /// The widget identifier is assigned when the widget is configured (see
    /// [`Events::configure`] and [`Events::configure_recurse`]). In case the
    /// [`Id`] is accessed before this, it will be [invalid](Id#invalid-state).
    /// The identifier *may* change when widgets which are descendants of some
    /// dynamic layout are reconfigured.
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn id_ref(&self) -> &Id {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Get the widget's identifier
    ///
    /// This method returns a [`Clone`] of [`Self::id_ref`]. Since cloning an
    /// `Id` is [very cheap](Id#representation), this can mostly be ignored.
    ///
    /// The widget identifier is assigned when the widget is configured (see
    /// [`Events::configure`] and [`Events::configure_recurse`]). In case the
    /// [`Id`] is accessed before this, it will be [invalid](Id#invalid-state).
    /// The identifier *may* change when widgets which are descendants of some
    /// dynamic layout are reconfigured.
    #[inline]
    fn id(&self) -> Id {
        self.id_ref().clone()
    }

    /// Get the widget's region, relative to its parent.
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn rect(&self) -> Rect {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Get the name of the widget struct
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn widget_name(&self) -> &'static str {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    ///
    /// This method is usually implemented automatically by the `#[widget]`
    /// macro. It should be implemented directly if and only if
    /// [`Tile::get_child`] and [`Widget::for_child_node`] are
    /// implemented directly.
    fn num_children(&self) -> usize {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Access a child as a `dyn Tile`
    ///
    /// This method is usually implemented automatically by the `#[widget]`
    /// macro.
    fn get_child(&self, index: usize) -> Option<&dyn Tile> {
        let _ = index;
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    ///
    /// The default implementation simply uses [`Id::next_key_after`].
    /// Widgets may choose to assign children custom keys by overriding this
    /// method and [`Events::make_child_id`].
    #[inline]
    fn find_child_index(&self, id: &Id) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }

    /// Navigation in spatial order
    ///
    /// Controls <kbd>Tab</kbd> navigation order of children.
    /// This method should:
    ///
    /// -   Return `None` if there is no next child
    /// -   Determine the next child after `from` (if provided) or the whole
    ///     range, optionally in `reverse` order
    /// -   Ensure that the selected widget is addressable through
    ///     [`Tile::get_child`]
    ///
    /// Both `from` and the return value use the widget index, as used by
    /// [`Tile::get_child`].
    ///
    /// Default implementation:
    ///
    /// -   Generated from `#[widget]`'s layout property, if used (not always possible!)
    /// -   Otherwise, iterate through children in order of definition
    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        let _ = (reverse, from);
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Get translation of children relative to this widget
    ///
    /// Usually this is zero; only widgets with scrollable or offset content
    /// *and* child widgets need to implement this.
    /// Such widgets must also implement [`Events::handle_scroll`].
    ///
    /// Affects event handling via [`Tile::probe`] and affects the positioning
    /// of pop-up menus. [`Layout::draw`] must be implemented directly using
    /// [`DrawCx::with_clip_region`] to offset contents.
    ///
    /// Default implementation: return [`Offset::ZERO`]
    #[inline]
    fn translation(&self) -> Offset {
        Offset::ZERO
    }

    /// Probe a coordinate for a widget's [`Id`]
    ///
    /// Returns the [`Id`] of the widget expected to handle clicks and touch
    /// events at the given `coord`. Typically this is the lowest descendant in
    /// the widget tree at the given `coord`, but it is not required to be; e.g.
    /// a `Button` may use an inner widget as a label but return its own [`Id`]
    /// to indicate that the button (not the inner label) handles clicks.
    ///
    /// # Calling
    ///
    /// **Prefer to call [`Layout::try_probe`] instead**.
    ///
    /// ## Call order
    ///
    /// It is expected that [`Layout::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// # Implementation
    ///
    /// The callee may usually assume that it occupies `coord` and may thus
    /// return its own [`Id`] when no child occupies the input `coord`.
    ///
    /// ## Default implementation
    ///
    /// ## Default implementation
    ///
    /// The `#[widget]` macro
    /// [may generate a default implementation](macros::widget#layout-1) by
    /// implementing [`LayoutVisitor`] for `Self`.
    /// In this case the default impl of this method is
    /// `self.layout_visitor().set_rect(/* ... */)`.
    /// The underlying implementation considers all children of the `layout`
    /// property and of  fields, like this:
    /// ```ignore
    /// let coord = coord + self.translation();
    /// for child in ITER_OVER_CHILDREN {
    ///     if let Some(id) = child.try_probe(coord) {
    ///         return Some(id);
    ///     }
    /// }
    /// self.id()
    /// ```
    fn probe(&mut self, coord: Coord) -> Id
    where
        Self: Sized,
    {
        let _ = coord;
        unimplemented!() // make rustdoc show that this is a provided method
    }
}

impl<W: Tile + ?Sized> HasId for &W {
    #[inline]
    fn has_id(self) -> Id {
        self.id_ref().clone()
    }
}

impl<W: Tile + ?Sized> HasId for &mut W {
    #[inline]
    fn has_id(self) -> Id {
        self.id_ref().clone()
    }
}

/// Extension trait over widgets
pub trait TileExt: Tile {
    /// Test widget identifier for equality
    ///
    /// This method may be used to test against `Id`, `Option<Id>`
    /// and `Option<&Id>`.
    #[inline]
    fn eq_id<T>(&self, rhs: T) -> bool
    where
        Id: PartialEq<T>,
    {
        *self.id_ref() == rhs
    }

    /// Display as "StructName#Id"
    #[inline]
    fn identify(&self) -> IdentifyWidget {
        IdentifyWidget(self.widget_name(), self.id_ref())
    }

    /// Check whether `id` is self or a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    fn is_ancestor_of(&self, id: &Id) -> bool {
        self.id_ref().is_ancestor_of(id)
    }

    /// Check whether `id` is not self and is a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    fn is_strict_ancestor_of(&self, id: &Id) -> bool {
        !self.eq_id(id) && self.id_ref().is_ancestor_of(id)
    }

    /// Run a closure on all children
    fn for_children(&self, mut f: impl FnMut(&dyn Tile)) {
        for index in 0..self.num_children() {
            if let Some(child) = self.get_child(index) {
                f(child);
            }
        }
    }

    /// Run a fallible closure on all children
    ///
    /// Returns early in case of error.
    fn for_children_try<E>(&self, mut f: impl FnMut(&dyn Tile) -> Result<(), E>) -> Result<(), E> {
        let mut result = Ok(());
        for index in 0..self.num_children() {
            if let Some(child) = self.get_child(index) {
                result = f(child);
            }
            if result.is_err() {
                break;
            }
        }
        result
    }

    /// Find the descendant with this `id`, if any
    ///
    /// Since `id` represents a path, this operation is normally `O(d)` where
    /// `d` is the depth of the path (depending on widget implementations).
    fn find_widget(&self, id: &Id) -> Option<&dyn Tile> {
        if let Some(child) = self.find_child_index(id).and_then(|i| self.get_child(i)) {
            child.find_widget(id)
        } else if self.eq_id(id) {
            Some(self.as_tile())
        } else {
            None
        }
    }
}
impl<W: Tile + ?Sized> TileExt for W {}
