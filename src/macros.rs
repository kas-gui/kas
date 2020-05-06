// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Library macros
//!
//! This documentation is provided as a reference. It may also be useful to
//! refer to the widget library and example apps for examples of usage.
//!
//! The following macros are provided:
//!
//! -   [`derive(Widget)`] is used to implement the [`Widget`] trait family
//! -   [`derive(VoidMsg)`] is a convenient way to implement `From<VoidMsg>`
//! -   [`make_widget`] allows a custom widget to be defined and instantiated
//!     simultaneously
//!
//! Note that these macros are defined in the external crate, `kas-macros`, only
//! because procedural macros must be defined in a special crate. The
//! `kas-macros` crate should not be used directly.
//!
//! [`make_widget`]: #the-make_widget-macro
//! [`derive(Widget)`]: #the-derivewidget-macro
//! [`derive(VoidMsg)`]: #the-derivevoidmsg-macro
//!
//!
//! ## The `derive(Widget)` macro
//!
//! The [`Widget`] trait is one of a family, all of which must be
//! implemented by a widget. This family may be extended with additional traits
//! in the future, and users are forbidden (to avoid breakage) from directly
//! implementing the [`Widget`] and [`WidgetCore`] traits. This `derive(Widget)`
//! macro is key to making this trait-family design possible: it (potentially)
//! implements all traits in the family at once, on an opt-out basis
//! (exception: the [`Layout`] trait is opt-in).
//!
//! It is recommended to use **nightly rustc** when developing code using this
//! macro for improved diagnostics using
//! [`proc_macro_diagnostics`](https://github.com/rust-lang/rust/issues/54140)
//! (this is enabled automatically). It is safe to use a stable Rust compiler
//! but debugging macros will be harder.
//!
//! The behaviour of this macro is controlled by attributes on struct fields and
//! on the widget struct itself.
//!
//! These attributes may be used on the struct: `widget`, `layout`, `handler`.
//! These may each appear zero or once (except `handler`; see below).
//! They support multiple parameters, e.g. `#[widget(config=noauto, children=noauto)]`.
//!
//! These attributes may be used on fields: `widget`, `widget_core`,
//! `layout_data`. The `widget` attribute supports multiple parameters,
//! discussed below (e.g. `#[widget(row=1, handler=f)]`).
//! Fields without attributes (plain data fields) are fine too.
//!
//! A simple example:
//! ```
//! use kas::prelude::*;
//!
//! #[layout(single)]
//! #[handler(generics = <> where W: Widget<Msg = event::VoidMsg>)]
//! #[derive(Clone, Debug, Widget)]
//! struct WrapperWidget<W: Widget> {
//!     #[widget_core] core: CoreData,
//!     #[widget] child: W,
//! }
//! ```
//!
//! We will now discuss each member of the [`Widget`] trait family in turn.
//!
//! ### Widget and WidgetCore
//!
//! The [`Widget`] and [`WidgetCore`] traits are always derived by this macro.
//! No configuration is available.
//!
//! One struct field with specification `#[widget_core] core: CoreData` is
//! required to support [`WidgetCore`]. The field may be accessed directly.
//!
//! ### WidgetChildren
//!
//! The [`WidgetChildren`] trait is used to enumerate child widgets. Any struct
//! field with the `#[widget]` attribute is identified as a child widget, and
//! will be enumerated by the derived implementation of this trait, in the order
//! of definition.
//!
//! In case child widgets are stored within a container (e.g. `Vec`), this macro
//! is unable to enumerate the widgets correctly. In that case one must opt out
//! of deriving this trait with `#[widget(children = noauto)]` on the struct.
//!
//! ### Layout
//!
//! The [`Layout`] trait is used to define size, structure and appearance of a
//! widget. Unlike other members of the trait family, this trait is not derived
//! by default, and the derived implementation is only useful for widgets with
//! at least one child and which don't directly draw themselves.
//!
//! The trait may be derived via a `layout` attribute, e.g. `#[layout(single)]`.
//! One of the following values must appear first in the parameter list:
//!
//! -   `single` — the widget wraps a single child, with no border or margin
//! -   `col`, `column` or `down` — child widgets are arranged in a vertical
//!     column, top-to-bottom
//! -   `up` — reversed column
//! -   `row` or `right` — child widgets are arranged in a horizontal row,
//!     left-to-right
//! -   `left` — reversed row
//! -   `grid` — child widgets are arranged in a grid; position is specified
//!     via parameters to the `#[widget]` attribute on child fields
//!
//! Optionally, a second parameter of form `area=FIELD` is allowed (e.g.
//! `#[layout(row, area=checkbox)]`). `FIELD` must identify a child widget.
//! This parameter causes the [`Layout::find_id`] function to map all
//! coordinates within the widget directly to the child's [`WidgetId`], causing
//! clicks on the parent area to send events directly to the child.
//!
//! **Child widget placement**
//!
//! All fields with attribute `#[widget]` are considered child widgets. For most
//! layouts, these are placed in order of definition.
//!
//! For the `grid` layout, parameters are used to specify position
//! (e.g. `#[widget(col=1, cspan=2)]`). These each have a default value:
//!
//! -   `col=0` or `column=0` — grid column, from left, counting from 0
//! -   `row=0` — grid row, from top, counting from 0
//! -   `cspan=1` — number of columns to span
//! -   `rspan=1` — number of rows to span
//!
//! Alignment may also be specified for children. The exact behaviour depends
//! on the child widget, and usually is only relevant when the available space
//! is greater than the child's ideal size. These parameters are used to
//! construct an [`AlignHints`] which is passed into [`Layout::set_rect`].
//!
//! -   `halign = ...` — one of `begin`, `centre`, `end`, `stretch`
//! -   `valign = ...` — one of `begin`, `centre`, `end`, `stretch`
//!
//! **Layout data storage**
//!
//! When deriving [`Layout`], data storage is required (exception: layout
//! `single` requires no storage, but defining it anyway is harmless).
//! The [`LayoutData`] trait is also derived and used to specify the required
//! data type. The `#[layout_data]` attribute is required to identify this
//! storage, resulting in a field like the following:
//! ```none
//! #[layout_data] layout_data: <Self as kas::LayoutData>::Data,
//! ```
//! This field supports `Default` and `Clone`, thus may be constructed with
//! `layout_data: Default::default()`.
//!
//! ### WidgetConfig
//!
//! The [`WidgetConfig`] trait allows additional configuration of widget
//! behaviour. It is derived by default but may be customised via a `config`
//! parameter to the `widget` attribute on the struct.
//!
//! `#[widget(config = noauto)]` or `#[widget(config(noauto))]` opts-out of
//! deriving this trait.
//!
//! The `config` parameter itself accepts parameters, which may be used to
//! modify the derived implementation, e.g. `#[widget(config(key_nav = true))]`.
//! Parameter description with default values:
//!
//! -   `key_nav = false`: a boolean, describing whether the widget supports
//!     keyboard navigation (see [`WidgetConfig::key_nav`])
//!  -   `cursor_icon = kas::event::CursorIcon::Default`: the cursor icon to use
//!     when the mouse hovers over this widget (see [`WidgetConfig::cursor_icon`])
//!
//! ### Handler and SendEvent
//!
//! The [`Handler`] and [`SendEvent`] traits are derived, unless opted out.
//! The `#[handler]` attribute allows control over this via the following
//! arguments, all of which are optional:
//!
//! -   `noauto` — do not derive [`Handler`] or [`SendEvent`]
//! -   `handle=noauto` — do not derive [`Handler`] (whose main method is [`Handler::handle`])
//! -   `send=noauto` — do not derive [`SendEvent`] (whose main method is [`SendEvent::send`])
//! -   `msg = TYPE` — the [`Handler::Msg`] associated type; if not
//!     specified, this type defaults to [`kas::event::VoidMsg`]
//! -   `generics = ...`; this parameter must appear last in the
//!     list and allows extra type parameters and/or restrictions to appear on
//!     the implementations of [`Handler`], [`SendEvent`] and [`Widget`].
//!     It accepts any of the following:
//!
//!     -   `<TYPE_PARAMS>`, for example `<T, W: Widget>` (these type parameters
//!         are *added* to those appearing on the struct definition)
//!     -   `<TYPE_PARAMS> where CONDS`, for example
//!         `<> where W: Widget<Msg = event::VoidMsg>`; note that conditions may
//!         apply to type parameters from the struct signature (in this example, `W`)
//!     -   `SUBS` where `SUBS` is a list of substitutions; e.g. if `M` is a
//!         type parameter of the struct, then `M => MyMsg` will substitute the
//!         parameter `M` for concrete type `MyMsg`.
//!         (Once [rust#20041](https://github.com/rust-lang/rust/issues/20041) is
//!         fixed, substitutions will no longer be required.)
//!     -   `SUBS <TYPE_PARAMS> where CONDS`; e.g. if `M` is a type parameter
//!         of the struct, one might use `M => <W as Handler>::Msg, <W: Widget>`
//!
//! Commonly, implementations of the [`Handler`] and [`Layout`] traits require
//! extra type bounds on the
//! `impl` which do not appear on the struct, for example a struct may be
//! parametrised with `W: Widget`, but the [`Handler`] impl may require
//! `W: Layout`. This may be achieved as follows:
//! ```
//! # use kas::macros::Widget;
//! # use kas::{CoreData, Layout, Widget, event::Handler};
//! #[layout(single)]
//! #[handler(msg = <W as Handler>::Msg, generics = <> where W: Layout)]
//! #[derive(Clone, Debug, Default, Widget)]
//! pub struct Frame<W: Widget> {
//!     #[widget_core]
//!     core: CoreData,
//!     #[widget]
//!     child: W,
//! }
//! ```
//!
//! Exceptionally, multiple `#[handler]` attributes may be used to generate
//! multiple implementations. These must use `generics` parameters which result
//! in non-overlapping bounds. This functionality is not well tested.
//!
//! **Handling response messages from children**
//!
//! The [`Handler`] trait supports a user-defined message type, `Msg`.
//! A "handler" maps a child's message type into the parent's message type.
//!
//! Where the child's message type can be converted into the parent's message
//! type using [`From`], no explicit handler is needed.
//! (This is why all message types must support `From<VoidMsg>`.)
//! In other cases, if no explicit handler is provided, an error will result:
//!
//! ```none
//! error[E0277]: the trait bound `kas::event::VoidMsg: std::convert::From<Item>` is not satisfied
//! ```
//!
//! A handler is a method on the parent struct with signature
//! `fn f(&mut self, mgr: &mut Manager, item: Item) -> Response<Out>`
//! (where `Item` is the child's message type and `Out` is the parent's message
//! type).
//!
//! A handler is bound to a child via the `widget` attribute, for example
//! `#[widget(handler = f)] child: ChildType`.
//!
//!
//! ### Examples
//!
//! A simple example is included above.
//! The example below includes multiple children and custom event handling.
//!
//! ```
//! use kas::event::{Manager, Response, VoidMsg};
//! use kas::macros::Widget;
//! use kas::widget::Label;
//! use kas::{CoreData, LayoutData, Widget};
//!
//! #[derive(Debug)]
//! enum ChildMessage { A }
//!
//! #[layout(column)]
//! #[handler(generics = <> where W: Widget<Msg = ChildMessage>)]
//! #[derive(Debug, Widget)]
//! struct MyWidget<W: Widget> {
//!     #[widget_core] core: CoreData,
//!     #[layout_data] layout_data: <Self as LayoutData>::Data,
//!     #[widget] label: Label,
//!     #[widget(handler = handler)] child: W,
//! }
//!
//! impl<W: Widget> MyWidget<W> {
//!     fn handler(&mut self, mgr: &mut Manager, msg: ChildMessage) -> Response<VoidMsg> {
//!         match msg {
//!             ChildMessage::A => { println!("handling ChildMessage::A"); }
//!         }
//!         Response::None
//!     }
//! }
//! ```
//!
//!
//! ## The `derive(VoidMsg)` macro
//!
//! This macro implements `From<VoidMsg>` for the given type (see [`VoidMsg`]).
//!
//! [`VoidMsg`]: crate::event::VoidMsg
//!
//! ### Example
//!
//! ```
//! use kas::macros::VoidMsg;
//!
//! #[derive(VoidMsg)]
//! enum MyMessage { A, B };
//! ```
//!
//!
//! ## The `make_widget` macro
//!
//! This macro allows easy creation of "layout" widgets (those whose purpose is
//! to house one or more child widgets) by introducing syntax for a struct
//! literal and adding the additional fields and implementations required by all
//! widgets.
//!
//! Syntax is similar to a Rust type definition, but with most of the types and
//! identifiers omitted. It's easiest to study an example:
//!
//! ```rust
//! # #![feature(proc_macro_hygiene)]
//! # use kas::event::{VoidResponse, VoidMsg, Manager};
//! # use kas::macros::make_widget;
//! # use kas::widget::Label;
//! # let inner_widgets = Label::new("");
//! #[derive(Clone, Copy, Debug)]
//! enum Item {
//!     Button,
//!     Check(bool),
//! }
//! let widget = make_widget! {
//!     #[layout(column)]
//!     #[handler(msg = VoidMsg)]
//!     struct {
//!         #[widget] _ = Label::new("Widget Gallery"),
//!         #[widget(handler = activations)] _ = inner_widgets,
//!         last_item: Item = Item::Button,
//!     }
//!     impl {
//!         fn activations(&mut self, mgr: &mut Manager, item: Item)
//!             -> VoidResponse
//!         {
//!             match item {
//!                 Item::Button => println!("Clicked!"),
//!                 Item::Check(b) => println!("Checkbox: {}", b),
//!             };
//!             self.last_item = item;
//!             VoidResponse::None
//!         }
//!     }
//! };
//! ```
//!
//! ### Struct and fields
//!
//! Starting from the middle, we have a `struct` definition, though two things
//! are unusual here: (1) the type is anonymous (unnamed), and (2) fields are
//! simultaneously given both type and value.
//!
//! Field specifications can get more unusual too, since both the field name and
//! the field type are optional. For example, all of the following are equivalent:
//!
//! ```nocompile
//! #[widget] l1: Label = Label::new("label 1"),
//! #[widget] _: Label = Label::new("label 2"),
//! #[widget] l3 = Label::new("label 3"),
//! #[widget] _ = Label::new("label 4"),
//! ```
//!
//! Omitting field names is fine, so long as you don't need to refer to them.
//! Omitting types, however, comes at a small cost: Rust does not support fields
//! of unspecified types, thus this must be emulated with generics. The macro
//! deals with the necessary type arguments to implementations, however macro
//! expansions (as sometimes seen in error messages) are ugly and, perhaps worst
//! of all, the field will have opaque type (making methods and inner fields
//! inaccessible). The latter can be partially remedied via trait bounds:
//!
//! ```nocompile
//! #[widget] display: impl HasText = EditBox::new("editable"),
//! ```
//!
//! Alternatively, generics can be introduced explicitly:
//!
//! ```nocompile
//! #[widget] display: for<W: Widget<Msg = VoidMsg>> Frame<W> =
//!     Frame::new(Label::new("example")),
//! ```
//!
//! ### Implementations
//!
//! Now, back to the example above, we see attributes and an `impl` block:
//!
//! ```nocompile
//! let widget = make_widget! {
//!     #[widget]
//!     #[layout(column)]
//!     #[handler(msg = VoidMsg)]
//!     struct {
//!         ...
//!     }
//!     impl {
//!         fn on_tick(&mut self, mgr: &mut Manager) {
//!             ...
//!         }
//!     }
//! };
//! ```
//!
//! Attributes may be applied to the anonymous struct like normal, with two
//! exceptions:
//!
//! 1.  `#[derive(Clone, Debug, kas::macros::Widget)]` is implied
//! 2.  `#[handler(msg = ..)]` is required unless `msg` can be inferred from a
//!     [`Handler`] implementation (this does not default to [`VoidMsg`] purely
//!     to avoid some terrible error messages)
//!
//! `impl` blocks work like usual except that the struct name and type
//! parameters are omitted. Traits may also be implemented this way:
//!
//! ```nocompile
//! impl Trait { ... }
//! ```
//!
//! ### Example
//!
//! ```
//! #![feature(proc_macro_hygiene)]
//!
//! use kas::macros::{make_widget};
//! use kas::widget::TextButton;
//!
//! #[derive(Copy, Clone, Debug)]
//! enum OkCancel {
//!     Ok,
//!     Cancel,
//! }
//!
//! let button_box = make_widget!{
//!     #[layout(row)]
//!     #[handler(msg = OkCancel)]
//!     struct {
//!         #[widget] _ = TextButton::new("Ok", OkCancel::Ok),
//!         #[widget] _ = TextButton::new("Cancel", OkCancel::Cancel),
//!     }
//! };
//! ```

// Imported for doc-links
#[allow(unused)]
use crate::{
    event::{Handler, SendEvent},
    AlignHints, CoreData, Layout, LayoutData, Widget, WidgetChildren, WidgetConfig, WidgetCore,
    WidgetId,
};

pub use kas_macros::{make_widget, VoidMsg, Widget};
