// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::fmt;

use crate::event::{self, Event, EventMgr, Response, Scroll};
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AlignHints, AxisInfo, SetRectMgr, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::util::IdentifyWidget;
use crate::WidgetId;
use kas_macros::autoimpl;

#[allow(unused)]
use crate::event::EventState;
#[allow(unused)]
use crate::layout::{self, AutoLayout};

/// Base widget functionality
///
/// See the [`Widget`] trait for documentation of the widget family.
///
/// This trait **must** be implement by the [`derive(Widget)`] macro.
/// Users **must not** implement this `WidgetCore` trait manually or may face
/// unexpected breaking changes.
///
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
#[autoimpl(for<'a, T: trait + ?Sized> &'a mut T, Box<T>)]
pub trait WidgetCore: fmt::Debug {
    /// Get the widget's identifier
    ///
    /// Note that the default-constructed [`WidgetId`] is *invalid*: any
    /// operations on this value will cause a panic. Valid identifiers are
    /// assigned by [`Widget::pre_configure`].
    fn id_ref(&self) -> &WidgetId;

    /// Get the widget's region, relative to its parent.
    fn rect(&self) -> Rect;

    /// Get the name of the widget struct
    fn widget_name(&self) -> &'static str;

    /// Erase type
    fn as_widget(&self) -> &dyn Widget;
    /// Erase type
    fn as_widget_mut(&mut self) -> &mut dyn Widget;
}

/// Listing of a widget's children
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless `#[widget(children = noauto)]` is used.
///
/// Dynamic widgets must implement this trait manually, since [`derive(Widget)`]
/// cannot currently handle fields like `Vec<SomeWidget>`. Additionally, any
/// parent adding child widgets must ensure they get configured by calling
/// [`SetRectMgr::configure`].
///
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
#[autoimpl(for<'a, T: trait + ?Sized> &'a mut T, Box<T>)]
pub trait WidgetChildren: WidgetCore {
    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    fn num_children(&self) -> usize;

    /// Get a reference to a child widget by index, or `None` if the index is
    /// out of bounds.
    ///
    /// For convenience, `Index<usize>` is implemented via this method.
    ///
    /// Required: `index < self.len()`.
    fn get_child(&self, index: usize) -> Option<&dyn Widget>;

    /// Mutable variant of get
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`EventState::send_action`].
    /// This method may be removed in the future.
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget>;

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    ///
    /// The default implementation simply uses [`WidgetId::next_key_after`].
    /// Widgets may choose to assign children custom keys by overriding this
    /// method and [`Widget::make_child_id`].
    #[inline]
    fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }
}

/// Positioning and drawing routines for widgets
///
/// This trait is related to [`Widget`], but may be used independently.
///
/// There are two methods of implementing this trait:
///
/// -   Use the `#[widget{ layout = .. }]` property (see [`#[widget]`] documentation)
/// -   Implement manually. When part of a widget, the [`Self::set_rect`] and
///     [`Self::find_id`] methods gain default implementations (generated by the
///     [`#[widget]`] macro).
///
/// Layout is resolved as follows:
///
/// 1.  [`Widget::configure`] is called (widgets only), and may be used to load assets
/// 2.  [`Self::size_rules`] is called at least once for each axis
/// 3.  [`Self::set_rect`] is called to position elements. This may use data cached by `size_rules`.
/// 4.  [`Self::find_id`] may be used to find the widget under the mouse and [`Self::draw`] to draw
///     elements.
///
/// Two methods of setting layout are possible:
///
/// 1.  Use [`layout::solve_size_rules`] or [`layout::SolveCache`] to solve and
///     set layout. This functions by calling [`Self::size_rules`] for each
///     axis then calling [`Self::set_rect`].
/// 2.  Only call [`Self::set_rect`]. For some widgets this is fine but for
///     others the internal layout will be incorrect.
///
/// [`#[widget]`]: kas_macros::widget
#[autoimpl(for<'a, T: trait + ?Sized> &'a mut T, Box<T>)]
pub trait Layout {
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
    ///
    /// Default implementation:
    ///
    /// -   No default implementation, except,
    /// -   For a widget with the `layout` property, call [`AutoLayout::size_rules`]
    ///
    /// [`#[widget]`]: kas_macros::widget
    fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules;

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
    /// regardless of the [`Stretch`] policy used.
    /// It is up to the widget to either stretch to occupy this space or align
    /// itself within the excess space, according to the `align` hints provided.
    ///
    /// Default implementation:
    ///
    /// -   Independent usage: no default
    /// -   For a widget without `layout` property, set `rect` field of `widget_core!()`
    /// -   For a widget with the `layout` property, set `rect` field of `widget_core!()` and
    ///     call [`AutoLayout::set_rect`]
    ///
    /// Default: set `rect` of `widget_core!()` field. If `layout = ..` property
    /// is used, also calls `<Self as AutoLayout>::set_rect`.
    ///
    /// [`Stretch`]: crate::layout::Stretch
    /// [`#[widget]`]: kas_macros::widget
    fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints);

    /// Translate a coordinate to a [`WidgetId`]
    ///
    /// This method is used in event handling, translating a mouse click or
    /// touch input to a widget.
    /// Usually, this is the widget which draws the target coordinate, but
    /// stealing focus is permitted: e.g. the `Button` widget will return its
    /// own [`WidgetId`] when a user clicks on its inner content.
    ///
    /// It is expected that [`Layout::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// The default implementation suffices unless:
    ///
    /// -   The `layout` property of [`#[widget]`] is not used but
    ///     there are child widgets
    /// -   Event stealing from child widgets is desired (but note that
    ///     `layout = button: ..;` does this already)
    /// -   The child widget is in a translated coordinate space *not equal* to
    ///     [`Widget::translation`]
    ///
    /// To implement directly:
    ///
    /// -   Return `None` if `coord` is not within `self.rect()`
    /// -   Find the child which should respond to input at `coord`, if any, and
    ///     call `find_id` recursively on this child
    /// -   Otherwise return `self.id()`
    ///
    /// Default implementation:
    ///
    /// -   Independent usage: no default
    /// -   For a widget without `layout` property, `self.rect().contains(coord).then(|| self.id())`
    /// -   For a widget with the `layout` property, return `None` if
    ///     `!self.rect().contains(coord)`, otherwise call [`AutoLayout::find_id`] with the
    ///     coord translated by [`Widget::translation`].
    ///
    /// [`#[widget]`]: kas_macros::widget
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId>;

    /// Draw a widget and its children
    ///
    /// This method is invoked each frame to draw visible widgets. It should
    /// draw itself and recurse into all visible children.
    ///
    /// It is expected that [`Self::set_rect`] is called before this method,
    /// but failure to do so should not cause a fatal error.
    ///
    /// Default implementation:
    ///
    /// -   No default implementation, except,
    /// -   For a widget with the `layout` property, call [`AutoLayout::draw`]
    fn draw(&mut self, draw: DrawMgr);
}

