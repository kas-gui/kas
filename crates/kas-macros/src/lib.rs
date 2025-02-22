// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS macros
//!
//! This crate extends [`impl-tools`](https://docs.rs/impl-tools/).

extern crate proc_macro;

use impl_tools_lib::{anon, autoimpl, scope};
use proc_macro::TokenStream;
use proc_macro_error2::{emit_call_site_error, emit_error, proc_macro_error};
use syn::parse_macro_input;
use syn::spanned::Spanned;

mod collection;
mod extends;
mod make_layout;
mod scroll_traits;
mod visitors;
mod widget;
mod widget_args;
mod widget_derive;

/// Implement `Default`
///
/// See [`impl_tools::impl_default`](https://docs.rs/impl-tools/0.6/impl_tools/attr.impl_default.html)
/// for full documentation.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn impl_default(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut toks = item.clone();
    match syn::parse::<impl_tools_lib::ImplDefault>(attr) {
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
/// See [`impl_tools::autoimpl`](https://docs.rs/impl-tools/0.6/impl_tools/attr.autoimpl.html)
/// for full documentation.
///
/// The following traits are supported:
///
/// | Path | *ignore* | *using* | *notes* |
/// |----- |--- |--- |--- |
/// | [`::core::borrow::Borrow<T>`] | - | borrow target | `T` is type of target field |
/// | [`::core::borrow::BorrowMut<T>`] | - | borrow target | `T` is type of target field |
/// | [`::core::clone::Clone`] | yes | - | ignored fields use `Default::default()` |
/// | [`::core::cmp::Eq`] | * | - | *allowed with `PartialEq` |
/// | [`::core::cmp::Ord`] | yes | - | |
/// | [`::core::cmp::PartialEq`] | yes | - | |
/// | [`::core::cmp::PartialOrd`] | yes | - | |
/// | [`::core::convert::AsRef<T>`] | - | ref target | `T` is type of target field |
/// | [`::core::convert::AsMut<T>`] | - | ref target | `T` is type of target field |
/// | [`::core::default::Default`] | - | - | [`macro@impl_default`] is a more flexible alternative |
/// | [`::core::fmt::Debug`] | yes | - | |
/// | [`::core::hash::Hash`] | yes | - | |
/// | [`::core::marker::Copy`] | * | - | *allowed with `Clone` |
/// | [`::core::ops::Deref`] | - | deref target | `type Target` is type of target field |
/// | [`::core::ops::DerefMut`] | - | deref target | `type Target` is type of target field |
#[proc_macro_attribute]
#[proc_macro_error]
pub fn autoimpl(attr: TokenStream, item: TokenStream) -> TokenStream {
    use autoimpl::ImplTrait;
    use scroll_traits::ImplScrollable;
    use std::iter::once;

    let mut toks = item.clone();
    match syn::parse::<autoimpl::Attr>(attr) {
        Ok(autoimpl::Attr::ForDeref(ai)) => toks.extend(TokenStream::from(ai.expand(item.into()))),
        Ok(autoimpl::Attr::ImplTraits(ai)) => {
            // We could use lazy_static to construct a HashMap for fast lookups,
            // but given the small number of impls a "linear map" is fine.
            let find_impl = |path: &syn::Path| {
                autoimpl::STD_IMPLS
                    .iter()
                    .cloned()
                    .chain(once(&ImplScrollable as &dyn ImplTrait))
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

const IMPL_SCOPE_RULES: [&dyn scope::ScopeAttr; 2] =
    [&scope::AttrImplDefault, &widget_args::AttrImplWidget];

fn find_attr(path: &syn::Path) -> Option<&'static dyn scope::ScopeAttr> {
    IMPL_SCOPE_RULES
        .iter()
        .cloned()
        .find(|rule| rule.path().matches(path))
}

/// Scope supporting `impl Self` and advanced attribute macros
///
/// This macro facilitates definition of a type (struct, enum or union) plus
/// implementations via `impl Self { .. }` syntax: `Self` is expanded to the
/// type's name, including generics and bounds (as defined on the type).
///
/// Caveat: `rustfmt` can not yet format contents (see
/// [rustfmt#5254](https://github.com/rust-lang/rustfmt/issues/5254),
/// [rustfmt#5538](https://github.com/rust-lang/rustfmt/pull/5538)).
///
/// See [`impl_tools::impl_scope`](https://docs.rs/impl-tools/0.6/impl_tools/macro.impl_scope.html)
/// for full documentation.
///
/// ## Special attribute macros
///
/// Additionally, `impl_scope!` supports special attribute macros evaluated
/// within its scope:
///
/// -   [`#[impl_default]`](macro@impl_default): implement [`Default`] using
///     field initializers (which are not legal syntax outside of `impl_scope!`)
/// -   [`#[widget]`](macro@widget): implement `kas::Widget` trait family
///
/// Note: matching these macros within `impl_scope!` does not use path
/// resolution. Using `#[impl_tools::impl_default]` would resolve the variant
/// of this macro which *doesn't support* field initializers.
#[proc_macro_error]
#[proc_macro]
pub fn impl_scope(input: TokenStream) -> TokenStream {
    let mut scope = parse_macro_input!(input as scope::Scope);
    scope.apply_attrs(find_attr);
    scope.expand().into()
}

/// Attribute to implement the `kas::Widget` family of traits
///
/// This may *only* be used within the [`impl_scope!`] macro.
///
/// Assists implementation of the [`Widget`], [`Events`], [`Layout`] and [`Tile`] traits.
/// Implementations of these traits are generated if missing or augmented with
/// missing method implementations.
///
/// This macro may inject methods into existing [`Layout`] / [`Tile`] / [`Events`] / [`Widget`] implementations.
/// This is used both to provide default implementations which could not be
/// written on the trait and to implement properties like `navigable`.
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
/// -   <code>Data = Type</code>: the `Widget::Data` associated type
/// -   <code>data_expr = expr</code>: a mapping expression for the derived
///     widget's input data; requires `derive` and `Data` arguments.
///     Inputs available to this expression are `self` and `data`.
/// -   <code>derive = self.<em>field</em></code> where
///     <code><em>field</em></code> is the name (or number) of a field:
///     enables "derive mode" ([see below](#derive)) over the given field
/// -   <code>navigable = <em>bool</em></code> — a quick implementation of
///     `Events::navigable`: whether this widget supports keyboard focus via
///     the <kbd>Tab</kbd> key (default is `false`)
/// -   <code>hover_highlight = <em>bool</em></code> — if true, generate
///     `Events::handle_hover` to request a redraw on focus gained/lost
/// -   <code>cursor_icon = <em>expr</em></code> — if used, generate
///     `Event::handle_hover`, calling `cx.set_hover_cursor(expr)`
/// -   <code>layout = <em>layout</em></code> — defines widget layout via an
///     expression; [see below for documentation](#layout)
///
/// The struct must contain a field of type `widget_core!()` (usually named
/// `core`). The macro `widget_core!()` is a placeholder, expanded by
/// `#[widget]` and used to identify the field used (any name may be used).
/// This type implements [`Default`] and [`Clone`], though the clone is not an
/// exact clone (cloned widgets must still be configured).
///
/// Assuming the deriving type is a `struct` or `tuple struct`, fields support
/// the following attributes:
///
/// -   `#[widget]`: marks the field as a [`Widget`] to be configured, enumerated by
///     [`Widget::get_child`] and included by glob layouts
/// -   `#[widget(expr)]`: the same, but maps the data reference type; `expr` is
///     an expression returning a reference to the child widget's input data;
///     available inputs are `self`, `data` (own input data) and `index`
///     (of the child).
///
/// ## Layout
///
/// Widget layout may be specified either by implementing the `Layout` trait or
/// via the `layout` property of `#[widget]`. The latter accepts the following
/// syntax, where _Layout_ is any of the below.
///
/// Using the `layout = ...;` property will also generate a corresponding
/// implementation of `Tile::nav_next`, with a couple of exceptions
/// (where macro-time analysis is insufficient to implement this method).
///
/// > [_Column_], [_Row_], [_List_] [_AlignedColumn_], [_AlignedRow_], [_Grid_],
/// > [_Float_], [_Frame_] :\
/// > &nbsp;&nbsp; These stand-alone macros are explicitly supported in this position.\
/// >
/// > _Single_ :\
/// > &nbsp;&nbsp; `self` `.` _Member_\
/// > &nbsp;&nbsp; A named child: `self.foo` (more precisely, this matches any
/// > expression starting `self`, and uses `&mut (#expr)`).
/// >
/// > _WidgetConstructor_ :\
/// > &nbsp;&nbsp; _Expr_\
/// > &nbsp;&nbsp; An expression yielding a widget, e.g.
/// > `Label::new("Hello world")`. The result must be an object of some type
/// > `W: Widget<Data = ()>`. This widget will be stored in a hidden field and
/// > is accessible through `Tile::get_child` but does not receive input data.
/// >
/// > _LabelLit_ :\
/// > &nbsp;&nbsp; _StrLit_\
/// > &nbsp;&nbsp; A string literal generates a label widget, e.g. "Hello
/// > world". This is an internal type without text wrapping.
///
/// Additional syntax rules (not layout items):
///
/// > _Member_ :\
/// > &nbsp;&nbsp; _Ident_ | _Index_\
/// > &nbsp;&nbsp; The name of a struct field or an index into a tuple struct.
///
/// ## Examples
///
/// A simple example is the
/// [`Frame`](https://docs.rs/kas-widgets/latest/kas_widgets/struct.Frame.html) widget:
///
/// ```ignore
/// impl_scope! {
///     /// A frame around content
///     #[derive(Clone, Default)]
///     #[widget{
///         layout = frame!(self.inner);
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
/// A simple row layout: `layout = row! [self.a, self.b];`
///
/// ## Derive
///
/// It is possible to derive from a field which is itself a widget, e.g.:
/// ```ignore
/// impl_scope! {
///     #[autoimpl(Deref, DerefMut using self.0)]
///     #[derive(Clone, Default)]
///     #[widget{ derive = self.0; }]
///     pub struct ScrollBarRegion<W: Widget>(ScrollBars<ScrollRegion<W>>);
/// }
/// ```
///
/// This is a special mode where most features of `#[widget]` are not
/// available; most notably, the deriving widget does not have its own `Id`.
///
/// ### A note on `Deref`
///
/// The "derive" example implements [`Deref`] over the inner widget. This is
/// acceptable for a simple wrapping "derive widget". It is not recommended to
/// implement [`Deref`] outside of derive mode (i.e. when the outer widget has
/// its own `Id`) due to the potential for method collision (e.g. `outer.id()`
/// may resolve to `outer.deref().id()` when the trait providing `fn id` is not
/// in scope, yet is available through a bound on the field).
///
/// ## Method modification
///
/// As a policy, this macro *may* inject code into user-defined methods of
/// `Widget` and its super traits, such that:
///
/// -   The modification cannot have harmful side effects (other than reported
///     errors).
/// -   All side effects observable outside of reported error cases must be
///     documented in the widget method documentation.
///
/// As an example, status checks are injected into some `Layout` methods to
/// enforce the expected call order of methods at runtime in debug builds.
///
/// ## Debugging
///
/// To inspect the output of this macro, set the environment variable
/// `KAS_DEBUG_WIDGET` to the name of the widget concerned, dump the output to
/// a temporary file and format. For example:
/// ```sh
/// KAS_DEBUG_WIDGET=Border cargo build > temp.rs
/// rustfmt temp.rs
/// ```
///
/// [`Widget`]: https://docs.rs/kas/latest/kas/trait.Widget.html
/// [`Widget::get_child`]: https://docs.rs/kas/latest/kas/trait.Widget.html#method.get_child
/// [`Layout`]: https://docs.rs/kas/latest/kas/trait.Layout.html
/// [`Tile`]: https://docs.rs/kas/latest/kas/trait.Tile.html
/// [`Events`]: https://docs.rs/kas/latest/kas/trait.Events.html
/// [`CursorIcon`]: https://docs.rs/kas/latest/kas/event/enum.CursorIcon.html
/// [`IsUsed`]: https://docs.rs/kas/latest/kas/event/enum.IsUsed.html
/// [`Deref`]: std::ops::Deref
/// [_Column_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.column.html
/// [_Row_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.row.html
/// [_List_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.list.html
/// [_Float_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.float.html
/// [_Frame_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.frame.html
/// [_Grid_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.grid.html
/// [_AlignedColumn_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.aligned_column.html
/// [_AlignedRow_]: https://docs.rs/kas-widgets/latest/kas_widgets/macro.aligned_row.html
#[proc_macro_attribute]
#[proc_macro_error]
pub fn widget(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must be used within impl_scope! { ... }");
    item
}

/// Construct a single-instance struct
///
/// Rust doesn't currently support [`impl Trait { ... }` expressions](https://github.com/canndrew/rfcs/blob/impl-trait-expressions/text/0000-impl-trait-expressions.md)
/// or implicit typing of struct fields. This macro is a **hack** allowing that.
///
/// Example:
/// ```
/// # use kas_macros as kas;
/// use std::fmt;
/// fn main() {
///     let world = "world";
///     let says_hello_world = kas::impl_anon! {
///         struct(&'static str = world);
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
/// -   **regular struct:** `ident: ty` (uses `Default` to construct value)
/// -   **tuple struct:** `ty = value`
/// -   **tuple struct:** `ty` (uses `Default` to construct value)
///
/// The field name, `ident`, may be `_` (anonymous field).
///
/// Refer to [examples](https://github.com/search?q=impl_anon+repo%3Akas-gui%2Fkas+path%3Aexamples&type=Code) for usage.
#[proc_macro_error]
#[proc_macro]
pub fn impl_anon(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as anon::Anon);
    for field in input.fields.iter_mut() {
        if matches!(&field.ty, syn::Type::Infer(_)) {
            let span = if let Some(ref ident) = field.ident {
                ident.span()
            } else if let Some((ref eq, _)) = field.assignment {
                eq.span()
            } else {
                // This is always available and should be the first choice,
                // but it may be synthesized (thus no useful span).
                // We can't test since Span::eq is unstable!
                field.ty.span()
            };
            emit_error!(span, "expected `: TYPE`");
        }
    }
    let mut scope = input.into_scope();
    scope.apply_attrs(find_attr);
    scope.expand().into()
}

/// Index of a child widget
///
/// This macro is usable only within an [`impl_scope!`]  macro using the
/// [`widget`](macro@widget) attribute.
///
/// Example usage: `widget_index![self.a]`. If `a` is a child widget (a field
/// marked with the `#[widget]` attribute), then this expands to the child
/// widget's index (as used by [`Widget::get_child`]). Otherwise, this is an error.
///
/// [`Widget::get_child`]: https://docs.rs/kas/latest/kas/trait.Widget.html#method.get_child
#[proc_macro_error]
#[proc_macro]
pub fn widget_index(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as visitors::UnscopedInput);
    input.into_token_stream().into()
}

/// Macro to set the `rect` stored in the widget core
///
/// Widgets have a hidden field of type [`Rect`] in their `widget_core!()`, used
/// to implement method [`Layout::rect`]. This macro assigns to that field.
///
/// This macro is usable only within the definition of `Layout::set_rect` within
/// an [`impl_scope!`] macro using the [`widget`](macro@widget) attribute.
///
/// The method `Layout::rect` will be generated if this macro is used by the
/// widget, otherwise a definition of the method must be provided.
///
/// Example usage:
/// ```ignore
/// fn set_rect(&mut self, _: &mut ConfigCx, rect: Rect, _: AlignHints) {
///     widget_set_rect!(rect);
/// }
/// ```
///
/// [`Rect`]: https://docs.rs/kas/latest/kas/geom/struct.Rect.html
/// [`Layout::rect`]: https://docs.rs/kas/latest/kas/trait.Layout.html#method.rect
#[proc_macro_error]
#[proc_macro]
pub fn widget_set_rect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as visitors::UnscopedInput);
    input.into_token_stream().into()
}

