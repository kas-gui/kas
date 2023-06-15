// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use crate::event::{ConfigMgr, Event, EventMgr, Response, Scroll};
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::util::IdentifyWidget;
use crate::{Erased, WidgetId};
use kas_macros::autoimpl;

#[allow(unused)] use crate::event::EventState;
#[allow(unused)]
use crate::layout::{self, AlignPair, AutoLayout};
#[allow(unused)] use crate::Action;
#[allow(unused)] use kas_macros as macros;

/// Base functionality for [`Widget`]s
///
/// # Implementing WidgetCore
///
/// Implementations of this trait are generated via macro.
/// **Directly implementing this trait is not supported**.
/// See [`Widget`] trait documentation.
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait WidgetCore {
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
    fn as_node(&self) -> &dyn Node;
    /// Erase type
    fn as_node_mut(&mut self) -> &mut dyn Node;
}

/// Listing of a [`Widget`]'s children
///
/// This trait enumerates child widgets (that is, components of the widget which
/// are themselves widgets).
///
/// Enumerated widgets are automatically configured, via recursion, when their
/// parent is. See [`Widget::configure`].
///
/// # Implementing WidgetChildren
///
/// Implementations of this trait are usually generated via macro.
/// See [`Widget`] trait documentation.
///
/// In a few cases, namely widgets which may add/remove children dynamically,
/// this trait should be implemented directly.
///
/// Note that parents are responsible for ensuring that newly added children
/// get configured, either by sending [`Action::RECONFIGURE`] by calling
/// [`ConfigMgr::configure`].
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait WidgetChildren: WidgetCore {
    /// Get the number of child widgets
    ///
    /// Every value in the range `0..self.num_children()` is a valid child
    /// index.
    fn num_children(&self) -> usize;

    /// Get a reference to a child widget by index, if any
    ///
    /// Required: `index < self.len()`.
    fn get_child(&self, index: usize) -> Option<&dyn Node>;

    /// Mutable variant of get
    ///
    /// Warning: directly adjusting a widget without requiring reconfigure or
    /// redraw may break the UI. If a widget is replaced, a reconfigure **must**
    /// be requested. This can be done via [`EventState::send_action`].
    /// This method may be removed in the future.
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Node>;

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
}

/// Positioning and drawing routines for [`Widget`]s
///
/// This trait is related to [`Widget`], but may be used independently.
///
/// # Implementing Layout
///
/// The [`#[widget]` macro](macros::widget) supports an optional property,
/// `layout`. If this is used then the `Layout` trait is implemented
/// automatically (although a custom implementation may still be used, which
/// may refer to the implementation of [`AutoLayout`] for `Self`).
///
/// If the `layout` property is not used then at least [`Self::size_rules`] and
/// [`Self::draw`] must be defined directly.
///
/// # Solving layout
///
/// Layout is resolved as follows:
///
/// 1.  [`Widget::configure`] is called (widgets only), and may be used to load assets
/// 2.  [`Self::size_rules`] is called at least once for each axis
/// 3.  [`Self::set_rect`] is called to position elements. This may use data cached by `size_rules`.
/// 4.  [`Self::find_id`] may be used to find the widget under the mouse and [`Self::draw`] to draw
///     elements.
///
/// Usually, [`Layout::size_rules`] methods are called recursively. To instead
/// solve layout for a single widget/layout object, it may be useful to use
/// [`layout::solve_size_rules`] or [`layout::SolveCache`].
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Layout: WidgetChildren {
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
    /// regardless of the [`Stretch`] policy used. If the widget should never
    /// stretch, it must align itself.
    /// Example: the `CheckBox` widget uses an [`AlignPair`] (set from
    /// `size_rules`'s [`AxisInfo`]) and uses [`ConfigMgr::align_feature`].
    /// Another example: `Label` uses a `Text` object which handles alignment
    /// internally.
    ///
    /// Default implementation when not using the `layout` property: set `rect`
    /// field of `widget_core!()` to the input `rect`.
    ///
    /// [`Stretch`]: crate::layout::Stretch
    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect);

    /// Get translation of children relative to this widget
    ///
    /// Usually this is zero; only widgets with scrollable or offset content
    /// *and* child widgets need to implement this.
    /// Such widgets must also implement [`Widget::handle_scroll`].
    ///
    /// Affects event handling via [`Layout::find_id`] and affects the positioning
    /// of pop-up menus. [`Layout::draw`] must be implemented directly using
    /// [`DrawMgr::with_clip_region`] to offset contents.
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
    /// [`Widget::handle_event`] to implement touch scrolling).
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
    /// supplied via [`DrawMgr::re_id`] or [`DrawMgr::recurse`].
    fn draw(&mut self, draw: DrawMgr);
}

