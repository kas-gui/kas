// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::any::Any;
use std::fmt;

use super::Boxed;
use crate::draw::{DrawHandle, InputState, SizeHandle};
use crate::event::{self, Manager, ManagerState};
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
/// This trait is almost always implemented via the
/// [`derive(Widget)` macro](macros/index.html#the-derivewidget-macro).
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

    /// Get the disabled state of a widget
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
/// Usually this is implemented by `derive(Widget)`, but for dynamic widgets it
/// may have to be implemented manually. Note that if the results of these
/// methods ever change, one must send [`TkAction::Reconfigure`].
/// TODO: full reconfigure may be too slow; find a better option.
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
/// This trait allows some configuration of widget behaviour. All items have
/// default values. This trait may be implemented by hand, or may be derived
/// with the [`derive(Widget)` macro](macros/index.html#the-derivewidget-macro)
/// by use of a `#[widget_config]` attribute. Optionally, this attribute can
/// contain parameters, e.g. `#[widget_config(key_nav = true)]`.
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
    /// The default implementation of this method does nothing.
    fn configure(&mut self, _: &mut Manager) {}

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
/// This trait contains methods concerned with positioning of contents
/// as well as low-level event handling.
///
/// For a description of the widget size model, see [`SizeRules`].
pub trait Layout: WidgetChildren {
    /// Get size rules for the given axis.
    ///
    /// This method takes `&mut self` to allow local caching of child widget
    /// configuration for future `size_rules` and `set_rect` calls.
    ///
    /// Optionally, this method may set `self.rect().size` to the widget's ideal
    /// size for use by [`Layout::set_rect`] when setting alignment.
    ///
    /// If operating on one axis and the other is fixed, then the `other`
    /// parameter is used for the fixed dimension. Additionally, one may assume
    /// that `size_rules` has previously been called on the fixed axis with the
    /// current widget configuration.
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Adjust to the given size.
    ///
    /// For widgets with children, this is usually implemented via the derive
    /// [macro](kas::macros). For non-parent widgets which stretch to fill
    /// available space, the default implementation suffices. For non-parent
    /// widgets which react to alignment, this is a little more complex to
    /// implement, and can be done in one of two ways:
    ///
    /// 1.  Shrinking to ideal area and aligning within available space (e.g.
    ///     `CheckBoxBare` widget)
    /// 2.  Filling available space and applying alignment to contents (e.g.
    ///     `Label` widget)
    ///
    /// One may assume that `size_rules` has been called for each axis with the
    /// current widget configuration.
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
    /// Disabled widgets should return an empty range, otherwise they should
    /// return a range over children in spatial order (left-to-right then
    /// top-to-bottom). Widgets outside the parent's rect (i.e. popups) should
    /// be excluded.
    ///
    /// The default implementation returns
    /// `(0, WidgetChildren::len(self).wrapping_sub(1))` when not disabled
    /// which should suffice for most widgets.
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

    /// Draw a widget
    ///
    /// This method is called to draw each visible widget (and should not
    /// attempt recursion on child widgets).
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool);
}

/// Widget trait
///
/// This is one of a family of widget traits, all of which must be implemented
/// for a functional widget. In general, most traits will be implemented via the
/// [`derive(Widget)` macro](macros/index.html#the-derivewidget-macro).
///
/// A [`Widget`] may be passed into a generic function via
/// `fn foo<W: Widget>(w: &mut W)` or via
/// `fn foo<M>(w: &mut dyn Widget<Msg = M>)`, or, e.g.
/// `fn foo(w: &mut dyn WidgetConfig)` (note that `WidgetConfig` is the last unparameterised
/// trait in the widget trait family).
pub trait Widget: event::SendEvent {}

impl<W: Widget + Sized> Boxed<dyn Widget<Msg = W::Msg>> for W {
    fn boxed(self) -> Box<dyn Widget<Msg = W::Msg>> {
        Box::new(self)
    }
}