/// Generate an anonymous struct which implements [`kas::Collection`]
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `collection!` `[` _Items_<sup>\?</sup> `]`
/// >
/// > _Items_ :\
/// > &nbsp;&nbsp; (_Item_ `,`)<sup>\*</sup> _Item_ `,`<sup>\?</sup>
///
/// In this case, _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `collection!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// For example usage, see [`List`](https://docs.rs/kas/latest/kas/widgets/struct.List.html).
///
/// [`kas::Collection`]: https://docs.rs/kas/latest/kas/trait.Collection.html
/// [`align`]: https://docs.rs/kas/latest/kas/widgets/adapt/trait.AdaptWidget.html#method.align
/// [`pack`]: https://docs.rs/kas/latest/kas/widgets/adapt/trait.AdaptWidget.html#method.pack
#[proc_macro_error]
#[proc_macro]
pub fn collection(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as collection::Collection)
        .expand()
        .into()
}

/// Generate an anonymous struct which implements [`kas::CellCollection`]
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `collection!` `[` _ItemArms_<sup>\?</sup> `]`
/// >
/// > _ItemArms_ :\
/// > &nbsp;&nbsp; (_ItemArm_ `,`)<sup>\*</sup> _ItemArm_ `,`<sup>\?</sup>
/// >
/// > _ItemArm_ :\
/// > &nbsp;&nbsp; `(` _Column_ `,` _Row_ `)` `=>` _Item_
/// >
/// > _Column_, _Row_ :\
/// > &nbsp;&nbsp; _LitInt_ | ( _LitInt_ `..` `+` _LitInt_ ) | ( _LitInt_ `..`
/// > _LitInt_ ) | ( _LitInt_ `..=` _LitInt_ )
///
/// Here, _Column_ and _Row_ are selected via an index (from 0), a range of
/// indices, or a start + increment. For example, `2` = `2..+1` = `2..3` =
/// `2..=2` while `5..+2` = `5..7` = `5..=6`.
///
/// _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `collection!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// For example usage, see [`Grid`](https://docs.rs/kas/latest/kas/widgets/struct.Grid.html).
///
/// [`kas::CellCollection`]: https://docs.rs/kas/latest/kas/trait.CellCollection.html
/// [`align`]: https://docs.rs/kas/latest/kas/widgets/adapt/trait.AdaptWidget.html#method.align
/// [`pack`]: https://docs.rs/kas/latest/kas/widgets/adapt/trait.AdaptWidget.html#method.pack
#[proc_macro_error]
#[proc_macro]
pub fn cell_collection(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as collection::CellCollection)
        .expand()
        .into()
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
///
/// Note: this is a very limited macro which *only* supports `ThemeDraw`.
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
