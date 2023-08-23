// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

use super::{Layout, Node};
use crate::event::{ConfigCx, Event, EventCx, Response, Scroll};
use crate::{Erased, WidgetId};
use kas_macros::autoimpl;

#[allow(unused)] use kas_macros as macros;

/// Widget event-handling
///
/// This trait governs event handling as part of a [`Widget`] implementation.
/// It is used by the [`#widget`] macro to generate hidden [`Widget`] methods.
///
/// The implementation of this method may be omitted where no event-handling is
/// required. All methods have a default trivial implementation except
/// [`Events::pre_configure`] which assigns `self.core.id = id`.
///
/// [`#widget`]: macros::widget
pub trait Events: Layout + Sized {
    /// Input data type
    ///
    /// This type must match [`Widget::Data`]. When using the `#widget` macro,
    /// the type must be specified exactly once in one of three places: here,
    /// in the implementation of [`Widget`], or via the `Data` property of
    /// [`#widget`].
    ///
    /// [`#widget`]: macros::widget
    type Data;

    /// Recursion range
    ///
    /// Usually on update, all child widgets are updated recursively. This
    /// method may be called to restrict which children get updated.
    ///
    /// Widgets do not need to be updated if not visible, but in this case must
    /// be updated when made visible (for example, the `Stack` widget updates
    /// only the visible page).
    ///
    /// Default implementation: `0..self.num_children()`.
    fn recurse_range(&self) -> std::ops::Range<usize> {
        0..self.num_children()
    }

    /// Pre-configuration
    ///
    /// This method is called before children are configured to assign a
    /// [`WidgetId`], therefore implementations should not access child state
    /// (`child.id()` will be invalid the first time this method is called).
    ///
    /// This method must set `self.core.id = id`.
    /// The default (macro-provided) impl does so.
    fn pre_configure(&mut self, cx: &mut ConfigCx, id: WidgetId);

    /// Configure widget
    ///
    /// Widgets are *configured* on window creation or dynamically via the
    /// parent calling [`ConfigCx::configure`]. Parent widgets are responsible
    /// for ensuring that children are configured before calling
    /// [`Layout::size_rules`] or [`Layout::set_rect`]. Configuration may be
    /// repeated and may be used as a mechanism to change a child's [`WidgetId`].
    ///
    /// This method may be used to configure event handling and to load
    /// resources, including resources affecting [`Layout::size_rules`].
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

