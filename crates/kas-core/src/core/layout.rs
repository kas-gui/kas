// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout and LayoutExt traits

use crate::event::ConfigCx;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx};
use crate::util::IdentifyWidget;
use crate::WidgetId;
use kas_macros::autoimpl;

#[allow(unused)] use crate::layout::{self, AlignPair};
#[allow(unused)] use kas_macros as macros;

/// Positioning and drawing routines for [`Widget`]s
///
/// `Layout` is an unparameterised dyn-safe super-trait of [`Widget`].
/// `Layout` covers sizing and drawing, child enumeration (introspection) and
/// identification via [`WidgetId`].
///
/// # Implementing Layout
///
/// Currently `Layout` may only be implemented alongside [`Widget`] via the
/// [`#widget`] macro. See also [`Widget`] documentation.
///
/// Methods may be categorised as follows:
///
/// -   **Core** methods `as_dyn`, `id_ref`, `rect` and `widget_name` are
///     *always* implemented via the [`#widget`] macro.
/// -   **Introspection** methods `num_children` and `get_child`, are *usually*
///     implemented by [`#widget`] but sometimes must be implemented manually
///     (as a pair). Likewise, the pair `find_child_index` and `make_child_id`
///     usually do not require an explicit implementation.
/// -   **Layout** methods `size_rules`, `set_rect`, `nav_next`, `translation`,
///     `find_id` and `draw` may be implemented manually, left at their defaults
///     (except `size_rules` and `draw`) or implemented via
///     [layout syntax](macros::widget#layout-1).
///
/// # Solving layout
///
/// Layout is resolved as follows:
///
/// 1.  [`Events::configure`] is called (widgets only), and may be used to load assets
/// 2.  [`Self::size_rules`] is called at least once for each axis
/// 3.  [`Self::set_rect`] is called to position elements. This may use data cached by `size_rules`.
/// 4.  [`Self::find_id`] may be used to find the widget under the mouse and [`Self::draw`] to draw
///     elements.
///
/// Usually, [`Layout::size_rules`] methods are called recursively. To instead
/// solve layout for a single widget/layout object, it may be useful to use
/// [`layout::solve_size_rules`] or [`layout::SolveCache`].
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Layout {
    /// Get as a `dyn Layout`
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn as_layout(&self) -> &dyn Layout;

    /// Get the widget's identifier
    ///
    /// Note that the default-constructed [`WidgetId`] is *invalid*: any
    /// operations on this value will cause a panic. Valid identifiers are
    /// assigned by [`Events::pre_configure`].
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn id_ref(&self) -> &WidgetId;

    /// Get the widget's region, relative to its parent.
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn rect(&self) -> Rect;

    /// Get the name of the widget struct
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn widget_name(&self) -> &'static str;

    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    ///
    /// This method is usually implemented automatically by the `#[widget]`
    /// macro. It should be implemented directly if and only if
    /// [`Layout::get_child`] and [`Widget::for_child_node`] are
    /// implemented directly.
    fn num_children(&self) -> usize;

    /// Access a child as a `dyn Layout`
    ///
    /// This method is usually implemented automatically by the `#[widget]`
    /// macro.
    fn get_child(&self, index: usize) -> Option<&dyn Layout>;

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    ///
    /// The default implementation simply uses [`WidgetId::next_key_after`].
    /// Widgets may choose to assign children custom keys by overriding this
    /// method and [`Self::make_child_id`].
    #[inline]
    fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }

    /// Make an identifier for a child
    ///
    /// This is used to configure children. It may return [`WidgetId::default`]
    /// in order to avoid configuring the child, but in this case the widget
    /// must configure via another means.
    ///
    /// Default impl: `self.id_ref().make_child(index)`
    #[inline]
    fn make_child_id(&mut self, index: usize) -> WidgetId {
        self.id_ref().make_child(index)
    }

    /// Get size rules for the given axis
    ///
    /// Typically, this method is called twice: first for the horizontal axis,
    /// second for the vertical axis (with resolved width available through
    /// the `axis` parameter allowing content wrapping).
    /// For a description of the widget size model, see [`SizeRules`].
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
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    ///
    /// This method is called after [`Self::size_rules`] and may use values
    /// cached by `size_rules` (in the case `size_rules` is not called first,
    /// the widget may exhibit incorrect layout but should not panic). This
    /// method should not write over values cached by `size_rules` since
    /// `set_rect` may be called multiple times consecutively.
    /// After `set_rect` is called, the widget must be ready for drawing and event handling.
    ///
    /// The size of the assigned `rect` is normally at least the minimum size
    /// requested by [`Self::size_rules`], but this is not guaranteed. In case
    /// this minimum is not met, it is permissible for the widget to draw
    /// outside of its assigned `rect` and to not function as normal.
    ///
    /// The assigned `rect` may be larger than the widget's size requirements,
    /// regardless of the [`Stretch`] policy used. If the widget should never
    /// stretch, it must align itself.
    /// Example: the `CheckBox` widget uses an [`AlignPair`] (set from
    /// `size_rules`'s [`AxisInfo`]) and uses [`ConfigCx::align_feature`].
    /// Another example: `Label` uses a `Text` object which handles alignment
    /// internally.
    ///
    /// Default implementation when not using the `layout` property: set `rect`
    /// field of `widget_core!()` to the input `rect`.
    ///
    /// [`Stretch`]: crate::layout::Stretch
    fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect);

    /// Navigation in spatial order
    ///
    /// Controls <kbd>Tab</kbd> navigation order of children.
    /// This method should:
    ///
    /// -   Return `None` if there is no next child
    /// -   Determine the next child after `from` (if provided) or the whole
    ///     range, optionally in `reverse` order
    /// -   Ensure that the selected widget is addressable through
    ///     [`Widget::for_child`]
    ///
    /// Both `from` and the return value use the widget index, as used by
    /// [`Widget::for_child`].
    ///
    /// Default implementation:
    ///
    /// -   Generated from `#[widget]`'s layout property, if used (not always possible!)
    /// -   Otherwise, iterate through children in order of definition
    fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize>;

    /// Get translation of children relative to this widget
    ///
    /// Usually this is zero; only widgets with scrollable or offset content
    /// *and* child widgets need to implement this.
    /// Such widgets must also implement [`Events::handle_scroll`].
    ///
    /// Affects event handling via [`Layout::find_id`] and affects the positioning
    /// of pop-up menus. [`Layout::draw`] must be implemented directly using
    /// [`DrawCx::with_clip_region`] to offset contents.
    ///
    /// Default implementation: return [`Offset::ZERO`]
    #[inline]
    fn translation(&self) -> Offset {
        Offset::ZERO
    }

    /// Translate a coordinate to a [`WidgetId`]
    ///
    /// This method is used to determine which widget reacts to the mouse cursor
    /// or a touch event. The result affects mouse-hover highlighting, event
    /// handling by the target, and potentially also event handling by other
    /// widgets (e.g. a `Label` widget will not handle touch events, but if it
    /// is contained by a `ScrollRegion`, that widget may capture these via
    /// [`Events::handle_event`] to implement touch scrolling).
    ///
    /// The result is usually the widget which draws at the given `coord`, but
    /// does not have to be. For example, a `Button` widget will return its own
    /// `id` for coordinates drawn by internal content, while the `CheckButton`
    /// widget uses an internal component for event handling and thus reports
    /// this component's `id` even over its own area.
    ///
    /// It is expected that [`Layout::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// The default implementation suffices for widgets without children as well
    /// as widgets using the `layout` property of [`#[widget]`](crate::widget).
    /// Custom implementations may be required if:
    ///
    /// -   A custom [`Layout`] implementation is used
    /// -   Event stealing or donation is desired (but note that
    ///     `layout = button: ..;` does this already)
    ///
    /// When writing a custom implementation:
    ///
    /// -   Widgets should test `self.rect().contains(coord)`, returning `None`
    ///     if this test is `false`; otherwise, they should always return *some*
    ///     [`WidgetId`], either a childs or their own.
    /// -   If the Widget uses a translated coordinate space (i.e.
    ///     `self.translation() != Offset::ZERO`) then pass
    ///     `coord + self.translation()` to children.
    ///
    /// The default implementation is non-trivial:
    /// ```ignore
    /// if !self.rect().contains(coord) {
    ///     return None;
    /// }
    /// let coord = coord + self.translation();
    /// for child in ITER_OVER_CHILDREN {
    ///     if let Some(id) = child.find_id(coord) {
    ///         return Some(id);
    ///     }
    /// }
    /// Some(self.id())
    /// ```
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId>;

    /// Draw a widget and its children
    ///
    /// This method is invoked each frame to draw visible widgets. It should
    /// draw itself and recurse into all visible children.
    ///
    /// It is expected that [`Self::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// The `draw` parameter is pre-parameterized with this widget's
    /// [`WidgetId`], allowing drawn components to react to input state. This
    /// implies that when calling `draw` on children, the child's `id` must be
    /// supplied via [`DrawCx::re_id`] or [`DrawCx::recurse`].
    fn draw(&mut self, draw: DrawCx);
}