/// Widget trait
///
/// Widgets must implement a family of traits, of which this trait is the final
/// member:
///
/// -   [`WidgetCore`] — base functionality (this trait is *always* derived)
/// -   [`WidgetChildren`] — enumerates children and provides methods derived
///     from this
/// -   [`Layout`] — handles sizing and positioning of self and children
/// -   [`Widget`] — the final trait
///
/// Widgets **must** use the [`derive(Widget)`] macro to implement at least
/// [`WidgetCore`] and [`Widget`]; these two traits **must not** be implemented
/// manually or users may face unexpected breaking changes.
/// This macro can optionally implement *all* above traits, and by default will
/// implement *all except for `Layout`*. This opt-out derive behaviour means
/// that adding additional traits into the family is not a breaking change.
///
/// To refer to a widget via dyn trait, use `&dyn Widget`.
/// To refer to a widget in generic functions, use `<W: Widget>`.
///
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
#[autoimpl(for<'a, T: trait + ?Sized> &'a mut T, Box<T>)]
pub trait Widget: WidgetChildren + Layout {
    /// Make an identifier for a child
    ///
    /// Default impl: `self.id_ref().make_child(index)`
    #[inline]
    fn make_child_id(&mut self, index: usize) -> WidgetId {
        self.id_ref().make_child(index)
    }

    /// Pre-configuration
    ///
    /// This method is called before children are configured to assign a
    /// [`WidgetId`]. Usually it does nothing else, but a custom implementation
    /// may be used to affect child configuration, e.g. via
    /// [`EventState::new_accel_layer`].
    ///
    /// Default impl: assign `id` to self
    fn pre_configure(&mut self, mgr: &mut SetRectMgr, id: WidgetId);

    /// Configure widget
    ///
    /// Widgets are *configured* on window creation or dynamically via the
    /// parent calling [`SetRectMgr::configure`]. Parent widgets are responsible
    /// for ensuring that children are configured before calling
    /// [`Layout::size_rules`] or [`Layout::set_rect`]. Configuration may be
    /// repeated and may be used as a mechanism to change a child's [`WidgetId`],
    /// but this may be expensive.
    ///
    /// This method may be used to configure event handling and to load
    /// resources, including resources affecting [`Layout::size_rules`].
    ///
    /// The window's scale factor (and thus any sizes available through
    /// [`SetRectMgr::size_mgr`]) may not be correct initially (some platforms
    /// construct all windows using scale factor 1) and/or may change in the
    /// future. Changes to the scale factor result in recalculation of
    /// [`Layout::size_rules`] but not repeated configuration.
    fn configure(&mut self, mgr: &mut SetRectMgr) {
        let _ = mgr;
    }

    /// Is this widget navigable via Tab key?
    ///
    /// Defaults to `false`.
    #[inline]
    fn key_nav(&self) -> bool {
        false
    }

