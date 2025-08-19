// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget and Events traits

#[allow(unused)] use super::{Events, Layout};
use super::{Node, Tile};
use crate::Id;
#[allow(unused)] use crate::event::EventState;
use crate::event::{ConfigCx, Event, EventCx, IsUsed, NavAdvance};
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
/// 1.  The widget is configured ([`Events::configure`]) and immediately updated
///     ([`Events::update`]).
/// 2.  The widget has its size-requirements checked by calling
///     [`Layout::size_rules`] for each axis.
/// 3.  [`Layout::set_rect`] is called to position elements. This may use data
///     cached by `size_rules`.
/// 4.  The widget is updated again after any data change (see [`ConfigCx::update`]).
/// 5.  The widget is ready for event-handling and drawing
///     ([`Events::handle_event`], [`Layout::try_probe`], [`Layout::draw`]).
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
/// -   **Layout** is specified either via [layout syntax](macros::widget#layout-1)
///     or via implementation of at least [`Layout::size_rules`] and
///     [`Layout::draw`] (optionally also `set_rect`, `nav_next`, `translation`
///     and [`Tile::probe`]).
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
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _configure(&mut self, cx: &mut ConfigCx, data: &Self::Data, id: Id);

    /// Internal method: update recursively
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _update(&mut self, cx: &mut ConfigCx, data: &Self::Data);

    /// Internal method: send recursively
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _send(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id, event: Event) -> IsUsed;

    /// Internal method: replay recursively
    ///
    /// Traverses the widget tree to `id`, then unwinds.
    /// It is expected that some message is available on the stack.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _replay(&mut self, cx: &mut EventCx, data: &Self::Data, id: Id);

    /// Internal method: search for the previous/next navigation target
    ///
    /// `focus`: the current focus or starting point.
    ///
    /// Do not implement this method directly!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn _nav_next(
        &mut self,
        cx: &mut ConfigCx,
        data: &Self::Data,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id>;
}
