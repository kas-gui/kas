// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

#[allow(unused)] use super::{Events, Layout};
use super::{Node, Tile};
use crate::Id;
#[allow(unused)] use crate::event::EventState;
use crate::event::{ConfigCx, Event, EventCx, IsUsed};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::theme::{DrawCx, SizeCx};
#[allow(unused)] use kas_macros as macros;
use kas_macros::autoimpl;

/// The Widget trait
///
/// The primary widget trait covers event handling over super trait [`Tile`]
/// which governs layout, drawing, child enumeration and identification.
/// Most methods of `Widget` are hidden and only for use within the Kas library.
///
/// `Widget` is dyn-safe given a type parameter, e.g. `dyn Widget<Data = ()>`.
/// [`Tile`] is dyn-safe without a type parameter. [`Node`] is a dyn-safe
/// abstraction over a `&dyn Widget<Data = T>` plus a `&T` data parameter.
///
/// # Widget lifecycle
///
/// Widget methods have a specified call order:
///
/// 1.  The widget is configured (see [`Events#configuration`])
/// 2.  The widget is updated ([`Events#update`])
/// 3.  The widget is sized (see [`Layout#sizing`])
/// 4.  The widget is ready for other methods to be called
///
/// Configuration, update and sizing may be repeated at any time (see above
/// linked documentation).
///
/// Widgets are responsible for ensuring that their children may observe this
/// lifecycle. Usually this simply involves inclusion of the child in layout
/// operations. Steps of the lifecycle may be postponed until a widget becomes
/// visible.
///
/// # Implementing Widget
///
/// To implement a widget, use the [`#widget`] macro within an
/// [`impl_self`](macros::impl_self), [`impl_scope!`](macros::impl_scope) or
/// [`impl_anon!`](macros::impl_anon) macro.
/// **This is the only supported method of implementing `Widget`.**
///
/// Explicit (partial) implementations of [`Widget`], [`Layout`], [`Tile`] and [`Events`]
/// are optional. The [`#widget`] macro completes implementations.
///
/// Synopsis:
/// ```ignore
/// #[impl_self]
/// mod MyWidget {
///     #[widget {
///         // macro properties (all optional)
///         Data = T;
///     }]
///     #[layout(self.foo)]
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
/// -   **Core** methods of [`Tile`] are *always* implemented via the [`#widget`]
///     macro, whether or not an `impl Tile { ... }` item is present.
/// -   **Introspection** methods [`Tile::child_indices`], [`Tile::get_child`]
///     and [`Widget::child_node`] are implemented by the [`#widget`] macro
///     in most cases: child widgets embedded within a layout descriptor or
///     included as fields marked with `#[widget]` are enumerated.
/// -   **Introspection** methods [`Tile::find_child_index`] and
///     [`Events::make_child_id`] have default implementations which *usually*
///     suffice.
/// -   **Layout** is specified either via [`layout`](macro@crate::layout) macro
///     or via implementation of at least [`Layout`].
///-    **Event handling** is optional, implemented through [`Events`].
///
/// For examples, check the source code of widgets in the widgets library
/// or [examples apps](https://github.com/kas-gui/kas/tree/master/examples).
/// (Check that the code uses the same Kas version since the widget traits are
/// not yet stable.)
///
/// [`#widget`]: macros::widget
#[autoimpl(for<T: trait + ?Sized> &'_ mut T, Box<T>)]
pub trait Widget: Tile {
    /// Input data type
    ///
    /// Widget expects data of this type to be provided by reference when
    /// calling any event-handling operation on this widget.
    ///
    /// Type `Data` should be specified either here (`impl Widget { ... }`) or
    /// in `impl Events { ... }`. Alternatively, if the widget has no children
    /// and no explicit `impl Events` or `impl Widget`, then `Data = ()` is
    /// assumed; or, if the prior conditions are met and `#[collection]` is used
    /// on some field, then `Data = <#field_ty as ::kas::Collection>::Data` is
    /// assumed.
    ///
    /// [`#widget`]: macros::widget
    //
    // SAFETY: the unsafe_node feature requires Data: Sized.
    type Data: Sized;

