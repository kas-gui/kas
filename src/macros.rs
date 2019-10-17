// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Library macros
//!
//! This module provides three important macros:
//!
//! -   [`derive(Widget)`] implements the [`Widget`] trait (including super-traits
//!     like [`Core`] and optionally [`Handler`]); this exists partly as a
//!     convenience, but **mostly because details of the [`Widget`] trait family
//!     are not part of the public API**, preventing manual implementation.
//! -   [`make_widget`] is a convenience macro to create a single instance of a
//!     custom widget type
//!
//! Note that these macros are defined in the external crate, `kas-macros`, only
//! because procedural macros must be defined in a special crate. The
//! `kas-macros` crate should not be used directly.
//!
//! Note further that these macros require gated functionality only available
//! in nightly `rustc` builds:
//! ```
//! #![feature(proc_macro_hygiene)]
//! ```
//!
//! [`make_widget`]: #the-make_widget-macro
//! [`derive(Widget)`]: #the-derivewidget-macro
//!
//!
//! ## The `derive(Widget)` macro
//!
//! The [`Widget`] trait requires multiple base traits to be implemented:
//! [`Core`] and [`Layout`]. These base traits should be considered
//! implementation details and not used directly; this macro therefore
//! implements both base traits and [`Widget`] directly.
//!
//! Additionally, widgets are usually required to implement the [`Handler`]
//! trait. If (and only if) the deriving struct is marked with a
//! `#[handler]` attribute, the [`Handler`] trait will also be implemented.
//! Note that it is also possible to implement this trait manually.
//!
//! ### Type attributes
//!
//! This `derive` attribute may only be used on structs. This struct must have
//! a `#[widget]` attribute and may have a `#[handler]` attribute, as follows.
//!
//! ```notest
//! #[widget(class = Class::X, ...)]
//! #[handler]
//! #[derive(Clone, Debug, Widget)]
//! struct MyWidget {
//!     ...
//! }
//! ```
//!
//! The `#[widget]` attribute on the struct supports the following arguments:
//!
//! -   `class = ...` (required) — an expression yielding the widget's [`Class`]
//! -   `layout = ...` (optional) — see below
//!
//! If the `layout` argument is missing, the [`Layout`] trait must be
//! implemented manually. If present, this trait will be implemented depending
//! on the `layout` argument's value:
//!
//! -   `empty` — the widget displays no content and has zero size,
//!     except when expanded to fill empty space
//! -   `derive` — this is a simple widget with no children; content and
//!     dimensions are derived from the toolkit based on the widget's class
//! -   `single` — the widget wraps a single child, with no border or margin
//!
//! If there is a `#[handler]` attribute on the struct, then the [`Handler`]
//! trait will be implemented. This attribute accepts the following arguments:
//!
//! -   `msg = ...` — the [`Handler::Msg`] associated type; defaults to `()`
//! -   `generics = < X, Y, ... > where CONDS` — see below
//!
//! Commonly the [`Handler`] implementation requires extra bounds on generic
//! types, and sometimes also additional type parameters; the `generics`
//! argument allows this. This argument is optional and if present must be the
//! last argument. Note that the generic types and bounds given are *added to*
//! the generics defined on the struct itself.
//!
//! ### Fields
//!
//! One struct field must be marked with `#[core]` and implement the [`Core`]
//! trait; usually this field has the specification `#[core] core: CoreData`.
//!
//! A `#[widget]` attribute is used to denote fields as child widgets. This
//! attribute accepts the following optional arguments, for use with `grid`
//! layouts and for handlers:
//!
//! -   `col = ...` — grid column, from left (defaults to 0)
//! -   `row = ...` — grid row, from top (defaults to 0)
//! -   `cspan = ...` — number of columns to span (defaults to 1)
//! -   `rspan = ...` — number of rows to span (defaults to 1)
//! -   `handler = ...` — the name (`f`) of a method defined on this type which
//!     handles a message from the child (type `M`) and converts it to the
//!     appropriate response type for this widget (`R`); this method should have
//!     signature `fn f(&mut self, tk: &TkWidget, msg: M) -> R`.
//!
//!
//! ### Examples
//!
//! A short example, without an implementation for [`Handler`] (which could
//! still be implemented separately):
//!
//! ```
//! use kas::{Widget, Class, CoreData};
//! use kas::macros::Widget;
//!
//! #[widget(class = Class::Window, layout = single)]
//! #[derive(Debug, Widget)]
//! struct MyWidget<W: Widget> {
//!     #[core] core: CoreData,
//!     #[widget] child: W,
//! }
//! ```
//!
//! A longer example, including derivation of the [`Handler`] trait:
//!
//! ```
//! use kas::{Widget, Class, CoreData, TkWidget};
//! use kas::event::{Handler, Response, err_unhandled};
//! use kas::macros::Widget;
//!
//! #[derive(Debug)]
//! enum ChildMessage { A }
//!
//! #[widget(class = Class::Container, layout = single)]
//! #[handler(generics = <> where W: Handler<Msg = ChildMessage>)]
//! #[derive(Debug, Widget)]
//! struct MyWidget<W: Widget> {
//!     #[core] core: CoreData,
//!     #[widget(handler = handler)] child: W,
//! }
//!
//! impl<W: Widget> MyWidget<W> {
//!     fn handler(&mut self, tk: &dyn TkWidget, msg: ChildMessage) -> Response<()> {
//!         match msg {
//!             ChildMessage::A => { println!("handling ChildMessage::A"); }
//!         }
//!         Response::None
//!     }
//! }
//! ```
//!
//!
//! ## The `make_widget` macro
//!
//! This macro supports widgets of the following classes:
//!
//! -   Container
//! -   Frame
//!
//! This exists purely to save you some typing. You could instead make your own
//! struct, derive `Widget` (with attributes to enable Core, Layout and Widget
//! implementation), manually implement `event::Handler`, and instantiate an
//! object.
//!
//! Syntax should match the following Backus-Naur Form:
//!
//! ```bnf
//! <input>     ::= <class> "=>" <msg> ";" <fields> ";" <funcs>
//! <class>     ::= "container" "(" <layout> ")" | "frame"
//! <layout>    ::= "single" | "horizontal" | "vertical" | "grid"
//! <msg>  ::= <type>
//! <fields>    ::= "" | <field> | <field> "," <fields>
//! <field>     ::= <w_attr> <opt_ident> <field_ty> = <expr>
//! <opt_ident> ::= "_" | <ident>
//! <field_ty>  ::= "" | ":" <type> | ":" impl <bound> | "->" <type> | ":" impl <bound> "->" <type>
//! <w_attr>    ::= "" | "#" "[" <widget> <w_params> "]"
//! <w_params>  ::= "" | "(" <w_args> ")"
//! <w_args>    ::= <w_arg> | <w_arg> "," <w_args>
//! <w_arg>     ::= <pos_arg> "=" <lit> | "handler" = <ident>
//! <pos_arg>   ::= "col" | "row" | "cspan" | "rspan"
//! <funcs>     ::= "" | <func> <funcs>
//! ```
//! where `<type>` is a type expression, `<expr>` is a (value) expression,
//! `<ident>` is an identifier, `<lit>` is a literal, `<path>` is a path,
//! `<bound>` is a trait object bound, and
//! `<func>` is a Rust method definition. `""` is the empty string (i.e. nothing).
//!
//! The effect of this macro is to create an anonymous struct with the above
//! fields (plus an implicit `core`), implement [`Core`], [`Layout`], [`Widget`]
//! and [`Handler`] (with the specified `<msg>` type), implement the
//! additional `<funcs>` listed on this type, then construct and return an
//! instance using the given value expressions to initialise each field.
//!
//! Each field is considered a child widget if the `#[widget]` attribute is
//! present, or a simple data field otherwise. The specification of this
//! attribute is identical to that used when deriving `Widget`.
//!
//! The `layout` specifier should be self-explanatory, with the exception of
//! `grid`, where each widget's position must be specified via attribute
//! arguments (e.g. `#[widget(col=1, row=2)]`). The `col` and `row` parameters
//! both default to 0, while `cspan` and `rspan` (column and row spans) both
//! default to 1.
//!
//! Fields may have an identifier or may be anonymous (via usage of `_`). This
//! is often convenient for child widgets which don't need to be referred to.
//!
//! Fields may have an explicit type (`ident : type = ...`), or the type may be
//! skipped, or (for widgets only) just the message type can be specified via
//! `ident -> type = ...`. Note that some type specification is usually needed
//! when referring to the field later.
//!
//! Optionally, a message handler may be specified for child widgets via
//! `#[widget(handler = f)] ident = value` where `f` is a method defined on the
//! anonymous struct with signature `fn f(&mut self, tk: &TkWidget, msg: M) -> R`
//! where `M` is the type of response received from the child widget, and `R` is
//! the type of response sent from this widget.
//!
//! ### Example
//!
//! ```
//! #![feature(proc_macro_hygiene)]
//!
//! use kas::control::TextButton;
//! use kas::macros::{make_widget};
//!
//! enum OkCancel {
//!     Ok,
//!     Cancel,
//! }
//!
//! let button_box = make_widget!{
//!     container(horizontal) => OkCancel;
//!     struct {
//!         #[widget] _ = TextButton::new_on("Ok", || OkCancel::Ok),
//!         #[widget] _ = TextButton::new_on("Cancel", || OkCancel::Cancel),
//!     }
//! };
//! ```
//!
//!
//! [`Core`]: crate::Core
//! [`Layout`]: crate::Layout
//! [`Widget`]: crate::Widget
//! [`Handler`]: crate::event::Handler
//! [`Class`]: crate::Class
//! [`CoreData`]: crate::CoreData
//! [`Handler::Msg`]: ../kas/event/trait.Handler.html#associatedtype.Msg

pub use kas_macros::{make_widget, Widget};
