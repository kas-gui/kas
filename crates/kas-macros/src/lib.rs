// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS macros
//!
//! This crate extends [impl-tools](https://crates.io/crates/impl-tools).

#![recursion_limit = "128"]
#![allow(clippy::let_and_return)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::collapsible_if)]

extern crate proc_macro;

use impl_tools_lib::autoimpl;
use impl_tools_lib::{AttrImplDefault, ImplDefault, Scope, ScopeAttr};
use proc_macro::TokenStream;
use proc_macro_error::{emit_call_site_error, proc_macro_error};
use syn::parse_macro_input;

mod args;
mod class_traits;
mod extends;
mod impl_singleton;
mod make_layout;
mod widget;
mod widget_index;

/// Implement `Default`
///
/// See [`impl_tools::impl_default`](https://docs.rs/impl-tools/0.3/impl_tools/attr.impl_default.html)
/// for full documentation.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn impl_default(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut toks = item.clone();
    match syn::parse::<ImplDefault>(attr) {
        Ok(attr) => toks.extend(TokenStream::from(attr.expand(item.into()))),
        Err(err) => {
            emit_call_site_error!(err);
            // Since this form of invocation only adds implementations, we can
            // safely output the original item, thus reducing secondary errors.
        }
    }
    toks
}

/// A variant of the standard `derive` macro
///
/// See [`impl_tools::autoimpl`](https://docs.rs/impl-tools/0.3/impl_tools/attr.autoimpl.html)
/// for full documentation.
///
/// The following traits are supported:
///
/// | Name | Path | *ignore* | *using* |
/// |----- |----- |--- |--- |
/// | `Clone` | `::std::clone::Clone` | initialized with `Default::default()` | - |
/// | `Debug` | `::std::fmt::Debug` | field is not printed | - |
/// | `Default` | `::std::default::Default` | - | - |
/// | `Deref` | `::std::ops::Deref` | - | deref target |
/// | `DerefMut` | `::std::ops::DerefMut` | - | deref target |
/// | `HasBool` | `::kas::class::HasBool` | - | deref target |
/// | `HasStr` | `::kas::class::HasStr` | - | deref target |
/// | `HasString` | `::kas::class::HasString` | - | deref target |
/// | `SetAccel` | `::kas::class::SetAccel` | - | deref target |
/// | `class_traits` | each `kas::class` trait | - | deref target |
///
/// *Ignore:* trait supports ignoring fields (e.g. `#[autoimpl(Debug ignore self.foo)]`).
///
/// *Using:* trait requires a named field to "use" (e.g. `#[autoimpl(Deref using self.foo)]`).
///
/// ### Examples
///
/// Implement all `kas::class` trait over a wrapper type:
/// ```no_test
/// # use kas_macros::autoimpl;
/// #[autoimpl(class_traits using self.0 where T: trait)]
/// struct WrappingType<T>(T);
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn autoimpl(attr: TokenStream, item: TokenStream) -> TokenStream {
    use autoimpl::ImplTrait;
    use class_traits::ImplClassTraits;
    use std::iter::once;

    let mut toks = item.clone();
    match syn::parse::<autoimpl::Attr>(attr) {
        Ok(autoimpl::Attr::ForDeref(ai)) => toks.extend(TokenStream::from(ai.expand(item.into()))),
        Ok(autoimpl::Attr::ImplTraits(ai)) => {
            // We could use lazy_static to construct a HashMap for fast lookups,
            // but given the small number of impls a "linear map" is fine.
            let find_impl = |path: &syn::Path| {
                (autoimpl::STD_IMPLS.iter())
                    .chain(class_traits::CLASS_IMPLS.iter())
                    .cloned()
                    .chain(once(&ImplClassTraits as &dyn ImplTrait))
                    .find(|impl_| impl_.path().matches_ident_or_path(path))
            };
            toks.extend(TokenStream::from(ai.expand(item.into(), find_impl)))
        }
        Err(err) => {
            emit_call_site_error!(err);
            // Since autoimpl only adds implementations, we can safely output
            // the original item, thus reducing secondary errors.
        }
    }
    toks
}

