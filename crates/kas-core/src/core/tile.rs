// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout, Tile and TileExt traits

use crate::geom::{Coord, Offset, Rect};
use crate::util::IdentifyWidget;
use crate::{ChildIndices, HasId, Id, Layout, Role, RoleCx};
use kas_macros::autoimpl;

#[allow(unused)] use super::{Events, RoleCxExt, Widget};
#[allow(unused)] use crate::layout::{self, AlignPair};
#[allow(unused)] use crate::theme::DrawCx;
#[allow(unused)] use kas_macros as macros;

/// A sizable, drawable, identifiable, introspectible 2D tree object
///
/// `Tile` is a super-trait of [`Widget`] which:
///
/// -   Is a sub-trait of [`Layout`]: is sizable and drawable
/// -   Has no [`Data`](Widget::Data) parameter or event-handling methods
/// -   Has an [identifier](Self::identify) and [`Self::role`]
/// -   Supports read-only tree reflection: [`Self::child_indices`], [`Self::get_child`]
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

    /// Return a [`Display`]-able widget identifier
    ///
    /// This method is implemented by the `#[widget]` macro.
    ///
    /// [`Display`]: std::fmt::Display
    fn identify(&self) -> IdentifyWidget<'_> {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Whether this widget supports navigation focus
    ///
    /// By default this is false.
    fn navigable(&self) -> bool {
        false
    }

    /// Tooltip
    ///
    /// This is shown on mouse hover and may or may not use the same text as the
    /// label defined by the role.
    ///
    /// By default this is `None`.
    fn tooltip(&self) -> Option<&str> {
        None
    }

    /// Describe the widget's role
    ///
    /// This descriptor supports accessibility tooling and UI introspection.
    ///
    /// ## Default implementation
    ///
    /// If the widget has children with macro-defined layout, the `#[widget]`
    /// macro will implement this method to return [`Role::None`]; otherwise,
    /// the default implementation simply returns [`Role::Unknown`].
    fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
        let _ = cx;
        Role::Unknown
    }

    /// Specify additional role properties for child `index`
    ///
    /// Role properties (for example [`RoleCxExt::set_label`]) may be specified
    /// by a widget for itself in [`Self::role`] or by a parent in this method.
    ///
    /// The default implementation does nothing.
    #[inline]
    fn role_child_properties(&self, cx: &mut dyn RoleCx, index: usize) {
        let _ = (cx, index);
    }

    /// Get child indices available to recursion
    ///
    /// This method returns a range of child indices for "visible" children.
    /// These children are expected to be accessible through [`Self::get_child`]
    /// and to be configured and sized (assuming that `self` is). They may
    /// or may not be visible on the screen.
    ///
    /// Hidden children might be excluded from this list, for example a
    /// collapsed element, closed menu or inactive stack page. (Such children
    /// might also not be configured or sized.)
    ///
    /// This method is generated by default unless [`Self::get_child`] has an
    /// explicit implementation. A non-default implementation may be needed when
    /// a child should be hidden. In this case it might be necessary to write
    /// an explicit implementation of [`Events::configure_recurse`].
    fn child_indices(&self) -> ChildIndices {
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Access a child as a `dyn Tile`, if available
    ///
    /// Valid `index` values may be discovered by calling
    /// [`Tile::child_indices`], [`Tile::find_child_index`] or
    /// [`Tile::nav_next`]. The `index`-to-child mapping is not
    /// required to remain fixed; use an [`Id`] to track a widget over time.
    ///
    /// This method is usually implemented automatically by the `#[widget]`
    /// macro, but must be implemented explicitly in some cases (e.g. if
    /// children are not marked with `#[widget]` or `#[collection]`).
    fn get_child(&self, index: usize) -> Option<&dyn Tile> {
        let _ = index;
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// As with [`Self::child_indices`], this method should only return
    /// `Some(index)` when [`Self::get_child`] succeeds on the given `index`,
    /// returning access to a child which is configured and sized (assuming that
    /// `self` is). `find_child_index` should return `None` if the above
    /// conditions would not be met, even if `id` identifies a known child.
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
    /// The return value is expected to be `None` or `Some(index)` where
    /// [`Self::get_child`] returns access to a child at the given `index` and
    /// that child is configured and sized (assuming that `self` is).
    ///
    /// Default (macro generated) implementation:
    ///
    /// -   Generated from `#[widget]`'s layout property, if used (not always possible!)
    /// -   Otherwise, iterate through children in order of definition
    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        let _ = (reverse, from);
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Get translation of child `index` relative to this widget
    ///
    /// Usually this is zero; only widgets with scrollable or offset content
    /// *and* child widgets need to implement this.
    /// Such widgets must also implement [`Events::handle_scroll`] and
    /// [`Tile::probe`].
    ///
    /// Affects event handling via [`Tile::probe`] and affects the positioning
    /// of pop-up menus. [`Layout::draw`] must be implemented directly using
    /// [`DrawCx::with_clip_region`] to offset contents.
    ///
    /// Default implementation: return [`Offset::ZERO`]
    #[inline]
    fn translation(&self, index: usize) -> Offset {
        let _ = index;
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
    /// If the [`Self::translation`] is non-zero for any child, then the
    /// coordinate passed to that child must be translated:
    /// `coord + translation`.
    ///
    /// ## Default implementation
    ///
    /// The `#[widget]` macro may implement this method as:
    /// ```ignore
    /// MacroDefinedLayout::try_probe(self, coord).unwrap_or_else(|| self.id())
    /// ```
    fn probe(&self, coord: Coord) -> Id
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

    /// Return an iterator over visible children
    ///
    /// This method may exclude hidden children: see [`Tile::child_indices`].
    fn children(&self) -> impl Iterator<Item = &dyn Tile> {
        self.child_indices()
            .into_iter()
            .flat_map(|i| self.get_child(i))
    }

    /// Find the descendant with this `id`, if any
    ///
    /// Since `id` represents a path, this operation is normally `O(d)` where
    /// `d` is the depth of the path (depending on widget implementations).
    fn find_tile(&self, id: &Id) -> Option<&dyn Tile> {
        if let Some(child) = self.find_child_index(id).and_then(|i| self.get_child(i)) {
            child.find_tile(id)
        } else if self.eq_id(id) {
            Some(self.as_tile())
        } else {
            None
        }
    }

    /// Find the [`Rect`] of the descendant with this `id`, if any
    ///
    /// The [`Rect`] is returned in the widgets own coordinate space where this
    /// space is translated by the [`Offset`] returned. The result is thus
    /// `rect + translation` in the caller's coordinate space.
    fn find_tile_rect(&self, id: &Id) -> Option<(Rect, Offset)> {
        let mut widget = self.as_tile();
        let mut translation = Offset::ZERO;
        loop {
            if widget.eq_id(id) {
                let rect = widget.rect();
                return Some((rect, translation));
            }

            let index = widget.find_child_index(id)?;
            translation += widget.translation(index);
            widget = widget.get_child(index)?;
        }
    }
}
impl<W: Tile + ?Sized> TileExt for W {}
