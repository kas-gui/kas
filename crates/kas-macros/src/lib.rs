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
/// See [`impl_tools::impl_default`](https://docs.rs/impl-tools/latest/impl_tools/attr.impl_default.html)
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
/// See [`impl_tools::autoimpl`](https://docs.rs/impl-tools/latest/impl_tools/attr.autoimpl.html)
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

const IMPL_SCOPE_RULES: [&dyn scope::ScopeAttr; 3] = [
    &scope::AttrImplDefault,
    &widget_args::AttrImplWidget,
    &widget_derive::AttrDeriveWidget,
];

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
/// Note: prefer [`macro@impl_self`] over this macro unless using
/// [`macro@impl_default`]. This macro will be removed once
/// [RFC 3681](https://github.com/rust-lang/rfcs/pull/3681) (default field
/// values) is stable in this crate's MSRV.
///
/// See [`impl_tools::impl_scope`](https://docs.rs/impl-tools/latest/impl_tools/macro.impl_scope.html)
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
/// resolution. Using `#[kas_macros::impl_default]` would resolve the variant
/// of this macro which *doesn't support* field initializers.
#[proc_macro_error]
#[proc_macro]
pub fn impl_scope(input: TokenStream) -> TokenStream {
    let mut scope = parse_macro_input!(input as scope::Scope);
    scope.apply_attrs(find_attr);
    scope.expand().into()
}

fn find_impl_self_attrs(path: &syn::Path) -> Option<&'static dyn scope::ScopeAttr> {
    use scope::ScopeAttr;
    if widget_args::AttrImplWidget.path().matches(path) {
        Some(&widget_args::AttrImplWidget)
    } else if widget_derive::AttrDeriveWidget.path().matches(path) {
        Some(&widget_derive::AttrDeriveWidget)
    } else {
        None
    }
}

/// Implement a type with `impl Self` syntax
///
/// This attribute macro supports a type (struct, enum, type alias or union)
/// definition plus associated `impl` items within a `mod`.
///
/// Macro expansion discards the `mod` entirely, placing all contents into the
/// outer scope. This simplifies privacy rules in many use-cases, and highlights
/// that the usage of `mod` is purely a hack to make the macro input valid Rust
/// syntax (and thus compatible with `rustfmt`).
///
/// ## Special attribute macros
///
/// Additionally, `#[impl_self]` supports special attribute macros evaluated
/// within its scope:
///
/// -   [`#[widget]`](macro@widget): implement `kas::Widget` trait family
///
/// ## Syntax
///
/// > _ImplSelf_ :\
/// > &nbsp;&nbsp; `#[impl_self]` `mod` _Name_ `{` _ScopeItem_ _ItemImpl_ * `}`
/// >
/// > _ScopeItem_ :\
/// > &nbsp;&nbsp; _ItemEnum_ | _ItemStruct_ | _ItemType_ | _ItemUnion_
///
/// Here, _ItemEnum_, _ItemStruct_, _ItemType_ and _ItemUnion_ are `enum`,
/// `struct`, `type` alias and `union` definitions respectively. Whichever of
/// these is used, it must match the module name _Name_.
///
/// _ItemImpl_ is an `impl` item. It may use the standard implementation syntax
/// (e.g. `impl Debug for MyType { .. }`) or `impl Self` syntax (see below).
///
/// The `mod` may not contain any other items, except `doc` items (documentation
/// on the module itself is ignored in favour of documentation on the defined
/// type) and attributes (which apply as usual).
///
/// ### `impl Self` syntax
///
/// `impl Self` "syntax" is syntactically-valid (but not semantically-valid)
/// Rust syntax for writing inherent and trait `impl` blocks:
///
/// -   `impl Self { ... }` — an inherent `impl` item on the defined type
/// -   `impl Debug for Self { ... }` — a trait `impl` item on the defined type
///
/// Generic parameters and bounds are copied from the type definition.
/// Additional generic parameters may be specified; these extend the list of
/// generic parameters on the type itself, and thus must have distinct names.
/// Additional bounds (where clauses) may be specified; these extend the list of
/// bounds on the type itself.
///
/// ## Example
///
/// ```
/// #[kas_macros::impl_self]
/// mod Pair {
///     /// A pair of values of type `T`
///     pub struct Pair<T>(T, T);
///
///     impl Self {
///         pub fn new(a: T, b: T) -> Self {
///             Pair(a, b)
///         }
///     }
///
///     impl Self where T: Clone {
///         pub fn splat(a: T) -> Self {
///             let b = a.clone();
///             Pair(a, b)
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn impl_self(attr: TokenStream, input: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(attr as scope::ScopeModAttrs);
    let mut scope = parse_macro_input!(input as scope::ScopeMod).contents;
    scope.apply_attrs(find_impl_self_attrs);
    scope.expand().into()
}

