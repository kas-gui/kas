// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::any::Any;
use std::fmt;

use crate::event;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{self, AlignHints, AxisInfo, SetRectMgr, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::util::IdentifyWidget;
use crate::{CoreData, WidgetId};
use kas_macros::autoimpl;

#[allow(unused)]
use crate::{event::EventState, TkAction};

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
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
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

    /// Get the name of the widget struct
    fn widget_name(&self) -> &'static str;

    /// Erase type
    fn as_widget(&self) -> &dyn WidgetConfig;
    /// Erase type
    fn as_widget_mut(&mut self) -> &mut dyn WidgetConfig;
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
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
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
    fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig>;

    /// Mutable variant of get
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`EventState::send_action`].
    /// This method may be removed in the future.
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig>;

    /// Get the [`WidgetId`] for this child
    ///
    /// Note: the result should be generated relative to `self.id`.
    /// Most widgets may use the default implementation.
    ///
    /// This must return `Some(..)` when `index` is valid; in other cases the
    /// result does not matter.
    ///
    /// If a custom implementation is used, then [`Self::find_child_index`]
    /// must be implemented to do the inverse of `make_child_id`, and
    /// probably a custom implementation of [`Layout::spatial_nav`] is needed.
    #[inline]
    fn make_child_id(&self, index: usize) -> Option<WidgetId> {
        Some(self.id_ref().make_child(index))
    }

    /// Find the child which is an ancestor of this `id`, if any
    ///
    /// If `Some(index)` is returned, this is *probably* but not guaranteed
    /// to be a valid child index.
    #[inline]
    fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
        id.next_key_after(self.id_ref())
    }
}

/// Widget configuration
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless explicitly implemented.
///
/// Widgets are *configured* on window creation or dynamically via the
/// parent calling [`SetRectMgr::configure`]. Configuration may be repeated.
///
/// Configuration is required to happen before [`Layout::size_rules`] is first
/// called, thus [`Self::configure`] may be used to load assets required by
/// [`Layout::size_rules`].
///
/// Configuration happens in this order:
///
/// 1.  [`Self::pre_configure`] is used to assign a [`WidgetId`] and may
///     influence configuration of children.
/// 2.  Configuration of children accessible through [`WidgetChildren`].
/// 3.  [`Self::configure`] allows user-configuration of event handling and
///     may be used for resource loading.
///
/// Note that if parent widgets add child widgets dynamically, that parent is
/// responsible for ensuring that the new child widgets gets configured. This
/// may be done (1) by adding the children during [`Self::pre_configure`], (2)
/// by requesting [`TkAction::RECONFIGURE`] or (3) by calling
/// [`SetRectMgr::configure`]. The latter may also be used to change a child's
/// [`WidgetId`].
///
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
//
// TODO(specialization): provide a blanket implementation, so that users only
// need implement manually when they have something to configure.
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait WidgetConfig: Layout {
    /// Configure widget and children
    ///
    /// This method:
    ///
    /// 1.  Assigns `id` to self
    /// 2.  Constructs an identifier for and call `configure_recurse` on each child
    /// 3.  Calls [`Self::configure`]
    ///
    /// Normally the default implementation is used. A custom implementation
    /// may be used to influence configuration of children, for example by
    /// calling [`EventState::new_accel_layer`] or by constructing children's
    /// [`WidgetId`] values in a non-standard manner.
    ///
    /// To directly configure a child, call [`SetRectMgr::configure`] instead.
    fn configure_recurse(&mut self, mgr: &mut SetRectMgr, id: WidgetId) {
        self.core_data_mut().id = id;

        for index in 0..self.num_children() {
            let id = self.id_ref().make_child(index);
            if let Some(widget) = self.get_child_mut(index) {
                widget.configure_recurse(mgr, id);
            }
        }

        self.configure(mgr);
    }

    /// Configure widget
    ///
    /// This method is part of configuration (see trait documentation).
    ///
    /// This method may be used to configure event handling and to load
    /// resources.
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
}