const IMPL_SCOPE_RULES: [&'static dyn ScopeAttr; 2] = [&AttrImplDefault, &widget::AttrImplWidget];

/// Implementation scope
///
/// See [`impl_tools::impl_scope`](https://docs.rs/impl-tools/0.3/impl_tools/macro.impl_scope.html)
/// for full documentation.
#[proc_macro_error]
#[proc_macro]
pub fn impl_scope(input: TokenStream) -> TokenStream {
    let mut scope = parse_macro_input!(input as Scope);
    scope.apply_attrs(|path| {
        IMPL_SCOPE_RULES
            .iter()
            .cloned()
            .find(|rule| rule.path().matches(path))
    });
    scope.expand().into()
}

/// Attribute to implement the `kas::Widget` family of traits
///
/// This may *only* be used within the [`impl_scope!`] macro.
///
/// Implements the [`WidgetCore`] and [`Widget`] traits for the deriving type.
/// Implements the [`WidgetChildren`] and [`Layout`]
/// traits only if not implemented explicitly within the
/// defining [`impl_scope!`].
///
/// This macro may inject methods into existing [`Layout`] / [`Widget`] implementations.
/// This is used both to provide default implementations which could not be
/// written on the trait and to implement properties like `key_nav`.
/// (In the case of multiple implementations of the same trait, as used for
/// specialization, only the first implementation of each trait is extended.)
///
/// ## Syntax
///
/// > _WidgetAttr_ :\
/// > &nbsp;&nbsp; `#` `[` `widget` _WidgetAttrArgs_? `]`
/// >
/// > _WidgetAttrArgs_ :\
/// > &nbsp;&nbsp; `{` (_WidgetAttrArg_ `;`) * `}`
///
/// Supported arguments (_WidgetAttrArg_) are:
///
/// -   <code>derive = self.<em>field</em></code> where
///     <code><em>field</em></code> is the name (or number) of a field:
///     enables "derive mode" ([see below](#derive)) over the given field
/// -   <code>key_nav = <em>bool</em></code> — a quick implementation of
///     `Widget::key_nav`: whether this widget supports keyboard focus via
///     the <kbd>Tab</kbd> key (default is `false`)
/// -   <code>hover_highlight = <em>bool</em></code> — if true, then match
///     `Event::MouseHover` and `Event::LostMouseHover`, requesting redraw and
///     returning `Response::Used`
/// -   <code>cursor_icon = <em>expr</em></code> — if used, then match
///     `Event::MouseHover`, calling `mgr.set_cursor_icon(expr)`
/// -   <code>layout = <em>layout</em></code> — defines widget layout via an
///     expression; [see below for documentation](#layout)
///
/// The struct must contain a field of type `widget_core!()` (usually named
/// `core`). The macro `widget_core!()` is a placeholder, expanded by
/// `#[widget]` and used to identify the field used (name may be anything).
/// This field *may* have type [`CoreData`] or may be a generated
/// type; either way it has fields `id: WidgetId` (assigned by
/// `Widget::pre_configure`) and `rect: Rect` (usually assigned by
/// `Widget::set_rect`). It may contain additional fields for layout data. The
/// type supports `Debug`, `Default` and `Clone` (although `Clone` actually
/// default-initializes all fields other than `rect` since clones of widgets
/// must themselves be configured).
///
/// Assuming the deriving type is a `struct` or `tuple struct`, fields support
/// the following attributes:
///
/// -   `#[widget]`: marks the field as a [`Widget`] to be configured, enumerated by
///     [`WidgetChildren`] and included by glob layouts
///
/// ## Layout
///
/// Widget layout may be specified either by implementing the `Layout` trait or
/// via the `layout` property of `#[widget]`. The latter accepts the following
/// syntax, where _Layout_ is any of the below.
///
/// Using the `layout = ...;` property will also generate a corresponding
/// implementation of `Widget::nav_next`, with a couple of exceptions
/// (where macro-time analysis is insufficient to implement this method).
///
/// > _Single_ :\
/// > &nbsp;&nbsp; `self` `.` _Member_\
/// > &nbsp;&nbsp; A named child: `self.foo` (more precisely, this matches any expression starting `self`, and uses `&mut (#expr)`)
/// >
/// > _List_ :\
/// > &nbsp;&nbsp; ( `column` | `row` | `list` `(` _Direction_ `)` ) _Storage_? `:` `[` ( _Layout_ `,`? ) * `]`\
/// > &nbsp;&nbsp; A list of children, e.g. `row: ["Foo", self.foo]` or `list(up): ..` or `list(self.direction()): ..`
/// >
/// > _AlignedList_ :\
/// > &nbsp;&nbsp; ( `aligned_column` | `aligned_row` ) _Storage_? `:` `[` ( _Layout_ `,`? ) * `]`\
/// > &nbsp;&nbsp; Inner component must be `row` or `column`, e.g.: `aligned_column: [row: ["One", "Two"], row: ["Three", "Four"]]`. This is syntactic sugar for a grid layout.
/// >
/// > _Slice_ :\
/// > &nbsp;&nbsp; `slice` `(` _Direction_ `)` _Storage_? `:` `self` `.` _Member_\
/// > &nbsp;&nbsp; A field with type `[W]` for some `W: Widget`
/// >
/// > _Grid_ :\
/// > &nbsp;&nbsp; `grid` _Storage_? `:` `{` _GridCell_* `}`\
/// > &nbsp;&nbsp; A two-dimensional layout, supporting cell spans, defined via a list of cells (see _GridCell_ below).
///
/// > _Float_ :\
/// > &nbsp;&nbsp; _float_ `:` `[` ( _Layout_ `,`? ) * `]`\
/// > &nbsp;&nbsp; A stack of overlapping elements, top-most first.
///
/// > _Align_ :\
/// > &nbsp;&nbsp; `align` `(` _AlignType_ ( `,` _AlignType_ )? `)` `:` _Layout_\
/// > &nbsp;&nbsp; Applies some alignment to a sub-layout, e.g. `align(top): self.foo`. Two-dimensional alignment is possible but must be horizontal first, e.g. `align(left, top): ..`
/// >
/// > _Frame_ :\
/// > &nbsp;&nbsp; `frame` ( `(` _Expr_ `)` )? _Storage_? `:` _Layout_\
/// > &nbsp;&nbsp; Adds a frame of type _Expr_ around content, defaulting to `FrameStyle::Frame`.
/// >
/// > _Button_ :\
/// > &nbsp;&nbsp; `button` ( `(` _Expr_ `)` ) ? _Storage_? `:` _Layout_\
/// > &nbsp;&nbsp; Adds a button frame (optionally with color _Expr_) around content.
/// >
/// > _Widget_ :\
/// > &nbsp;&nbsp; _ExprStartingUpperCase_\
/// > &nbsp;&nbsp; An expression yielding a widget, e.g. `Label::new("Hello world")`. The result must be an object of some type `W: Widget`. Since the expression must start with an upper case letter it is necessary to bring the type into scope first, e.g. `use kas::widgets::Label;`.
/// >
/// > _Label_ :\
/// > &nbsp;&nbsp; _StrLit_\
/// > &nbsp;&nbsp; A string literal generates a label widget, e.g. "Hello world". This is an internal type without text wrapping.
/// >
/// > _NonNavigable_ :\
/// > &nbsp;&nbsp; `non_navigable` `:` _Layout_ \
/// > &nbsp;&nbsp; Does not affect layout. Specifies that the content is excluded from tab-navigation order.
///
/// Additional syntax rules (not layout items):
///
/// > _Member_ :\
/// > &nbsp;&nbsp; _Ident_ | _Index_\
/// > &nbsp;&nbsp; The name of a struct field or an index into a tuple struct.
/// >
/// > _Direction_ :\
/// > &nbsp;&nbsp; `left` | `right` | `up` | `down` | _Expr_
/// >
/// > _GridCell_ :\
/// > &nbsp;&nbsp; _CellRange_ `,` _CellRange_ `:` _Layout_\
/// > &nbsp;&nbsp; Cell location in the order `(col, row)`, e.g.: `1, 0: self.foo`. Spans are specified via range syntax, e.g. `0..2, 1: self.bar`.
/// >
/// > _CellRange_ :\
/// > &nbsp;&nbsp; _LitInt_ ( `..` `+`? _LitInt_ )?
/// >
/// > _AlignType_ :\
/// > &nbsp;&nbsp; `default` | `center` | `stretch` | `top` | `bottom` | `left` | `right`
/// >
/// > _Storage_ :\
/// > &nbsp;&nbsp; `'` _Ident_\
/// > &nbsp;&nbsp; Used to explicitly name the storage used by a generated widget or layout; for example `row 'x: ["A", "B", "C"]` will add a field `x: R` where `R: RowStorage` within the generated `widget_core!()`. If omitted, the field name will be anonymous (generated).
///
/// ## Examples
///
/// A simple example is the
/// [`Frame`](https://docs.rs/kas-widgets/latest/kas_widgets/struct.Frame.html) widget:
///
/// ```ignore
/// impl_scope! {
///     /// A frame around content
///     #[autoimpl(Deref, DerefMut using self.inner)]
///     #[autoimpl(class_traits using self.inner where W: trait)]
///     #[derive(Clone, Debug, Default)]
///     #[widget{
///         layout = frame(kas::theme::FrameStyle::Frame): self.inner;
///     }]
///     pub struct Frame<W: Widget> {
///         core: widget_core!(),
///         #[widget]
///         pub inner: W,
///     }
///
///     impl Self {
///         /// Construct a frame
///         #[inline]
///         pub fn new(inner: W) -> Self {
///             Frame {
///                 core: Default::default(),
///                 inner,
///             }
///         }
///     }
/// }
/// ```
///
/// A simple row layout: `layout = row: [self.a, self.b];`
///
/// Grid cells are defined by `row, column` ranges, where the ranges are either
/// a half-open range or a single number (who's end is implicitly `start + 1`).
///
/// ```ignore
/// layout = grid: {
///     0..2, 0: self.merged_title;
///     0, 1: self.a;
///     1, 1: self.b;
///     1, 2: self.c;
/// };
/// ```
///
/// ## Derive
///
/// It is possible to derive from a field which is itself a widget, e.g.:
/// ```ignore
/// impl_scope! {
///     #[autoimpl(Deref, DerefMut using self.0)]
///     #[derive(Clone, Debug, Default)]
///     #[widget{ derive = self.0; }]
///     pub struct ScrollBarRegion<W: Widget>(ScrollBars<ScrollRegion<W>>);
/// }
/// ```
///
/// This is a special mode where most features of `#[widget]` are not
/// available. A few may still be used: `key_nav`, `hover_highlight`,
/// `cursor_icon`. Additionally, it is currently permitted to implement
/// [`WidgetChildren`], [`Layout`] and [`Widget`] traits manually (this option
/// may be removed in the future if not deemed useful).
///
/// [`Widget`]: https://docs.rs/kas/0.11/kas/trait.Widget.html
/// [`WidgetCore`]: https://docs.rs/kas/0.11/kas/trait.WidgetCore.html
/// [`WidgetChildren`]: https://docs.rs/kas/0.11/kas/trait.WidgetChildren.html
/// [`Layout`]: https://docs.rs/kas/0.11/kas/trait.Layout.html
/// [`CursorIcon`]: https://docs.rs/kas/0.11/kas/event/enum.CursorIcon.html
/// [`Response`]: https://docs.rs/kas/0.11/kas/event/enum.Response.html
/// [`CoreData`]: https://docs.rs/kas/0.11/kas/struct.CoreData.html
#[proc_macro_attribute]
#[proc_macro_error]
pub fn widget(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must be used within impl_scope! { ... }");
    item
}

/// Create a widget singleton
///
/// Rust doesn't currently support [`impl Trait { ... }` expressions](https://github.com/canndrew/rfcs/blob/impl-trait-expressions/text/0000-impl-trait-expressions.md)
/// or implicit typing of struct fields. This macro is a **hack** allowing that.
///
/// Example:
/// ```
/// use std::fmt;
/// use kas_macros::impl_singleton;
/// fn main() {
///     let world = "world";
///     let says_hello_world = impl_singleton! {
///         struct(impl fmt::Display = world);
///         impl fmt::Display for Self {
///             fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
///                 write!(f, "hello {}", self.0)
///             }
///         }
///     };
///     assert_eq!(format!("{}", says_hello_world), "hello world");
/// }
/// ```
///
/// That is, this macro creates an anonymous struct type (must be a struct),
/// which may have trait implementations, then creates an instance of that
/// struct.
///
/// Struct fields may have a fixed type or may be generic. Syntax is as follows:
///
/// -   **regular struct:** `ident: ty = value`
/// -   **regular struct:** `ident: ty` — uses `Default` to construct value
/// -   **regular struct:** `ident = value` — type is generic without bounds
/// -   **tuple struct:** `ty = value`
/// -   **tuple struct:** `ty`
/// -   **tuple struct:** `_ = value`
///
/// The `ident` may be `_` (anonymous field).
///
/// The `ty` expression may be replaced with one of the following:
///
/// -   `impl Trait` (or `impl A + B + C`) — type is generic with given bounds
/// -   `for<'a, T> std::borrow::Cow<'a, T>` — a generic with locally declared
///     type parameters (note: parameter names are made unique)
///
/// As a special rule, any field using the `#[widget]` attribute and without a
/// fixed type has the `::kas::Widget` trait bound applied.
///
/// Refer to [examples](https://github.com/search?q=impl_singleton+repo%3Akas-gui%2Fkas+path%3Aexamples&type=Code) for usage.
#[proc_macro_error]
#[proc_macro]
pub fn impl_singleton(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as args::ImplSingleton);
    impl_singleton::impl_singleton(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Index of a child widget
///
/// This macro is usable only within an [`impl_scope!`]  macro using the
/// [`widget`](macro@widget) attribute.
///
/// Example usage: `widget_index![self.a]`. If `a` is a child widget (a field
/// marked with the `#[widget]` attribute), then this expands to the child
/// widget's index (as used by [`WidgetChildren`]). Otherwise, this is an error.
///
/// [`WidgetChildren`]: https://docs.rs/kas/0.11/kas/trait.WidgetChildren.html
#[proc_macro_error]
#[proc_macro]
pub fn widget_index(input: TokenStream) -> TokenStream {
    let input2 = input.clone();
    let _ = parse_macro_input!(input2 as widget_index::BaseInput);
    input
}

/// A trait implementation is an extension over some base
///
/// Usage as follows:
/// ```ignore
/// #[extends(ThemeDraw, base = self.base())]
/// impl ThemeDraw for Object {
///     // All methods not present are implemented automatically over
///     // `self.base()`, which mut return an object implementing ThemeDraw
/// }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn extends(attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse::<extends::Extends>(attr) {
        Ok(extends) => match extends.extend(item.into()) {
            Ok(result) => result.into(),
            Err(err) => {
                emit_call_site_error!(err);
                TokenStream::new()
            }
        },
        Err(err) => {
            emit_call_site_error!(err);
            item
        }
    }
}
