// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::any::Any;
use std::fmt;

use super::Boxed;
use crate::draw::{DrawHandle, InputState, SizeHandle};
use crate::event::{self, ConfigureManager, Manager, ManagerState};
use crate::geom::{Coord, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::{AlignHints, CoreData, TkAction, WidgetId};

impl dyn WidgetCore {
    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn is<T: Any>(&self) -> bool {
        Any::is::<T>(self.as_any())
    }

    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        Any::downcast_ref::<T>(self.as_any())
    }

    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        Any::downcast_mut::<T>(self.as_any_mut())
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
        TkAction::Redraw
    }

    /// Set disabled state (chaining)
    ///
    /// This is identical to [`WidgetCore::set_disabled`], but can be called in
    /// chaining fashion. Example:
    /// ```
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
    fn input_state(&self, mgr: &ManagerState, disabled: bool) -> InputState {
        let id = self.core_data().id;
        InputState {
            disabled: self.core_data().disabled || disabled,
            error: false,
            hover: mgr.is_hovered(id),
            depress: mgr.is_depressed(id),
            nav_focus: mgr.nav_focus(id),
            char_focus: mgr.char_focus(id),
        }
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
/// one must send [`TkAction::Reconfigure`].
/// (TODO: this is slow. Find an option for partial reconfigures. This requires
/// better widget identifiers; see #91.)
///
/// [`derive(Widget)`]: macros/index.html#the-derivewidget-macro
pub trait WidgetChildren: WidgetCore {
    /// Get the number of child widgets
    fn len(&self) -> usize;

    /// Get a reference to a child widget by index, or `None` if the index is
    /// out of bounds.
    ///
    /// For convenience, `Index<usize>` is implemented via this method.
    ///
    /// Required: `index < self.len()`.
    fn get(&self, index: usize) -> Option<&dyn WidgetConfig>;

    /// Mutable variant of get
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`Manager::send_action`].
    /// This method may be removed in the future.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig>;

    /// Check whether `id` is a descendant
    ///
    /// This function assumes that `id` is a valid widget.
    #[inline]
    fn is_ancestor_of(&self, id: WidgetId) -> bool {
        self.find(id).is_some()
    }

    /// Find a child widget by identifier
    ///
    /// This requires that the widget tree has already been configured by
    /// [`event::ManagerState::configure`].
    ///
    /// If the widget is disabled, this returns `None` without recursing children.
    fn find(&self, id: WidgetId) -> Option<&dyn WidgetConfig> {
        if id == self.id() {
            return Some(self.as_widget());
        } else if id > self.id() {
            return None;
        }

        for i in 0..self.len() {
            if let Some(w) = self.get(i) {
                if id > w.id() {
                    continue;
                }
                return w.find(id);
            }
            break;
        }
        None
    }

    /// Find a child widget by identifier
    ///
    /// This requires that the widget tree has already been configured by
    /// [`ManagerState::configure`].
    fn find_mut(&mut self, id: WidgetId) -> Option<&mut dyn WidgetConfig> {
        if id == self.id() {
            return Some(self.as_widget_mut());
        } else if id > self.id() {
            return None;
        }

        for i in 0..self.len() {
            if self.get(i).map(|w| id > w.id()).unwrap_or(true) {
                continue;
            }
            if let Some(w) = self.get_mut(i) {
                return w.find_mut(id);
            }
            break;
        }
        None
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk(&self, f: &mut dyn FnMut(&dyn WidgetConfig)) {
        for i in 0..self.len() {
            if let Some(w) = self.get(i) {
                w.walk(f);
            }
        }
        f(self.as_widget());
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn WidgetConfig)) {
        for i in 0..self.len() {
            if let Some(w) = self.get_mut(i) {
                w.walk_mut(f);
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
    /// [`TkAction::Reconfigure`] is sent.
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
    fn configure_recurse<'a, 'b>(&mut self, mut cmgr: ConfigureManager<'a, 'b>) {
        for i in 0..self.len() {
            if let Some(w) = self.get_mut(i) {
                w.configure_recurse(cmgr.child());
            }
        }
        self.core_data_mut().id = cmgr.next_id(self.id());
        self.configure(cmgr.mgr());
    }

    /// Is this widget navigable via Tab key?
    ///
    /// Defaults to `false`.
    fn key_nav(&self) -> bool {
        false
    }

    /// Which cursor icon should be used on hover?
    ///
    /// Defaults to [`event::CursorIcon::Default`].
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
/// [`kas::macros`]); otherwise, a layout engine may be used (see
/// [`kas::layout`]). For leaf widgets, it is implemented directly.
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
    /// For widgets with children, a [`kas::layout::RulesSolver`] engine may be
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
    /// For widgets with children, a [`kas::layout::RulesSetter`] engine may be
    /// useful (used with a corresponding [`kas::layout::RulesSolver`]).
    ///
    /// One may assume that `size_rules` has been called at least once for each
    /// axis with current size information before this method, however
    /// `size_rules` might not be re-called before calling `set_rect` again.
    #[inline]
    fn set_rect(&mut self, rect: Rect, _align: AlignHints) {
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
    /// [`Coord::ZERO`] for non-existant children.
    #[inline]
    fn translation(&self, _child_index: usize) -> Coord {
        Coord::ZERO
    }

    /// Iterate through children in spatial order
    ///
    /// Returns a "range" of children, by index, in spatial order. Unlike
    /// `std::ops::Range` this is inclusive and reversible, e.g. `(1, 3)` means
    /// `1, 2, 3` and `(5, 2)` means `5, 4, 3, 2`. As a special case,
    /// `(_, std::usize::MAX)` means the range is empty.
    ///
    /// Widgets should return a range over children in spatial order
    /// (left-to-right then top-to-bottom). Widgets outside the parent's rect
    /// (i.e. popups) should be excluded.
    ///
    /// The default implementation should suffice for most widgets (excluding
    /// pop-up parents and those with reversed child order).
    fn spatial_range(&self) -> (usize, usize) {
        (0, WidgetChildren::len(self).wrapping_sub(1))
    }

    /// Find a widget by coordinate
    ///
    /// Returns the identifier of the widget containing this `coord`, if any.
    /// Should only return `None` when `coord` is outside the widget's rect,
    /// but this is not guaranteed.
    ///
    /// Implementations should:
    ///
    /// 1.  return `None` if `!self.rect().contains(coord)`
    /// 2.  if, for any child (containing `coord`), `child.find_id(coord)`
    ///     returns `Some(id)`, return that
    /// 3.  otherwise, return `Some(self.id())`
    ///
    /// Exceptionally, a widget may deviate from this behaviour, but only when
    /// the coord is within the widget's rect (example: `CheckBox` contains an
    /// embedded `CheckBoxBare` and always forwards this child's id).
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

impl<W: Widget + Sized> Boxed<dyn Widget<Msg = W::Msg>> for W {
    fn boxed(self) -> Box<dyn Widget<Msg = W::Msg>> {
        Box::new(self)
    }
}