    /// Update data
    ///
    /// This method is called immediately after [`Self::configure`] and after
    /// any input data is updated, before [`Layout::draw`] is called.
    /// Typically this method is called immediately after the data is updated
    /// but the call may be delayed until when the widget becomes visible.
    ///
    /// This method is called on the parent widget before children get updated.
    ///
    /// It is possible to limit which children get updated via
    /// [`Self::recurse_range`].
    /// Widgets should be updated even if their data is `()` or is unchanged.
    ///
    /// The default implementation does nothing.
    fn update(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (cx, data);
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
    /// Called on [`Event::MouseHover`] before [`Self::handle_event`]. If this
    /// returns [`Response::Used`], then `handle_event` is not called.
    ///
    /// When the [`#widget`] macro properties `hover_highlight` or `cursor_icon`
    /// are used, an instance of this method is generated. Otherwise, the
    /// default implementation of this method does nothing.
    ///
    /// To implement this functionality directly (instead of using the
    /// properties), `hover_highlight` should call `cx.redraw(self.id());` on
    /// focus gain and loss while `cursor_icon` should call
    /// `cx.set_cursor_icon(EXPR);` on focus gain. (Such code may be implemented
    /// in this method or in [`Self::handle_event`]).
    ///
    /// [`#widget`]: macros::widget
    #[inline]
    fn mouse_hover(&mut self, cx: &mut EventCx, state: bool) -> Response {
        let _ = (cx, state);
        Response::Unused
    }

    /// Handle an [`Event`]
    ///
    /// This is the primary event handler (see [documentation](crate::event)).
    ///
    /// This method is called on the primary event target. In this case,
    /// [`EventCx::last_child`] returns `None`.
    ///
    /// This method may also be called on ancestors during unwinding (if the
    /// event remains [unused](Response::Unused) and the event
    /// [is reusable](Event::is_reusable)). In this case,
    /// [`EventCx::last_child`] returns `Some(index)` with the index of the
    /// child being unwound from.
    ///
    /// Default implementation of `handle_event`: do nothing; return
    /// [`Response::Unused`].
    ///
    /// Use [`EventCx::send`] instead of calling this method.
    #[inline]
    fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> Response {
        let _ = (cx, data, event);
        Response::Unused
    }

    /// Potentially steal an event before it reaches a child
    ///
    /// This is an optional event handler (see [documentation](crate::event)).
    ///
    /// May cause a panic if this method returns [`Response::Unused`] but does
    /// affect `cx` (e.g. by calling [`EventCx::set_scroll`] or leaving a
    /// message on the stack, possibly from [`EventCx::send`]).
    /// This is considered a corner-case and not currently supported.
    ///
    /// Default implementation: return [`Response::Unused`].
    #[inline]
    fn steal_event(
        &mut self,
        cx: &mut EventCx,
        data: &Self::Data,
        id: &WidgetId,
        event: &Event,
    ) -> Response {
        let _ = (cx, data, id, event);
        Response::Unused
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
    /// May match `focus` only if `allow_focus: bool`.
    Forward(bool),
    /// Walk children backwards, self last
    ///
    /// May match `focus` only if `allow_focus: bool`.
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
/// # Implementing Widget
///
/// To implement a widget, use the [`#widget`] macro within an
/// [`impl_scope`](macros::impl_scope). **This is the only supported method of
/// implementing `Widget`.** Synopsis:
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
///     [`#widget`] macro, as [`Events::Data`] or as [`Widget::Data`].
/// -   **Core** methods of [`Layout`] are *always* implemented via the [`#widget`]
///     macro, whether or not an `impl Layout { ... }` item is present.
/// -   **Introspection** methods [`Layout::num_children`], [`Layout::get_child`]
///     and [`Widget::for_child_node`] are implemented by the [`#widget`] macro
///     in most cases: child widgets embedded within a layout descriptor or
///     included as fields marked with `#[widget]` are enumerated.
/// -   **Introspection** methods [`Layout::find_child_index`] and
///     [`Layout::make_child_id`] have default implementations which *usually*
///     suffice.
/// -   **Layout** is specified either via [layout syntax](macros::widget#layout-1)
///     or via implementation of at least [`Layout::size_rules`] and
///     [`Layout::draw`] (optionally also `set_rect`, `nav_next`, `translation`
///     and `find_id`).
///-    **Event handling** is optional, implemented through [`Events`].
///
/// Some simple examples follow. See also
/// [examples apps](https://github.com/kas-gui/kas/tree/master/examples)
/// and [`kas_widgets` code](https://github.com/kas-gui/kas/tree/master/crates/kas-widgets).
/// ```
/// # extern crate kas_core as kas;
/// use kas::event;
/// use kas::prelude::*;
/// use kas::text::Text;
/// use kas::theme::TextClass;
/// use std::fmt::Debug;
///
/// impl_scope! {
///     /// A text label
///     #[widget {
///         Data = ();
///     }]
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
///         /// Get the accelerator key
///         pub fn accel_key(&self) -> Option<&event::Key> {
///             self.label.text().key()
///         }
///     }
///
///     impl Layout for Self {
///         fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
///             axis.set_default_align_hv(Align::Default, Align::Center);
///             sizer.text_rules(&mut self.label, self.class, axis)
///         }
///
///         fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
///             self.core.rect = rect;
///             cx.text_set_size(&mut self.label, self.class, rect.size, None);
///         }
///
///         fn draw(&mut self, mut draw: DrawCx) {
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
///     impl Events for Self {
///         type Data = ();
///
///         fn configure(&mut self, cx: &mut ConfigCx) {
///             if let Some(key) = self.label.accel_key() {
///                 cx.add_accel_key(self.id_ref(), key.clone());
///             }
///         }
///
///         fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> Response {
///             event.on_activate(cx, self.id(), |cx| {
///                 cx.push(self.message.clone());
///                 Response::Used
///             })
///         }
///     }
/// }
/// ```
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Widget: Layout {
    /// Input data type
    ///
    /// Widget expects data of this type to be provided by reference when
    /// calling any event-handling operation on this widget.
    ///
    /// This type must match [`Events::Data`] if `Events` is implemented when
    /// using the `#[widget]` macro. The type only needs to be specified once,
    /// here, in the implementation of [`Events`], or via the `Data` property.
    type Data;

    /// Erase type
    fn as_node<'a>(&'a mut self, data: &'a Self::Data) -> Node<'a>;

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
    );

    /// Internal method: configure recursively
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _configure(&mut self, cx: &mut ConfigCx, data: &Self::Data, id: WidgetId);

    /// Internal method: update recursively
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _update(&mut self, cx: &mut ConfigCx, data: &Self::Data);

    /// Internal method: send recursively
    ///
    /// If `disabled`, widget `id` does not receive the `event`. Widget `id` is
    /// the first disabled widget (may be an ancestor of the original target);
    /// ancestors of `id` are not disabled.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _send(
        &mut self,
        cx: &mut EventCx,
        data: &Self::Data,
        id: WidgetId,
        disabled: bool,
        event: Event,
    ) -> Response;

    /// Internal method: replay recursively
    ///
    /// Behaves as if an event had been sent to `id`, then the widget had pushed
    /// `msg` to the message stack. Widget `id` or any ancestor may handle.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _replay(&mut self, cx: &mut EventCx, data: &Self::Data, id: WidgetId, msg: Erased);

    /// Internal method: search for the previous/next navigation target
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn _nav_next(
        &mut self,
        cx: &mut EventCx,
        data: &Self::Data,
        focus: Option<&WidgetId>,
        advance: NavAdvance,
    ) -> Option<WidgetId>;
}