/// Extension trait over widgets
pub trait LayoutExt: Layout {
    /// Get the widget's identifier
    ///
    /// Note that the default-constructed [`WidgetId`] is *invalid*: any
    /// operations on this value will cause a panic. Valid identifiers are
    /// assigned during configure.
    #[inline]
    fn id(&self) -> WidgetId {
        self.id_ref().clone()
    }

    /// Test widget identifier for equality
    ///
    /// This method may be used to test against `WidgetId`, `Option<WidgetId>`
    /// and `Option<&WidgetId>`.
    #[inline]
    fn eq_id<T>(&self, rhs: T) -> bool
    where
        WidgetId: PartialEq<T>,
    {
        *self.id_ref() == rhs
    }

    /// Display as "StructName#WidgetId"
    #[inline]
    fn identify(&self) -> IdentifyWidget {
        IdentifyWidget(self.widget_name(), self.id_ref())
    }

    /// Check whether `id` is self or a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    fn is_ancestor_of(&self, id: &WidgetId) -> bool {
        self.id().is_ancestor_of(id)
    }

    /// Check whether `id` is not self and is a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    fn is_strict_ancestor_of(&self, id: &WidgetId) -> bool {
        !self.eq_id(id) && self.id().is_ancestor_of(id)
    }

    /// Run a closure on all children
    fn for_children(&self, mut f: impl FnMut(&dyn Layout)) {
        for index in 0..self.num_children() {
            if let Some(child) = self.get_child(index) {
                f(child);
            }
        }
    }

    /// Run a fallible closure on all children
    ///
    /// Returns early in case of error.
    fn for_children_try<E>(
        &self,
        mut f: impl FnMut(&dyn Layout) -> Result<(), E>,
    ) -> Result<(), E> {
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
    fn get_id(&self, id: &WidgetId) -> Option<&dyn Layout> {
        if let Some(child) = self.find_child_index(id).and_then(|i| self.get_child(i)) {
            child.get_id(id)
        } else if self.eq_id(id) {
            Some(self.as_layout())
        } else {
            None
        }
    }
}
impl<W: Layout + ?Sized> LayoutExt for W {}