/// The Widget trait
///
/// Widgets implement a family of traits, of which this trait is the final
/// member:
///
/// -   [`WidgetCore`] — base functionality
/// -   [`WidgetChildren`] — enumerates children
/// -   [`Layout`] — handles sizing and positioning for self and children
/// -   [`Widget`] — configuration, event handling
///
/// # Implementing Widget
///
/// To implement a widget, use the [`macros::widget`] macro. **This is the
/// only supported method of implementing `Widget`.**
///
/// The [`macros::widget`] macro only works within [`macros::impl_scope`].
/// Other trait implementations can be detected within this scope:
///
/// -   [`WidgetCore`] is always generated
/// -   [`WidgetChildren`] is generated if no direct implementation is present
/// -   [`Layout`] is generated if the `layout` attribute property is set, and
///     no direct implementation is found. In other cases where a direct
///     implementation of the trait is found, (default) method implementations
///     may be injected where not already present.
/// -   [`Widget`] is generated if no direct implementation is present,
///     otherwise some (default) method implementations are injected where
///     these methods are not directly implemented.
///
/// Some simple examples follow. See also
/// [examples apps](https://github.com/kas-gui/kas/tree/master/examples)
/// and [`kas_widgets` code](https://github.com/kas-gui/kas/tree/master/crates/kas-widgets).
/// ```
/// # extern crate kas_core as kas;
/// use kas::event;
/// use kas::prelude::*;
/// use kas::theme::TextClass;
/// use std::fmt::Debug;
///
/// impl_scope! {
///     /// A text label
///     #[widget]
///     pub struct AccelLabel {
///         core: widget_core!(),
///         class: TextClass,
///         label: Text<AccelString>,
///     }
///
///     impl Self {
///         /// Construct from `label`
///         pub fn new(label: impl Into<AccelString>) -> Self {
///             AccelLabel {
///                 core: Default::default(),
///                 class: TextClass::AccelLabel(true),
///                 label: Text::new(label.into()),
///             }
///         }
///
///         /// Set text class (inline)
///         pub fn with_class(mut self, class: TextClass) -> Self {
///             self.class = class;
///             self
///         }
///
///         /// Get the accelerator keys
///         pub fn keys(&self) -> &[event::VirtualKeyCode] {
///             self.label.text().keys()
///         }
///     }
///
///     impl Layout for Self {
///         fn size_rules(&mut self, size_mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
///             axis.set_default_align_hv(Align::Default, Align::Center);
///             size_mgr.text_rules(&mut self.label, self.class, axis)
///         }
///
///         fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
///             self.core.rect = rect;
///             mgr.text_set_size(&mut self.label, self.class, rect.size, None);
///         }
///
///         fn draw(&mut self, mut draw: DrawMgr) {
///             draw.text_effects(self.rect(), &self.label, self.class);
///         }
///     }
/// }
///
/// impl_scope! {
///     /// A push-button with a text label
///     #[widget {
///         layout = button!(self.label);
///         navigable = true;
///         hover_highlight = true;
///     }]
///     pub struct TextButton<M: Clone + Debug + 'static> {
///         core: widget_core!(),
///         #[widget]
///         label: AccelLabel,
///         message: M,
///     }
///
///     impl Self {
///         /// Construct a button with given `label`
///         pub fn new(label: impl Into<AccelString>, message: M) -> Self {
///             TextButton {
///                 core: Default::default(),
///                 label: AccelLabel::new(label).with_class(TextClass::Button),
///                 message,
///             }
///         }
///     }
///     impl Widget for Self {
///         fn configure(&mut self, mgr: &mut ConfigMgr) {
///             mgr.add_accel_keys(self.id_ref(), self.label.keys());
///         }
///
///         fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
///             event.on_activate(mgr, self.id(), |mgr| {
///                 mgr.push(self.message.clone());
///                 Response::Used
///             })
///         }
///     }
/// }
/// ```
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Widget: Layout {
    /// Pre-configuration
    ///
    /// This method is called before children are configured to assign a
    /// [`WidgetId`]. Usually it does nothing else, but a custom implementation
    /// may be used to affect child configuration, e.g. via
    /// [`EventState::new_accel_layer`].
    ///
    /// Default impl: assign `id` to self
    fn pre_configure(&mut self, mgr: &mut ConfigMgr, id: WidgetId);

    /// Configure widget
    ///
    /// Widgets are *configured* on window creation or dynamically via the
    /// parent calling [`ConfigMgr::configure`]. Parent widgets are responsible
    /// for ensuring that children are configured before calling
    /// [`Layout::size_rules`] or [`Layout::set_rect`]. Configuration may be
    /// repeated and may be used as a mechanism to change a child's [`WidgetId`],
    /// but this may be expensive.
    ///
    /// This method may be used to configure event handling and to load
    /// resources, including resources affecting [`Layout::size_rules`].
    ///
    /// The window's scale factor (and thus any sizes available through
    /// [`ConfigMgr::size_mgr`]) may not be correct initially (some platforms
    /// construct all windows using scale factor 1) and/or may change in the
    /// future. Changes to the scale factor result in recalculation of
    /// [`Layout::size_rules`] but not repeated configuration.
    fn configure(&mut self, mgr: &mut ConfigMgr) {
        let _ = mgr;
    }

    /// Is this widget navigable via <kbd>Tab</kbd> key?
    ///
    /// Defaults to `false`.
    #[inline]
    fn navigable(&self) -> bool {
        false
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
    /// Default implementation:
    ///
    /// -   Generated from `#[widget]`'s layout property, if used
    /// -   Otherwise, iterate through children in order of definition
    #[inline]
    fn nav_next(
        &mut self,
        mgr: &mut EventMgr,
        reverse: bool,
        from: Option<usize>,
    ) -> Option<usize> {
        let _ = mgr;
        crate::util::nav_next(reverse, from, self.num_children())
    }

    /// Pre-event-handler
    ///
    /// This is an internal method generated by the `#[widget]` macro. It is not
    /// part of the stable API. Do not implement or call this method directly.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn pre_handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response;

    /// Handle an [`Event`]
    ///
    /// This is the primary event handler (see [documentation](crate::event)).
    ///
    /// This method is called on the primary event target. In this case,
    /// [`EventMgr::last_child`] returns `None`.
    ///
    /// This method may also be called on ancestors during unwinding (if the
    /// event remains [unused](Response::Unused) and the event
    /// [is reusable](Event::is_reusable)). In this case,
    /// [`EventMgr::last_child`] returns `Some(index)` with the index of the
    /// child being unwound from.
    ///
    /// Default implementation of `handle_event`: do nothing; return
    /// [`Response::Unused`].
    ///
    /// # Calling `handle_event`
    ///
    /// It is not recommended to call `handle_event` directly except on `self`.
    /// Doing so would miss related event handling code such as cursor-hover
    /// effects and calling other event-handling methods on parents.
    /// Instead, one should call [`EventMgr::send`] with the target's `id`.
    #[inline]
    fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        let _ = (mgr, event);
        Response::Unused
    }

    /// Potentially steal an event before it reaches a child
    ///
    /// This is an optional event handler (see [documentation](crate::event)).
    ///
    /// May cause a panic if this method returns [`Response::Unused`] but does
    /// affect `mgr` (e.g. by calling [`EventMgr::set_scroll`] or leaving a
    /// message on the stack, possibly from [`EventMgr::send`]).
    /// This is considered a corner-case and not currently supported.
    ///
    /// Default implementation: return [`Response::Unused`].
    #[inline]
    fn steal_event(&mut self, mgr: &mut EventMgr, id: &WidgetId, event: &Event) -> Response {
        let _ = (mgr, id, event);
        Response::Unused
    }

    /// Handler for messages from children/descendants
    ///
    /// This is the secondary event handler (see [documentation](crate::event)).
    ///
    /// It is implied that the stack contains at least one message.
    /// Use [`EventMgr::try_pop`] and/or [`EventMgr::try_observe`].
    ///
    /// [`EventMgr::last_child`] may be called to find the message's sender.
    /// This may return [`None`] (if no child was visited, which implies that
    /// the message was sent by `self`).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_message(&mut self, mgr: &mut EventMgr) {
        let _ = mgr;
    }

    /// Handler for scrolling
    ///
    /// When, during [event handling](crate::event), a widget which is a strict
    /// descendant of `self` (i.e. not `self`) calls [`EventMgr::set_scroll`]
    /// with a value other than [`Scroll::None`], this method is called.
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
    /// [`EventMgr::last_child`] may be called to find the child responsible,
    /// and should never return [`None`] (when called from this method).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) {
        let _ = (mgr, scroll);
    }
}

