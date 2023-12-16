// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

use super::{Layout, Node};
#[allow(unused)] use crate::event::Used;
use crate::event::{ConfigCx, Event, EventCx, IsUsed, Scroll, Unused};
use crate::{messages::Erased, Id};
use kas_macros::autoimpl;

#[allow(unused)] use kas_macros as macros;

/// Widget event-handling
///
/// This trait governs event handling as part of a [`Widget`] implementation.
/// It is used by the [`#widget`] macro to generate hidden [`Widget`] methods.
///
/// The implementation of this method may be omitted where no event-handling is
/// required. All methods have a default implementation.
///
/// # Widget lifecycle
///
/// 1.  The widget is configured ([`Events::configure`]) and immediately updated
///     ([`Events::update`]).
/// 2.  The widget has its size-requirements checked by calling
///     [`Layout::size_rules`] for each axis.
/// 3.  [`Layout::set_rect`] is called to position elements. This may use data
///     cached by `size_rules`.
/// 4.  The widget is updated again after any data change (see [`ConfigCx::update`]).
/// 5.  The widget is ready for event-handling and drawing
///     ([`Events::handle_event`], [`Layout::find_id`], [`Layout::draw`]).
///
/// Widgets are responsible for ensuring that their children may observe this
/// lifecycle. Usually this simply involves inclusion of the child in layout
/// operations. Steps of the lifecycle may be postponed until a widget becomes
/// visible.
///
/// [`#widget`]: macros::widget
pub trait Events: Widget + Sized {
    /// Make an identifier for a child
    ///
    /// This is used to assign children identifiers. It may return
    /// [`Id::default`] in order to avoid configuring the child, but in
    /// this case the widget must configure via another means.
    ///
    /// If this is implemented explicitly then [`Layout::find_child_index`] must
    /// be too.
    ///
    /// Default impl: `self.id_ref().make_child(index)`
    #[inline]
    fn make_child_id(&mut self, index: usize) -> Id {
        self.id_ref().make_child(index)
    }

    /// Configure self
    ///
    /// Widgets are *configured* before sizing, drawing and event handling (see
    /// [widget lifecycle](Widget#widget-lifecycle)). Configuration may be
    /// repeated at any time.
    ///
    /// [`Self::update`] is always called immediately after this method,
    /// followed by [`Self::configure_recurse`].
    ///
    /// The window's scale factor (and thus any sizes available through
    /// [`ConfigCx::size_cx`]) may not be correct initially (some platforms
    /// construct all windows using scale factor 1) and/or may change in the
    /// future. Changes to the scale factor result in recalculation of
    /// [`Layout::size_rules`] but not repeated configuration.
    ///
    /// The default implementation does nothing.
    fn configure(&mut self, cx: &mut ConfigCx) {
        let _ = cx;
    }

