// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::any::Any;
use std::fmt;

use crate::draw::{DrawHandle, InputState, SizeHandle};
use crate::event::{self, ConfigureManager, Manager, ManagerState};
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::{CoreData, TkAction, WidgetId};

impl dyn WidgetCore {
    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn is<T: Any>(&self) -> bool {
        <dyn Any>::is::<T>(self.as_any())
    }

    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        <dyn Any>::downcast_ref::<T>(self.as_any())
    }

    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        <dyn Any>::downcast_mut::<T>(self.as_any_mut())
    }
}

/// Base widget functionality
///
/// See the [`Widget`] trait for documentation of the widget family.
///
/// This trait **must** be implement by the [`derive(Widget)`] macro.
/// Users **must not** implement this `WidgetCore` trait manually or may face
/// unexpected breaking changes.
///
/// [`derive(Widget)`]: macros/index.html#the-derivewidget-macro
pub trait WidgetCore: Any + fmt::Debug {
    /// Get self as type `Any`
    fn as_any(&self) -> &dyn Any;

    /// Get self as type `Any` (mutable)
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get direct access to the [`CoreData`] providing property storage.
    fn core_data(&self) -> &CoreData;

    /// Get mutable access to the [`CoreData`] providing property storage.
    ///
    /// This should not normally be needed by user code.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn core_data_mut(&mut self) -> &mut CoreData;

    /// Get the widget's numeric identifier
    #[inline]
    fn id(&self) -> WidgetId {
        self.core_data().id
    }

    /// Get whether the widget is disabled
    #[inline]
    fn is_disabled(&self) -> bool {
        self.core_data().disabled
    }

    /// Set the disabled state of a widget
    ///
    /// If disabled, a widget should not respond to input and should appear
    /// greyed out.
    ///
    /// The disabled status is inherited by children: events should not be
    /// passed to them, and they should also be drawn greyed out.
    #[inline]
    fn set_disabled(&mut self, disabled: bool) -> TkAction {
        self.core_data_mut().disabled = disabled;
        TkAction::REDRAW
    }

    /// Set disabled state (chaining)
    ///
    /// This is identical to [`WidgetCore::set_disabled`], but can be called in
    /// chaining fashion. Example:
    /// ```ignore
    /// use kas::{WidgetCore, widget::MenuEntry};
    /// let entry = MenuEntry::new("Disabled Item", ()).with_disabled(true);
    /// ```
    #[inline]
    fn with_disabled(mut self, disabled: bool) -> Self
    where
        Self: Sized,
    {
        self.core_data_mut().disabled = disabled;
        self
    }

    /// Get the widget's region, relative to its parent.
    #[inline]
    fn rect(&self) -> Rect {
        self.core_data().rect
    }

    /// Get the name of the widget struct
    fn widget_name(&self) -> &'static str;

    /// Erase type
    fn as_widget(&self) -> &dyn WidgetConfig;
    /// Erase type
    fn as_widget_mut(&mut self) -> &mut dyn WidgetConfig;

    /// Construct [`InputState`]
    ///
    /// The `disabled` flag is inherited from parents. [`InputState::disabled`]
    /// will be true if either `disabled` or `self.is_disabled()` are true.
    ///
    /// The error state defaults to `false` since most widgets don't support
    /// this.
    ///
    /// Note: most state changes should automatically cause a redraw, but change
    /// in `hover` status will not (since this happens frequently and many
    /// widgets are unaffected), unless [`WidgetConfig::hover_highlight`]
    /// returns true.
    fn input_state(&self, mgr: &ManagerState, disabled: bool) -> InputState {
        let id = self.core_data().id;
        let (char_focus, sel_focus) = mgr.has_char_focus(id);
        let mut state = InputState::empty();
        if self.core_data().disabled || disabled {
            state |= InputState::DISABLED;
        }
        if mgr.is_hovered(id) {
            state |= InputState::HOVER;
        }
        if mgr.is_depressed(id) {
            state |= InputState::DEPRESS;
        }
        if mgr.nav_focus(id) {
            state |= InputState::NAV_FOCUS;
        }
        if char_focus {
            state |= InputState::CHAR_FOCUS;
        }
        if sel_focus {
            state |= InputState::SEL_FOCUS;
        }
        state
    }
}