/// Action of Node::_nav_next
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NavAdvance {
    /// Match only `focus` if navigable
    None,
    /// Walk children forwards, self first
    ///
    /// May match `focus` only if `allow_focus: bool`.
    Forward(bool),
    /// Walk children backwards, self last
    ///
    /// May match `focus` only if `allow_focus: bool`.
    Reverse(bool),
}

/// Node: dyn-safe widget
///
/// This trait is automatically implemented for every [`Widget`].
/// Directly implementing this trait is not supported.
///
/// All methods are hidden and direct usage is not supported; instead use the
/// [`ConfigMgr`] and [`EventMgr`] types which use these methods internally.
pub trait Node: Layout {
    /// Internal method: configure recursively
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId);

    /// Internal method: broadcast recursively
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _broadcast(&mut self, cx: &mut EventMgr, count: &mut usize, event: Event);

    /// Internal method: send recursively
    ///
    /// If `disabled`, widget `id` does not receive the `event`. Widget `id` is
    /// the first disabled widget (may be an ancestor of the original target);
    /// ancestors of `id` are not disabled.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _send(&mut self, cx: &mut EventMgr, id: WidgetId, disabled: bool, event: Event) -> Response;

    /// Internal method: replay recursively
    ///
    /// Behaves as if an event had been sent to `id`, then the widget had pushed
    /// `msg` to the message stack. Widget `id` or any ancestor may handle.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: Erased);

    /// Internal method: search for the previous/next navigation target
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _nav_next(
        &mut self,
        cx: &mut EventMgr,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId>;
}