    /// Configure children
    ///
    /// This method is called after [`Self::configure`].
    /// It usually configures all children.
    ///
    /// The default implementation suffices except where children should *not*
    /// be configured (for example, to delay configuration of hidden children).
    ///
    /// Use [`Events::make_child_id`] and [`ConfigCx::configure`].
    fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        for index in 0..self.num_children() {
            let id = self.make_child_id(index);
            if id.is_valid() {
                self.as_node(data)
                    .for_child(index, |node| cx.configure(node, id));
            }
        }
    }

    /// Update self using input data
    ///
    /// This method is called immediately after [`Self::configure`] and after
    /// any input data is updated, before [`Layout::draw`] is called.
    /// Typically this method is called immediately after the data is updated
    /// but the call may be delayed until when the widget becomes visible.
    ///
    /// This method is called before [`Self::update_recurse`].
    ///
    /// The default implementation does nothing.
    fn update(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (cx, data);
    }

    /// Update children
    ///
    /// This method is called after [`Self::update`]. It usually configures all
    /// children. Children should be updated even if their data is `()` or is
    /// unchanged.
    ///
    /// The default implementation suffices except where children should *not*
    /// be updated (for example, to delay update of hidden children).
    ///
    /// Use [`ConfigCx::update`].
    fn update_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        for index in 0..self.num_children() {
            self.as_node(data).for_child(index, |node| cx.update(node));
        }
    }

    /// Is this widget navigable via <kbd>Tab</kbd> key?
    ///
    /// Note that when this method returns `false` the widget will not receive
    /// navigation focus via the <kbd>Tab</kbd> key, but it may still receive
    /// navigation focus through some other means, for example a keyboard
    /// shortcut or a mouse click.
    ///
    /// Defaults to `false`. May instead be set via the `navigable` property of
    /// the `#[widget]` macro.
    #[inline]
    fn navigable(&self) -> bool {
        false
    }

    /// Mouse focus handler
    ///
    /// Called when mouse hover state changes.
    ///
    /// When the [`#widget`] macro properties `hover_highlight` or `cursor_icon`
    /// are used, an instance of this method is generated. Otherwise, the
    /// default implementation of this method does nothing.
    ///
    /// Note: to implement `hover_highlight`, simply request a redraw on
    /// focus gain and loss. To implement `cursor_icon`, call
    /// `cx.set_hover_cursor(EXPR);` on focus gain.
    ///
    /// [`#widget`]: macros::widget
    #[inline]
    fn handle_hover(&mut self, cx: &mut EventCx, state: bool) {
        let _ = (cx, state);
    }

    /// Handle an [`Event`]
    ///
    /// This is the primary event handler (see [documentation](crate::event)).
    ///
    /// This method is called on the primary event target. In this case,
    /// [`EventCx::last_child`] returns `None`.
    ///
    /// This method may also be called on ancestors during unwinding (if the
    /// event remains [unused](Unused) and the event
    /// [is reusable](Event::is_reusable)). In this case,
    /// [`EventCx::last_child`] returns `Some(index)` with the index of the
    /// child being unwound from.
    ///
    /// Default implementation of `handle_event`: do nothing; return
    /// [`Unused`].
    fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> IsUsed {
        let _ = (cx, data, event);
        Unused
    }

    /// Potentially steal an event before it reaches a child
    ///
    /// This is an optional event handler (see [documentation](crate::event)).
    ///
    /// The method should *either* return [`Used`] or return [`Unused`] without
    /// modifying `cx`; attempting to do otherwise (e.g. by calling
    /// [`EventCx::set_scroll`] or leaving a message on the stack when returning
    /// [`Unused`]) will result in a panic.
    ///
    /// Default implementation: return [`Unused`].
    fn steal_event(
        &mut self,
        cx: &mut EventCx,
        data: &Self::Data,
        id: &Id,
        event: &Event,
    ) -> IsUsed {
        let _ = (cx, data, id, event);
        Unused
    }

    /// Handler for messages from children/descendants
    ///
    /// This is the secondary event handler (see [documentation](crate::event)).
    ///
    /// It is implied that the stack contains at least one message.
    /// Use [`EventCx::try_pop`] and/or [`EventCx::try_observe`].
    ///
    /// [`EventCx::last_child`] may be called to find the message's sender.
    /// This may return [`None`] (if no child was visited, which implies that
    /// the message was sent by `self`).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
        let _ = (cx, data);
    }

    /// Handler for scrolling
    ///
    /// When, during [event handling](crate::event), a widget which is a strict
    /// descendant of `self` (i.e. not `self`) calls [`EventCx::set_scroll`]
    /// with a value other than [`Scroll::None`], this method is called.
    ///
    /// Note that [`Scroll::Rect`] values are in the child's coordinate space,
    /// and must be translated to the widget's own coordinate space by this
    /// method (this is not done by the default implementation since any widget
    /// with non-zero translation very likely wants to implement this method
    /// anyway).
    ///
    /// If the child is in an independent coordinate space, then this method
    /// should call `cx.set_scroll(Scroll::None)` to avoid any reactions to
    /// child's scroll requests.
    ///
    /// [`EventCx::last_child`] may be called to find the child responsible,
    /// and should never return [`None`] (when called from this method).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_scroll(&mut self, cx: &mut EventCx, data: &Self::Data, scroll: Scroll) {
        let _ = (cx, data, scroll);
    }
}

/// Action of Widget::_nav_next
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NavAdvance {
    /// Match only `focus` if navigable
    None,
    /// Walk children forwards, self first
    ///
    /// Parameter: whether this can match self (in addition to other widgets).
    Forward(bool),
    /// Walk children backwards, self last
    ///
    /// Parameter: whether this can match self (in addition to other widgets).
    Reverse(bool),
}