/// Listing of a widget's children
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless `#[widget(children = noauto)]` is used.
///
/// Dynamic widgets must implement this trait manually, since [`derive(Widget)`]
/// cannot currently handle fields like `Vec<SomeWidget>`.
///
/// Whenever the number of child widgets changes or child widgets are replaced,
/// one must send [`TkAction::RECONFIGURE`].
/// (TODO: this is slow. Find an option for partial reconfigures. This requires
/// better widget identifiers; see #91.)
///
/// [`derive(Widget)`]: macros/index.html#the-derivewidget-macro
pub trait WidgetChildren: WidgetCore {
    /// Get the first identifier of self or any children
    ///
    /// Widget identifiers are assigned sequentially by depth-first-search,
    /// children before parents. Any widget thus has a range of identifiers,
    /// from the first assigned to any descendent (or self) to its own
    /// ([`WidgetCore::id`]). This method must return the first identifier.
    fn first_id(&self) -> WidgetId;

    /// Record first identifier
    ///
    /// This is called during [`WidgetConfig::configure_recurse`] with the first
    /// identifier. This may be used to implement [`WidgetChildren::first_id`],
    /// although in many cases the first identifier can be read directly from
    /// the first child. This method has a default implementation doing nothing.
    ///
    /// This method should only be called from `configure_recurse`.
    #[inline]
    fn record_first_id(&mut self, _id: WidgetId) {}

    /// Get the number of child widgets
    fn num_children(&self) -> usize;

    /// Get a reference to a child widget by index, or `None` if the index is
    /// out of bounds.
    ///
    /// For convenience, `Index<usize>` is implemented via this method.
    ///
    /// Required: `index < self.len()`.
    fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig>;