impl<W: Widget> Node for W {
    fn _configure(&mut self, cx: &mut ConfigMgr, id: WidgetId) {
        self.pre_configure(cx, id);

        for index in 0..self.num_children() {
            let id = self.make_child_id(index);
            if id.is_valid() {
                if let Some(widget) = self.get_child_mut(index) {
                    widget._configure(cx, id);
                }
            }
        }

        self.configure(cx);
    }

    fn _broadcast(&mut self, cx: &mut EventMgr, count: &mut usize, event: Event) {
        self.handle_event(cx, event.clone());
        *count += 1;
        for index in 0..self.num_children() {
            if let Some(w) = self.get_child_mut(index) {
                w._broadcast(cx, count, event.clone());
            }
        }
    }

    fn _send(&mut self, cx: &mut EventMgr, id: WidgetId, disabled: bool, event: Event) -> Response {
        let mut response = Response::Unused;
        if id == self.id_ref() {
            if event == Event::NavFocus(true) {
                cx.set_scroll(Scroll::Rect(self.rect()));
            }

            if disabled {
                return response;
            }

            response |= self.pre_handle_event(cx, event);
        } else if self.steal_event(cx, &id, &event).is_used() {
            response = Response::Used;
        } else {
            cx.assert_post_steal_unused();
            let Some(index) = self.find_child_index(&id) else {
                log::warn!(
                    "Node: Widget {} cannot find path to {id}",
                    self.identify()
                );
                return response;
            };

            let translation = self.translation();
            if let Some(w) = self.get_child_mut(index) {
                response = w._send(cx, id, disabled, event.clone() + translation);
                if let Some(scroll) = cx.post_send(index) {
                    self.handle_scroll(cx, scroll);
                }
            } else {
                log::warn!(
                    "Node: {} found index {index} for {id} but not child",
                    self.identify()
                );
            }

            if response.is_unused() && event.is_reusable() {
                response = self.handle_event(cx, event);
            }
        }

        if cx.has_msg() {
            self.handle_message(cx);
        }

        response
    }