    /// Does this widget have hover-state highlighting?
    ///
    /// If true, a redraw will be requested whenever this widget gains or loses
    /// mouse-hover status.
    #[inline]
    fn hover_highlight(&self) -> bool {
        false
    }

    /// Which cursor icon should be used on hover?
    ///
    /// The "hovered" widget is determined by [`Layout::find_id`], thus is the
    /// same widget which would receive click events. Other widgets do not
    /// affect the cursor icon used.
    ///
    /// Defaults to [`event::CursorIcon::Default`].
    #[inline]
    fn cursor_icon(&self) -> event::CursorIcon {
        event::CursorIcon::Default
    }

    /// Get translation of children relative to this widget
    ///
    /// Usually this is zero; only widgets with scrollable or offset content
    /// need implement this. Such widgets must also implement
    /// [`Widget::handle_scroll`].
    ///
    /// Affects event handling via [`Layout::find_id`] and affects the positioning
    /// of pop-up menus. [`Layout::draw`] must be implemented directly using
    /// [`DrawMgr::with_clip_region`] to offset contents.
    #[inline]
    fn translation(&self) -> Offset {
        Offset::ZERO
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
    ///     [`WidgetChildren::get_child`]
    ///
    /// Both `from` and the return value use the widget index, as used by
    /// [`WidgetChildren::get_child`].
    ///
    /// The default implementation often suffices: it will navigate through
    /// children in order.
    #[inline]
    fn spatial_nav(
        &mut self,
        mgr: &mut SetRectMgr,
        reverse: bool,
        from: Option<usize>,
    ) -> Option<usize> {
        let _ = mgr;
        crate::util::spatial_nav(reverse, from, self.num_children())
    }

    /// Handle an event sent to this widget
    ///
    /// An [`Event`] is some form of user input, timer or notification.
    ///
    /// This is the primary event handler for a widget. Secondary handlers are:
    ///
    /// -   If this method returns [`Response::Unused`], then
    ///     [`Widget::handle_unused`] is called on each parent until the event
    ///     is used (or the root widget is reached)
    /// -   If a message is left on the stack by [`EventMgr::push_msg`], then
    ///     [`Widget::handle_message`] is called on each parent until the stack is
    ///     empty (failing to empty the stack results in a warning in the log).
    /// -   If any scroll state is set by [`EventMgr::set_scroll`], then
    ///     [`Widget::handle_scroll`] is called for each parent
    ///
    /// Default implementation: do nothing; return [`Response::Unused`].
    #[inline]
    fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        let _ = (mgr, event);
        Response::Unused
    }

    /// Handle an event sent to child `index` but left unhandled
    ///
    /// Default implementation: call [`Self::handle_event`] with `event`.
    #[inline]
    fn handle_unused(&mut self, mgr: &mut EventMgr, index: usize, event: Event) -> Response {
        let _ = index;
        self.handle_event(mgr, event)
    }

    /// Handler for messages from children/descendants
    ///
    /// This method is called when a child leaves a message on the stack. *Some*
    /// parent or ancestor widget should read this message.
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
        let _ = (mgr, index);
    }

    /// Handler for scrolling
    ///
    /// When a child calls [`EventMgr::set_scroll`] with a value other than
    /// [`Scroll::None`], this method is called. (This method is not called
    /// after [`Self::handle_event`] or other handlers called on self.)
    ///
    /// Note that [`Scroll::Rect`] values are in the child's coordinate space,
    /// and must be translated to the widget's own coordinate space by this
    /// method (this is not done by the default implementation since any widget
    /// with non-zero translation very likely wants to implement this method
    /// anyway).
    ///
    /// If the child is in an independent coordinate space, then this method
    /// should call `mgr.set_scroll(Scroll::None)` to avoid any reactions to
    /// child's scroll requests.
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) {
        let _ = (mgr, scroll);
    }
}

/// Extension trait over widgets
pub trait WidgetExt: Widget {
    /// Get the widget's identifier
    ///
    /// Note that the default-constructed [`WidgetId`] is *invalid*: any
    /// operations on this value will cause a panic. Valid identifiers are
    /// assigned by [`Widget::pre_configure`].
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
        IdentifyWidget(self.widget_name(), self.id())
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

    /// Find the descendant with this `id`, if any
    fn find_widget(&self, id: &WidgetId) -> Option<&dyn Widget> {
        if let Some(index) = self.find_child_index(id) {
            self.get_child(index)
                .and_then(|child| child.find_widget(id))
        } else if self.eq_id(id) {
            return Some(self.as_widget());
        } else {
            None
        }
    }

    /// Find the descendant with this `id`, if any
    fn find_widget_mut(&mut self, id: &WidgetId) -> Option<&mut dyn Widget> {
        if let Some(index) = self.find_child_index(id) {
            self.get_child_mut(index)
                .and_then(|child| child.find_widget_mut(id))
        } else if self.eq_id(id) {
            return Some(self.as_widget_mut());
        } else {
            None
        }
    }
}
impl<W: Widget + ?Sized> WidgetExt for W {}