    /// Erase type
    ///
    /// This method is implemented by the `#[widget]` macro.
    fn as_node<'a>(&'a mut self, data: &'a Self::Data) -> Node<'a> {
        let _ = data;
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Access a child as a [`Node`], if available
    ///
    /// This method is the `mut` version of [`Tile::get_child`] but which also
    /// pairs the returned widget with its input `data`. It is expected to
    /// succeed where [`Tile::get_child`] succeeds.
    ///
    /// Valid `index` values may be discovered by calling
    /// [`Tile::child_indices`], [`Tile::find_child_index`] or
    /// [`Tile::nav_next`]. The `index`-to-child mapping is not
    /// required to remain fixed; use an [`Id`] to track a widget over time.
    ///
    /// This method must be implemented explicitly when [`Tile::get_child`] is.
    /// It might also need to be implemented explicitly to map `data`, though
    /// usually the `#[widget]` attribute on children specifies this mapping.
    fn child_node<'n>(&'n mut self, data: &'n Self::Data, index: usize) -> Option<Node<'n>> {
        let _ = (data, index);
        unimplemented!() // make rustdoc show that this is a provided method
    }

    /// Internal method: configure recursively
    ///
    /// Requires status: none.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _configure(&mut self, cx: &mut ConfigCx, data: &Self::Data, id: Id);

    /// Internal method: update recursively
    ///
    /// Requires status: configured.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _update(&mut self, cx: &mut ConfigCx, data: &Self::Data);

    /// Internal method: send recursively
    ///
    /// Requires status: configured and sized.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _send(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id, event: Event) -> IsUsed;

    /// Internal method: replay recursively
    ///
    /// Traverses the widget tree to `id`, then unwinds with standard handling
    /// of event state.
    ///
    /// If a message is being sent, it must already be on the stack. If the
    /// target is not found, unsent messages are dropped.
    ///
    /// Requires status: configured.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _replay(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id);
}

/// Layout routines for scrollable content
///
/// A `Viewport` supports content larger than its assigned `rect` (the `rect`
/// passed to [`Layout::set_rect`]). This `rect` is considered the viewport
/// through which content may be viewed (approximately: see
/// [`Self::draw_with_offset`]).
///
/// If the parent widget supports scrolling over contents implementing
/// `Viewport`, it should call [`Viewport::draw_with_offset`] instead of
/// [`Layout::draw`].
///
/// It is intended that the widget implementing this trait is the child of some
/// parent widget which supports scrolling through event handling and provision
/// of a scroll offset, and that this parent uses the methods of this trait
/// where applicable (see below). In case the parent does not support scrolling,
/// the widget should remain usable (but with only a subset of content being
/// accessible).
pub trait Viewport: Widget {
    /// Get content size
    ///
    /// When the content size is larger than the viewport, content becomes
    /// scrollable with a maximum offset of `content_size - viewport_size`.
    ///
    /// # Calling
    ///
    /// This method is called during sizing.
    fn content_size(&self) -> Size;

    /// Set the scroll offset
    ///
    /// The `viewport` and `offset` parameters are the same as those of
    /// [`Viewport::draw_with_offset`].
    ///
    /// # Calling
    ///
    /// This method should be called immediately after [`Layout::set_rect`]
    /// (unless it's known that the implementation doesn't use this method).
    ///
    /// # Implementation
    ///
    /// This method only needs to do anything in cases where only a subset of
    /// content is prepared.
    fn set_offset(&mut self, cx: &mut SizeCx, viewport: Rect, offset: Offset) {
        let _ = (cx, viewport, offset);
    }

    /// Update the scroll offset
    ///
    /// The `viewport` and `offset` parameters are the same as those of
    /// [`Viewport::draw_with_offset`].
    ///
    /// # Calling
    ///
    /// This method should be called whenever the scroll offset changes to allow
    /// preparation of content (unless it's known that the implementation
    /// doesn't use this method). It must be called before drawing and event
    /// handling operations using this new `offset`.
    ///
    /// # Implementation
    ///
    /// This method only needs to do anything in cases where only a subset of
    /// content is prepared.
    fn update_offset(
        &mut self,
        cx: &mut ConfigCx,
        data: &Self::Data,
        viewport: Rect,
        offset: Offset,
    ) {
        let _ = (cx, data, viewport, offset);
    }

    /// Draw with a scroll offset
    ///
    /// Drawing should be clamped to the given `viewport`. This `viewport` may
    /// be the same as [`Layout::rect`] but is allowed to be slightly different;
    /// for example `EditBox` passes a larger [`Rect`] to allow drawing in the
    /// margin allocated between its frame and content.
    ///
    /// The `offset` should be the same as that used by the parent widget
    /// in [`Tile::translation`].
    ///
    /// Effectively, content is drawn at position `self.rect().pos - offset`
    /// but clamped to `viewport`.
    ///
    /// # Calling
    ///
    /// This method should be called instead of [`Layout::draw`] by compatible
    /// parent widgets.
    ///
    /// # Implementation
    ///
    /// ## Method modification
    ///
    /// The `#[widget]` macro injects a call to [`DrawCx::set_id`] into this
    /// method where possible, allowing correct detection of disabled and
    /// highlight states.
    ///
    /// This method modification should never cause issues (besides the implied
    /// limitation that widgets cannot easily detect a parent's state while
    /// being drawn).
    fn draw_with_offset(&self, draw: DrawCx, viewport: Rect, offset: Offset);

    /// Probe a coordinate for a widget's [`Id`]
    ///
    /// # Calling
    ///
    /// This method should be called instead of [`Tile::try_probe`].
    ///
    /// # Implementation
    ///
    /// The default implementation will normally suffice. It calls
    /// [`Events::probe`] with `coord + offset` when
    /// `self.rect().contains(coord)`.
    fn try_probe_with_offset(&self, coord: Coord, offset: Offset) -> Option<Id> {
        let _ = (coord, offset);
        unimplemented!() // make rustdoc show that this is a provided method
    }
}