    fn _replay(&mut self, cx: &mut EventMgr, id: WidgetId, msg: Erased) {
        if let Some(index) = self.find_child_index(&id) {
            if let Some(w) = self.get_child_mut(index) {
                w._replay(cx, id, msg);
                if let Some(scroll) = cx.post_send(index) {
                    self.handle_scroll(cx, scroll);
                }
            } else {
                log::warn!(
                    "Node: {} found index {index} for {id} but not child",
                    self.identify()
                );
            }

            if cx.has_msg() {
                self.handle_message(cx);
            }
        } else if id == self.id_ref() {
            cx.push_erased(msg);
            self.handle_message(cx);
        } else {
            log::warn!("Node: Widget {} cannot find path to {id}", self.identify());
        }
    }

    fn _nav_next(
        &mut self,
        cx: &mut EventMgr,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId> {
        if cx.is_disabled(self.id_ref()) {
            return None;
        }

        let mut child = focus.and_then(|id| self.find_child_index(id));

        if let Some(index) = child {
            if let Some(id) = self
                .get_child_mut(index)
                .and_then(|w| w._nav_next(cx, focus, advance))
            {
                return Some(id);
            }
        }

        let can_match_self = match advance {
            NavAdvance::None => true,
            NavAdvance::Forward(true) => true,
            NavAdvance::Forward(false) => !self.eq_id(focus),
            _ => false,
        };
        if can_match_self && self.navigable() {
            return Some(self.id());
        }

        let rev = match advance {
            NavAdvance::None => return None,
            NavAdvance::Forward(_) => false,
            NavAdvance::Reverse(_) => true,
        };

        while let Some(index) = self.nav_next(cx, rev, child) {
            if let Some(id) = self
                .get_child_mut(index)
                .and_then(|w| w._nav_next(cx, focus, advance))
            {
                return Some(id);
            }
            child = Some(index);
        }

        let can_match_self = match advance {
            NavAdvance::Reverse(true) => true,
            NavAdvance::Reverse(false) => !self.eq_id(focus),
            _ => false,
        };
        if can_match_self && self.navigable() {
            return Some(self.id());
        }

        None
    }
}

/// Extension trait over widgets
pub trait NodeExt: Node {
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
    fn find_node(&self, id: &WidgetId) -> Option<&dyn Node> {
        if let Some(index) = self.find_child_index(id) {
            self.get_child(index).and_then(|child| child.find_node(id))
        } else if self.eq_id(id) {
            return Some(self.as_node());
        } else {
            None
        }
    }

    /// Find the descendant with this `id`, if any
    fn find_node_mut(&mut self, id: &WidgetId) -> Option<&mut dyn Node> {
        if let Some(index) = self.find_child_index(id) {
            self.get_child_mut(index)
                .and_then(|child| child.find_node_mut(id))
        } else if self.eq_id(id) {
            return Some(self.as_node_mut());
        } else {
            None
        }
    }
}
impl<W: Node + ?Sized> NodeExt for W {}