/// Positioning and drawing routines for widgets
///
/// This trait is part of the [`Widget`] family. It may be derived by
/// the [`crate::macros::widget`] macro, but is not by default.
///
/// Implementations of this trait should *either* define [`Self::layout`]
/// (optionally with other methods as required) *or* define at least
/// [`Self::size_rules`] and [`Self::draw`].
///
/// Layout solving happens in two steps:
///
/// 1.  [`Self::size_rules`] calculates size requirements recursively
/// 2.  [`Self::set_rect`] applies the result recursively
///
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Layout: WidgetChildren {
    /// Describe layout
    ///
    /// This is purely a helper method used to implement other methods:
    /// [`Self::size_rules`], [`Self::set_rect`], [`Self::find_id`], [`Self::draw`].
    /// If those methods are implemented directly (or their default
    /// implementation over the default "empty" layout provided by this method
    /// suffices), then this method need not be implemented.
    ///
    /// The default implementation is for an empty layout (zero size required,
    /// no child elements, no graphics).
    fn layout(&mut self) -> layout::Layout<'_> {
        Default::default() // TODO: remove default impl
    }

    /// Get size rules for the given axis
    ///
    /// For a description of the widget size model, see [`SizeRules`].
    ///
    /// Typically, this method is called twice: first for the horizontal axis,
    /// second for the vertical axis (with resolved width available through
    /// the `axis` parameter allowing content wrapping). On re-sizing, the
    /// first or both method calls may be skipped.
    ///
    /// This method takes `&mut self` since it may be necessary to store child
    /// element size rules in order to calculate layout by `size_rules` on the
    /// second axis and by `set_rect`.
    ///
    /// This method may be implemented through [`Self::layout`] or directly.
    /// A [`crate::layout::RulesSolver`] engine may be useful to calculate
    /// requirements of complex layouts.
    ///
    /// [`WidgetConfig::configure`] will be called before this method and may
    /// be used to load assets.
    fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
        self.layout().size_rules(size_mgr, axis)
    }

    /// Apply a given `rect` to self
    ///
    /// This method applies the layout resolved by [`Self::size_rules`].
    ///
    /// This method may be implemented through [`Self::layout`] or directly.
    /// For widgets without children, typically this method only stores the
    /// calculated `rect`, which is done by the default implementation (even
    /// with the default empty layout for [`Self::layout`]).
    ///
    /// This method may also be useful for alignment, which may be applied in
    /// one of two ways:
    ///
    /// 1.  Shrinking `rect` to the "ideal size" and aligning within (see
    ///     [`crate::layout::CompleteAlignment::aligned_rect`] or example usage in
    ///     `CheckBoxBare` widget)
    /// 2.  Applying alignment to contents (see for example `Label` widget)
    ///
    /// One may assume that `size_rules` has been called at least once for each
    /// axis with current size information before this method.
    fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
        self.core_data_mut().rect = rect;
        self.layout().set_rect(mgr, rect, align);
    }

    /// Get translation of children relative to this widget
    ///
    /// Usually this is zero; only widgets with scrollable or offset content
    /// need implement this.
    ///
    /// Affects event handling via [`Self::find_id`] and affects the positioning
    /// of pop-up menus. [`Self::draw`] must be implemented directly using
    /// [`DrawMgr::with_clip_region`] to offset contents.
    #[inline]
    fn translation(&self) -> Offset {
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
        mgr: &mut SetRectMgr,
        reverse: bool,
        from: Option<usize>,
    ) -> Option<usize> {
        let _ = mgr;
        let last = self.num_children().wrapping_sub(1);
        if last == usize::MAX {
            return None;
        }

        let reverse = reverse ^ self.layout().is_reversed();

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

    /// Translate a coordinate to a [`WidgetId`]
    ///
    /// This method is used in event handling, translating a mouse click or
    /// touch input to a widget and resolving a [`WidgetConfig::cursor_icon`].
    /// Usually, this is the widget which draws the target coordinate, but
    /// stealing focus is permitted: e.g. the `Button` widget handles clicks on
    /// inner content, while the `CheckBox` widget forwards click events to its
    /// `CheckBoxBare` component.
    ///
    /// The default implementation suffices unless:
    ///
    /// -   [`Self::layout`] is not implemented and there are child widgets
    /// -   Event stealing from child widgets is desired (but note that
    ///     [`crate::layout::Layout::button`] does this already)
    /// -   The child widget is in a translated coordinate space *not equal* to
    ///     [`Self::translation`]
    ///
    /// To implement directly:
    ///
    /// -   Return `None` if `coord` is not within `self.rect()`
    /// -   Find the child which should respond to input at `coord`, if any, and
    ///     call `find_id` recursively on this child
    /// -   Otherwise return `self.id()`
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        let coord = coord + self.translation();
        self.layout().find_id(coord).or_else(|| Some(self.id()))
    }

    /// Draw a widget and its children
    ///
    /// This method is invoked each frame to draw visible widgets. It should
    /// draw itself and recurse into all visible children.
    ///
    /// The default impl draws elements as defined by [`Self::layout`].
    fn draw(&mut self, draw: DrawMgr) {
        let id = self.id(); // clone to avoid borrow conflict
        self.layout().draw(draw, &id);
    }
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
/// [`derive(Widget)`]: https://docs.rs/kas/latest/kas/macros/index.html#the-derivewidget-macro
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Widget: event::SendEvent {}

/// Extension trait over widgets
pub trait WidgetExt: WidgetChildren {
    /// Get the widget's numeric identifier
    ///
    /// Note that the default-constructed [`WidgetId`] is *invalid*: any
    /// operations on this value will cause a panic. Valid identifiers are
    /// assigned by [`WidgetConfig::pre_configure`].
    #[inline]
    fn id(&self) -> WidgetId {
        self.core_data().id.clone()
    }

    /// Get the widget's numeric identifier
    ///
    /// Note that the default-constructed [`WidgetId`] is *invalid*: any
    /// operations on this value will cause a panic. Valid identifiers are
    /// assigned by [`WidgetConfig::pre_configure`].
    #[inline]
    fn id_ref(&self) -> &WidgetId {
        &self.core_data().id
    }

    /// Get the `u64` version of the widget identifier
    ///
    /// This may be used to approximately test identity (see notes on
    /// [`WidgetId::as_u64`]).
    #[inline]
    fn id_u64(&self) -> u64 {
        self.core_data().id.as_u64()
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
        self.core_data().id == rhs
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
    fn find_widget(&self, id: &WidgetId) -> Option<&dyn WidgetConfig> {
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
    fn find_widget_mut(&mut self, id: &WidgetId) -> Option<&mut dyn WidgetConfig> {
        if let Some(index) = self.find_child_index(id) {
            self.get_child_mut(index)
                .and_then(|child| child.find_widget_mut(id))
        } else if self.eq_id(id) {
            return Some(self.as_widget_mut());
        } else {
            None
        }
    }

    /// Get the widget's region, relative to its parent.
    #[inline]
    fn rect(&self) -> Rect {
        self.core_data().rect
    }
}
impl<W: WidgetChildren + ?Sized> WidgetExt for W {}
