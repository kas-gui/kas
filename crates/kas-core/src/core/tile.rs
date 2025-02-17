// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout, Tile and TileExt traits

use crate::geom::{Coord, Offset, Rect};
use crate::util::IdentifyWidget;
use crate::{HasId, Id, Layout};
use kas_macros::autoimpl;

#[allow(unused)] use super::{Events, Widget};
#[allow(unused)]
use crate::layout::{self, AlignPair};
#[allow(unused)] use crate::theme::DrawCx;
#[allow(unused)] use kas_macros as macros;

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
    /// This method is usually implemented by the `#[widget]` macro.
    /// See also [`kas::widget_set_rect`].
    fn rect(&self) -> Rect;

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
    /// This method returns `None` exactly when `index >= self.num_children()`.
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
    /// -   Return `None` if there is no (next) navigable child
    /// -   In the case there are navigable children and `from == None`, return
    ///     the index of the first (or last if `reverse`) navigable child
    /// -   In the case there are navigable children and `from == Some(index)`,
    ///     it may be expected that `from` is the output of a previous call to
    ///     this method; the method should return the next (or previous if
    ///     `reverse`) navigable child (if any)
    ///
    /// The return value mut be `None` or `Some(index)` where
    /// `self.get_child(index).is_some()` (see [`Tile::get_child`]).
    ///
    /// It is not required that all children (all indices `i` for
    /// `i < self.num_children()`) are returnable from this method.
    ///
    /// Default (macro generated) implementation:
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
    /// The `#[widget]` macro may implement this method as:
    /// ```ignore
    /// let coord = coord + self.translation();
    /// MacroDefinedLayout::try_probe(self, coord).unwrap_or_else(|| self.id())
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