/// Attribute to implement the `kas::Widget` family of traits
///
/// This may *only* be used within the [`macro@impl_self`], [`impl_scope!`]
/// and [`impl_anon!`] macros. It does not need to be imported (it is resolved
/// by the afore-mentioned macros).
///
/// Assists implementation of the [`Widget`], [`Events`], [`Layout`] and [`Tile`] traits.
/// Implementations of these traits are generated if missing or augmented with
/// missing method implementations.
///
/// This macro may inject methods into existing [`Layout`] / [`Tile`] /
/// [`Events`] / [`Widget`] implementations.
/// (In the case of multiple implementations of the same trait, as used for
/// specialization, only the first implementation of each trait is extended.)
///
/// See also the [`macro@layout`] attribute which assists in implementing
/// [`Layout`].
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
/// -   `#[widget = expr]`: an alternative way of writing the above
///
/// ## Examples
///
/// A simple example is the
/// [`Frame`](https://docs.rs/kas-widgets/latest/kas_widgets/struct.Frame.html) widget:
///
/// ```ignore
/// #[impl_self]
/// mod Frame {
///     /// A frame around content
///     #[derive(Clone, Default)]
///     #[widget]
///     #[layout(frame!(self.inner))]
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
#[proc_macro_attribute]
#[proc_macro_error]
pub fn widget(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must be used within scope of #[impl_self], impl_scope! or impl_anon!");
    item
}

/// Provide a default implementation of the [`Layout`] trait for a widget
///
/// The [`macro@widget`] macro uses this attribute to implement
/// [`MacroDefinedLayout`] for the widget, then adjusts the default
/// implementations of each [`Layout`] method to call the corresponding
/// [`MacroDefinedLayout`] method.
///
/// This attribute may *only* appear after the [`macro@widget`] attribute (it is
/// not a stand-alone macro). It does not need to be imported (it is resolved by
/// [`macro@widget`]).
///
/// ## Layout
///
/// Widget layout may be specified by implementing the `Layout` trait and/or
/// with a `#[layout(...)]` attribute (this must appear after `#[widget]` on the
/// type definition). The latter accepts the following
/// syntax, where _Layout_ is any of the below.
///
/// Using the `#[layout]` attribute will also generate a corresponding
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
/// [`Layout`]: https://docs.rs/kas/latest/kas/trait.Layout.html
/// [`MacroDefinedLayout`]: https://docs.rs/kas/latest/kas/trait.MacroDefinedLayout.html
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
pub fn layout(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must follow use of #[widget]");
    item
}

/// Derive the [`Widget`] family of traits
///
/// This may *only* be used within the [`macro@impl_self`], [`impl_scope!`]
/// and [`impl_anon!`] macros.
///
/// This macro derives a [`Widget`] implementation from the inner field
/// annotated with `#[widget]`.
///
/// ## Example
///
/// ```ignore
/// #[impl_self]
/// mod ScrollBarRegion {
///     #[autoimpl(Deref, DerefMut using self.0)]
///     #[derive(Clone, Default)]
///     #[derive_widget]
///     pub struct ScrollBarRegion<W: Widget>(#[widget] ScrollBars<ScrollRegion<W>>);
/// }
/// ```
///
/// ### Example mapping data
///
/// This macro supports mapping the data passed to the inner widget. The
/// attribute annotating the inner field specifies the data map. It is required
/// to specify the data type, either via an explicit `impl` of `Widget` or as
/// below.
///
/// ```ignore
/// #[impl_self]
/// mod Map {
///     #[autoimpl(Deref, DerefMut using self.inner)]
///     #[autoimpl(Scrollable using self.inner where W: trait)]
///     #[derive_widget(type Data = A)]
///     pub struct Map<A, W: Widget, F>
///     where
///         F: for<'a> Fn(&'a A) -> &'a W::Data,
///     {
///         #[widget((self.map_fn)(data))]
///         pub inner: W,
///         map_fn: F,
///         _data: PhantomData<A>,
///     }
/// }
/// ```
///
/// ### A note on `Deref`
///
/// The above examples implement [`Deref`] over the inner widget. This is
/// acceptable for a simple wrapping "derive widget". It is not recommended to
/// implement [`Deref`] for non-derived widgets (i.e. when the outer widget has
/// its own `Id`) due to the potential for method collision (e.g. `outer.id()`
/// may resolve to `outer.deref().id()` when the trait providing `fn id` is not
/// in scope, yet is available through a bound on the field).
///
/// [`Widget`]: https://docs.rs/kas/latest/kas/trait.Widget.html
/// [`Deref`]: std::ops::Deref
#[proc_macro_attribute]
#[proc_macro_error]
pub fn derive_widget(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must be used within scope of #[impl_self], impl_scope! or impl_anon!");
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
/// This macro is usable only within an [`macro@impl_self`], [`impl_scope!`] or
/// [`impl_anon!`] macro using the [`macro@widget`] attribute.
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
/// an [`macro@impl_self`], [`impl_scope!`] or [`impl_anon!`] macro using the
/// [`macro@widget`] attribute.
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