    /// Mutable variant of get
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`Manager::send_action`].
    /// This method may be removed in the future.
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig>;

    /// Check whether `id` is a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    fn is_ancestor_of(&self, id: WidgetId) -> bool {
        id <= self.id() && self.first_id() <= id
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// This child may then be accessed via [`Self::get_child`] or
    /// [`Self::get_child_mut`].
    ///
    /// This requires that the widget tree has already been configured by
    /// [`event::ManagerState::configure`].
    fn find_child(&self, id: WidgetId) -> Option<usize> {
        if id < self.first_id() || id >= self.id() {
            return None;
        }

        let (mut start, mut end) = (0, self.num_children());
        while start + 1 < end {
            let mid = start + (end - start) / 2;
            if id <= self.get_child(mid - 1).unwrap().id() {
                end = mid;
            } else {
                start = mid;
            }
        }
        Some(start)
    }

    /// Find the leaf (lowest descendant) with this `id`, if any
    ///
    /// This requires that the widget tree has already been configured by
    /// [`event::ManagerState::configure`].
    fn find_leaf(&self, id: WidgetId) -> Option<&dyn WidgetConfig> {
        if let Some(child) = self.find_child(id) {
            self.get_child(child).unwrap().find_leaf(id)
        } else if id == self.id() {
            return Some(self.as_widget());
        } else {
            None
        }
    }

    /// Find the leaf (lowest descendant) with this `id`, if any
    ///
    /// This requires that the widget tree has already been configured by
    /// [`ManagerState::configure`].
    fn find_leaf_mut(&mut self, id: WidgetId) -> Option<&mut dyn WidgetConfig> {
        if let Some(child) = self.find_child(id) {
            self.get_child_mut(child).unwrap().find_leaf_mut(id)
        } else if id == self.id() {
            return Some(self.as_widget_mut());
        } else {
            None
        }
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk_children<F: FnMut(&dyn WidgetConfig)>(&self, mut f: F)
    where
        Self: Sized,
    {
        self.walk_children_dyn(&mut f)
    }

    #[doc(hidden)]
    fn walk_children_dyn(&self, f: &mut dyn FnMut(&dyn WidgetConfig)) {
        for i in 0..self.num_children() {
            if let Some(w) = self.get_child(i) {
                w.walk_children_dyn(f);
            }
        }
        f(self.as_widget());
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk_children_mut<F: FnMut(&mut dyn WidgetConfig)>(&mut self, mut f: F)
    where
        Self: Sized,
    {
        self.walk_children_mut_dyn(&mut f)
    }

    #[doc(hidden)]
    fn walk_children_mut_dyn(&mut self, f: &mut dyn FnMut(&mut dyn WidgetConfig)) {
        for i in 0..self.num_children() {
            if let Some(w) = self.get_child_mut(i) {
                w.walk_children_mut_dyn(f);
            }
        }
        f(self.as_widget_mut());
    }
}

/// Widget configuration
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless `#[widget(config = noauto)]` is used.
/// `key_nav` and `cursor_icon` may be customised without a manual
/// implementation (e.g. `#[widget(config(key_nav = true))]`).
///
/// This trait allows some configuration of widget behaviour. All methods have
/// default implementations. Most frequently, this trait is used to implement
/// some custom action during configure: [`WidgetConfig::configure`].
///
/// [`derive(Widget)`]: macros/index.html#the-derivewidget-macro
//
// TODO(specialization): provide a blanket implementation, so that users only
// need implement manually when they have something to configure.
pub trait WidgetConfig: Layout {
    /// Configure widget
    ///
    /// Widgets are *configured* on window creation and when
    /// [`TkAction::RECONFIGURE`] is sent.
    ///
    /// Configure is called before resizing (but after calculation of the
    /// initial window size). This method is called after
    /// a [`WidgetId`] has been assigned to self, and after `configure` has
    /// been called on each child.
    ///
    /// It is not advised to perform any action requiring a reconfigure (e.g.
    /// adding a child widget) during configure due to the possibility of
    /// getting stuck in a reconfigure-loop. See issue kas#91 for more on this.
    /// KAS has a crude mechanism to detect this and panic.
    ///
    /// The default implementation of this method does nothing.
    fn configure(&mut self, _: &mut Manager) {}

    /// Configure self and children
    ///
    /// In most cases one should not override the default implementation of this
    /// method but instead use [`WidgetConfig::configure`]; the exception is
    /// widgets with pop-ups.
    fn configure_recurse(&mut self, mut cmgr: ConfigureManager) {
        self.record_first_id(cmgr.peek_next());
        for i in 0..self.num_children() {
            if let Some(w) = self.get_child_mut(i) {
                w.configure_recurse(cmgr.child());
            }
        }
        self.core_data_mut().id = cmgr.next_id(self.id());
        self.configure(cmgr.mgr());
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
    /// Defaults to [`event::CursorIcon::Default`].
    #[inline]
    fn cursor_icon(&self) -> event::CursorIcon {
        event::CursorIcon::Default
    }
}

/// Positioning and drawing routines for widgets
///
/// This trait is part of the [`Widget`] family. It may be derived by
/// [`derive(Widget)`], but is not by default.
///
/// This trait contains methods concerned with positioning of contents
/// as well as low-level event handling.
///
/// For parent widgets, the implementation will often be derived (see
/// [`kas::macros`](https://docs.rs/kas/latest/kas/macros/));
/// otherwise, a layout engine may be used (see
/// [`crate::layout`]). For leaf widgets, it is implemented directly.
///
/// For a description of the widget size model, see [`SizeRules`].
///
/// [`derive(Widget)`]: macros/index.html#the-derivewidget-macro
pub trait Layout: WidgetChildren {
    /// Get size rules for the given axis
    ///
    /// This method takes `&mut self` to allow local caching of child widget
    /// configuration for future `size_rules` and `set_rect` calls.
    /// Fields written by `set_rect` should not be used for this cache since
    /// `set_rect` may be called multiple times without re-calling `size_rules`.
    ///
    /// To allow automatic flow of content over new lines, the width is sized
    /// first, followed by the height; when sizing for height, [`AxisInfo`]
    /// contains the size of the *other* axis (i.e. the width).
    ///
    /// For widgets with children, a [`crate::layout::RulesSolver`] engine may be
    /// useful to calculate requirements of complex layouts.
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Apply a given `rect` to self
    ///
    /// For widgets without children, the trivial default implementation of this
    /// method often suffices, though some widgets choose to align themselves
    /// within this space. Alignment may be applied in one of two ways:
    ///
    /// 1.  Shrinking to ideal area and aligning within available space (e.g.
    ///     `CheckBoxBare` widget)
    /// 2.  Filling available space and applying alignment to contents (e.g.
    ///     `Label` widget)
    ///
    /// For widgets with children, a [`crate::layout::RulesSetter`] engine may be
    /// useful (used with a corresponding [`crate::layout::RulesSolver`]).
    ///
    /// One may assume that `size_rules` has been called at least once for each
    /// axis with current size information before this method, however
    /// `size_rules` might not be re-called before calling `set_rect` again.
    #[inline]
    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let _ = (mgr, align);
        self.core_data_mut().rect = rect;
    }

    /// Get translation of a child
    ///
    /// Children may live in a translated coordinate space relative to their
    /// parent. This method returns an offset which should be *added* to a
    /// coordinate to translate *into* the child's coordinate space or
    /// subtracted to translate out.
    ///
    /// In most cases, the translation will be zero. Widgets should return
    /// [`Offset::ZERO`] for non-existant children.
    #[inline]
    fn translation(&self, _child_index: usize) -> Offset {
        Offset::ZERO
    }

    /// Navigation in spatial order
    ///
    /// Returns the index of the "next" child in iteration order within the
    /// widget's rect, if any. (Pop-up widgets should be excluded.)
    ///
    /// If `reverse` is true, move in left/up direction, otherwise right/down.
    /// If `from.is_some()`, return its next sibling in iteration order,
    /// otherwise return the first or last child.
    ///
    /// The default implementation often suffices: it will navigate through
    /// children in order.
    fn spatial_nav(
        &mut self,
        mgr: &mut Manager,
        reverse: bool,
        from: Option<usize>,
    ) -> Option<usize> {
        let _ = mgr;
        let last = self.num_children().wrapping_sub(1);
        if last == usize::MAX {
            return None;
        }

        if let Some(index) = from {
            match reverse {
                false if index < last => Some(index + 1),
                true if 0 < index => Some(index - 1),
                _ => None,
            }
        } else {
            match reverse {
                false => Some(0),
                true => Some(last),
            }
        }
    }

    /// Find a widget by coordinate
    ///
    /// Used to find the widget responsible for handling events at this `coord`
    /// — usually the leaf-most widget containing the coordinate.
    ///
    /// The default implementation suffices for widgets without children;
    /// otherwise this is usually implemented as follows:
    ///
    /// 1.  return `None` if `!self.rect().contains(coord)`
    /// 2.  for each `child`, check whether `child.find_id(coord)` returns
    ///     `Some(id)`, and if so return this result (parents with many children
    ///     might use a faster search strategy here)
    /// 3.  otherwise, return `Some(self.id())`
    ///
    /// Exceptionally, a widget may deviate from this behaviour, but only when
    /// the coord is within the widget's own rect (example: `CheckBox` contains
    /// an embedded `CheckBoxBare` and always forwards this child's id).
    ///
    /// This must not be called before [`Layout::set_rect`].
    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        Some(self.id())
    }

    /// Draw a widget and its children
    ///
    /// This method is invoked each frame to draw visible widgets. It should
    /// draw itself and recurse into all visible children.
    ///
    /// The `disabled` argument is passed in from the *parent*; a widget should
    /// use `let disabled = disabled || self.is_disabled();` to determine its
    /// own disabled state, then pass this value on to children.
    ///
    /// [`WidgetCore::input_state`] may be used to obtain an [`InputState`] to
    /// determine active visual effects.
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool);
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
/// -   [`WidgetConfig`] — the last unparametrised trait allows customisation of
///     some aspects of widget behaviour
/// -   [`event::Handler`] — parametrised widgets over a `Msg` type and handles
///     events
/// -   [`event::SendEvent`] — routes events to children and handles responses
/// -   [`Widget`] — the final trait
///
/// Widgets **must** use the [`derive(Widget)`] macro to implement at least
/// [`WidgetCore`] and [`Widget`]; these two traits **must not** be implemented
/// manually or users may face unexpected breaking changes.
/// This macro can optionally implement *all* above traits, and by default will
/// implement *all except for `Layout`*. This opt-out derive behaviour means
/// that adding additional traits into the family is not a breaking change.
///
/// To refer to a widget via dyn trait, use `&dyn WidgetConfig` (or, if the
/// message type is known, one may use `&dyn Widget<Msg = M>`).
/// To refer to a widget in generic functions, use `<W: Widget>` or
/// `<M, W: Widget<Msg = M>>`.
///
/// [`derive(Widget)`]: macros/index.html#the-derivewidget-macro
pub trait Widget: event::SendEvent {}