/// The Widget trait
///
/// The primary widget trait covers event handling over super trait [`Layout`]
/// which governs layout, drawing, child enumeration and identification.
/// Most methods of `Widget` are hidden and only for use within the Kas library.
///
/// `Widget` is dyn-safe given a type parameter, e.g. `dyn Widget<Data = ()>`.
/// [`Layout`] is dyn-safe without a type parameter. [`Node`] is a dyn-safe
/// abstraction over a `&dyn Widget<Data = T>` plus a `&T` data parameter.
///
/// # Widget lifecycle
///
/// 1.  The widget is configured ([`Events::configure`]) and immediately updated
///     ([`Events::update`]).
/// 2.  The widget has its size-requirements checked by calling
///     [`Layout::size_rules`] for each axis.
/// 3.  [`Layout::set_rect`] is called to position elements. This may use data
///     cached by `size_rules`.
/// 4.  The widget is updated again after any data change (see [`ConfigCx::update`]).
/// 5.  The widget is ready for event-handling and drawing
///     ([`Events::handle_event`], [`Layout::find_id`], [`Layout::draw`]).
///
/// Widgets are responsible for ensuring that their children may observe this
/// lifecycle. Usually this simply involves inclusion of the child in layout
/// operations. Steps of the lifecycle may be postponed until a widget becomes
/// visible.
///
/// # Implementing Widget
///
/// To implement a widget, use the [`#widget`] macro within an
/// [`impl_scope`](macros::impl_scope). **This is the only supported method of
/// implementing `Widget`.**
///
/// Explicit (partial) implementations of [`Widget`], [`Layout`] and [`Events`]
/// are optional. The [`#widget`] macro completes implementations.
///
/// Synopsis:
/// ```ignore
/// impl_scope! {
///     #[widget {
///         // macro properties (all optional)
///         Data = T;
///         layout = self.foo;
///     }]
///     struct MyWidget {
///         core: widget_core!(),
///         #[widget] foo: impl Widget<Data = T> = make_foo(),
///         // ...
///     }
///
///     // Optional implementations:
///     impl Layout for Self { /* ... */ }
///     impl Events for Self { /* ... */ }
///     impl Self { /* ... */ }
/// }
/// ```
///
/// Details may be categorised as follows:
///
/// -   **Data**: the type [`Widget::Data`] must be specified exactly once, but
///     this type may be given in any of three locations: as a property of the
///     [`#widget`] macro or as [`Widget::Data`].
/// -   **Core** methods of [`Layout`] are *always* implemented via the [`#widget`]
///     macro, whether or not an `impl Layout { ... }` item is present.
/// -   **Introspection** methods [`Layout::num_children`], [`Layout::get_child`]
///     and [`Widget::for_child_node`] are implemented by the [`#widget`] macro
///     in most cases: child widgets embedded within a layout descriptor or
///     included as fields marked with `#[widget]` are enumerated.
/// -   **Introspection** methods [`Layout::find_child_index`] and
///     [`Events::make_child_id`] have default implementations which *usually*
///     suffice.
/// -   **Layout** is specified either via [layout syntax](macros::widget#layout-1)
///     or via implementation of at least [`Layout::size_rules`] and
///     [`Layout::draw`] (optionally also `set_rect`, `nav_next`, `translation`
///     and `find_id`).
///-    **Event handling** is optional, implemented through [`Events`].
///
/// For examples, check the source code of widgets in the widgets library
/// or [examples apps](https://github.com/kas-gui/kas/tree/master/examples).
/// (Check that the code uses the same Kas version since the widget traits are
/// not yet stable.)
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Widget: Layout {
    /// Input data type
    ///
    /// Widget expects data of this type to be provided by reference when
    /// calling any event-handling operation on this widget.
    ///
    /// This type may be specified using a [`#widget`] macro property in case
    /// this trait is not explicitly implemented.
    ///
    /// [`#widget`]: macros::widget
    type Data;

    /// Erase type
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn as_node<'a>(&'a mut self, data: &'a Self::Data) -> Node<'a> {
        let _ = data;
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Call closure on child with given `index`, if `index < self.num_children()`.
    ///
    /// Widgets with no children or using the `#[widget]` attribute on fields do
    /// not need to implement this. Widgets with an explicit implementation of
    /// [`Layout::num_children`] also need to implement this.
    ///
    /// It is recommended to use the methods on [`Node`]
    /// instead of calling this method.
    fn for_child_node(
        &mut self,
        data: &Self::Data,
        index: usize,
        closure: Box<dyn FnOnce(Node<'_>) + '_>,
    ) {
        let _ = (data, index, closure);
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Internal method: configure recursively
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _configure(&mut self, cx: &mut ConfigCx, data: &Self::Data, id: Id);

    /// Internal method: update recursively
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _update(&mut self, cx: &mut ConfigCx, data: &Self::Data);

    /// Internal method: send recursively
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _send(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id, event: Event) -> IsUsed;

    /// Internal method: replay recursively
    ///
    /// Behaves as if an event had been sent to `id`, then the widget had pushed
    /// `msg` to the message stack. Widget `id` or any ancestor may handle.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _replay(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id, msg: Erased);

    /// Internal method: search for the previous/next navigation target
    ///
    /// `focus`: the current focus or starting point.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        data: &Self::Data,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id>;
}
