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

extern crate proc_macro;

use impl_tools_lib::autoimpl;
use impl_tools_lib::{AttrImplDefault, ImplDefault, Scope, ScopeAttr};
use proc_macro::TokenStream;
use proc_macro_error::{emit_call_site_error, proc_macro_error};
use quote::quote;
use syn::parse_macro_input;

mod args;
mod class_traits;
mod make_layout;
mod make_widget;
mod widget;
mod widget_index;

/// Implement `Default`
///
/// This macro may be used in one of two ways.
///
/// ### Type-level initialiser
///
/// ```
/// # use kas_macros::impl_default;
/// /// A simple enum; default value is Blue
/// #[impl_default(Colour::Blue)]
/// enum Colour {
///     Red,
///     Green,
///     Blue,
/// }
///
/// fn main() {
///     assert!(matches!(Colour::default(), Colour::Blue));
/// }
/// ```
///
/// A where clause is optional: `#[impl_default(EXPR where BOUNDS)]`.
///
/// ### Field-level initialiser
///
/// This variant only supports structs. Fields specified as `name: type = expr`
/// will be initialised with `expr`, while other fields will be initialised with
/// `Defualt::default()`.
///
/// ```
/// # use kas_macros::{impl_default, impl_scope};
///
/// impl_scope! {
///     #[impl_default]
///     struct Person {
///         name: String = "Jane Doe".to_string(),
///         age: u32 = 72,
///         occupation: String,
///     }
/// }
///
/// fn main() {
///     let person = Person::default();
///     assert_eq!(person.name, "Jane Doe");
///     assert_eq!(person.age, 72);
///     assert_eq!(person.occupation, "");
/// }
/// ```
///
/// A where clause is optional: `#[impl_default(where BOUNDS)]`.
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
/// This macro is similar to `#[derive(Trait)]`, but with a few differences.
///
/// If using `autoimpl` **and** `derive` macros with Rust < 1.57.0, the
/// `autoimpl` attribute must come first (see rust#81119).
///
/// Unlike `derive`, `autoimpl` is not extensible by third-party crates. The
/// "trait names" provided to `autoimpl` are matched directly, unlike
/// `derive(...)` arguments which are paths to [`proc_macro_derive`] instances.
/// Without language support for this there appears to be no option for
/// third-party extensions.
///
/// [`proc_macro_derive`]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-macros
///
/// ### Bounds on generic parameters
///
/// If a type has generic parameters, generated implementations will assume the
/// same parameters and bounds as specified in the type, but not additional
/// bounds for the trait implemented.
///
/// Additional bounds may be specified via a `where` clause. A special predicate
/// is supported: `T: trait`; here `trait` is replaced the name of the trait
/// being implemented.
///
/// # Multi-field traits
///
/// Some trait implementations make use of all fields (except those ignored):
///
/// -   `Clone` — implements `std::clone::Clone`; ignored fields are
///     initialised with `Default::default()`
/// -   `Debug` — implements `std::fmt::Debug`; ignored fields are not printed
/// -   `Default` — implements `std::default::Default` using
///     `Default::default()` for all fields (see also [`impl_default`](macro@impl_default))
///
/// ### Parameter syntax
///
/// > _ParamsMulti_ :\
/// > &nbsp;&nbsp; ( _Trait_ ),+ _Ignores_? _WhereClause_?
/// >
/// > _Ignores_ :\
/// > &nbsp;&nbsp; `ignore` ( `self` `.` _Member_ ),+
/// >
/// > _WhereClause_ :\
/// > &nbsp;&nbsp; `where` ( _WherePredicate_ ),*
///
/// ### Examples
///
/// Implement `std::fmt::Debug`, ignoring the last field:
/// ```
/// # use kas_macros::autoimpl;
/// #[autoimpl(Debug ignore self.f)]
/// struct PairWithFn<T> {
///     x: f32,
///     y: f32,
///     f: fn(&T),
/// }
/// ```
///
/// Implement `Clone` and `Debug` on a wrapper, with the required bounds:
/// ```
/// # use kas_macros::autoimpl;
/// #[autoimpl(Clone, Debug where T: trait)]
/// struct Wrapper<T>(pub T);
/// ```
/// Note: `T: trait` is a special predicate implying that for each
/// implementation the type `T` must support the trait being implemented.
///
/// # Single-field traits
///
/// Other traits are implemented using a single field (for structs):
///
/// -   `Deref` — implements `std::ops::Deref`
/// -   `DerefMut` — implements `std::ops::DerefMut`
/// -   `HasBool`, `HasStr`, `HasString`, `SetAccel` — implement the `kas::class` traits
/// -   `class_traits` — implements each `kas::class` trait (intended to be
///     used with a where clause like `where W: trait`)
///
/// ### Parameter syntax
///
/// > _ParamsSingle_ :\
/// > &nbsp;&nbsp; ( _Trait_ ),+ _Using_ _WhereClause_?
/// >
/// > _Using_ :\
/// > &nbsp;&nbsp; `using` `self` `.` _Member_
///
/// ### Examples
///
/// Implement `Deref` and `DerefMut`, dereferencing to the given field:
/// ```
/// # use kas_macros::autoimpl;
/// #[autoimpl(Deref, DerefMut using self.0)]
/// struct MyWrapper<T>(T);
/// ```
///
/// # Trait re-implementations
///
/// User-defined traits may be implemented over any type supporting `Deref`
/// (and if required `DerefMut`) to another type supporting the trait.
///
/// ### Parameter syntax
///
/// > _ParamsTrait_ :\
/// > &nbsp;&nbsp; `for` _Generics_ ( _Type_ ),+ _Definitive_? _WhereClause_?
/// >
/// > _Generics_ :\
/// > &nbsp;&nbsp; `<` ( _GenericParam_ ) `>`
/// >
/// > _Definitive_ :\
/// > &nbsp;&nbsp; `using` _Type_
///
/// ### Examples
///
/// Implement `MyTrait` for `&T`, `&mut T` and `Box<dyn MyTrait>`:
/// ```
/// # use kas_macros::autoimpl;
/// #[autoimpl(for<'a, T: trait + ?Sized> &'a T, &'a mut T, Box<T>)]
/// trait MyTrait {
///     fn f(&self) -> String;
/// }
/// ```
/// Note that the first parameter bound like `T: trait` is used as the
/// definitive type (required). For example, here, `f` is implemented with the
/// body `<T as MyTrait>::f(self)`.
///
/// Note further: if the trait uses generic parameters itself, these must be
/// introduced explicitly in the `for<..>` parameter list.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn autoimpl(attr: TokenStream, item: TokenStream) -> TokenStream {
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
                    .chain(std::iter::once(
                        &class_traits::ImplClassTraits as &dyn autoimpl::ImplTrait,
                    ))
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

/// Implementation scope
///
/// Supports `impl Self` syntax.
///
/// Also supports struct field assignment syntax for `Default`: see [`impl_default`](macro@impl_default).
///
/// Caveat: `rustfmt` will not format contents (see
/// [rustfmt#5254](https://github.com/rust-lang/rustfmt/issues/5254)).
///
/// ## Syntax
///
/// > _ImplScope_ :\
/// > &nbsp;&nbsp; `impl_scope!` `{` _ScopeItem_ _ItemImpl_ * `}`
/// >
/// > _ScopeItem_ :\
/// > &nbsp;&nbsp; _ItemEnum_ | _ItemStruct_ | _ItemType_ | _ItemUnion_
///
/// The result looks a little like a module containing a single type definition
/// plus its implementations, but is injected into the parent module.
///
/// Implementations must target the type defined at the start of the scope. A
/// special syntax for the target type, `Self`, is added:
///
/// > _ScopeImplItem_ :\
/// > &nbsp;&nbsp; `impl` _GenericParams_? _ForTrait_? _ScopeImplTarget_ _WhereClause_? `{`
/// > &nbsp;&nbsp; &nbsp;&nbsp; _InnerAttribute_*
/// > &nbsp;&nbsp; &nbsp;&nbsp; _AssociatedItem_*
/// > &nbsp;&nbsp; `}`
/// >
/// > _ScopeImplTarget_ :\
/// > &nbsp;&nbsp; `Self` | _TypeName_ _GenericParams_?
///
/// That is, implementations may take one of two forms:
///
/// -   `impl MyType { ... }`
/// -   `impl Self { ... }`
///
/// Generic parameters from the type are included automatically, with bounds as
/// defined on the type. Additional generic parameters and an additional where
/// clause are supported (generic parameter lists and bounds are merged).
///
/// ## Example
///
/// ```
/// use kas_macros::impl_scope;
/// use std::ops::Add;
///
/// impl_scope! {
///     struct Pair<T>(T, T);
///
///     impl Self where T: Clone + Add {
///         fn sum(&self) -> <T as Add>::Output {
///             self.0.clone().add(self.1.clone())
///         }
///     }
/// }
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn impl_scope(input: TokenStream) -> TokenStream {
    let mut scope = parse_macro_input!(input as Scope);
    let rules: [&'static dyn ScopeAttr; 2] = [&AttrImplDefault, &widget::AttrImplWidget];
    scope.apply_attrs(|path| rules.iter().cloned().find(|rule| rule.path().matches(path)));
    scope.expand().into()
}

/// Attribute to implement the `kas::Widget` family of traits
///
/// This may *only* be used within the [`impl_scope!`] macro.
///
/// Implements the [`WidgetCore`] and [`Widget`] traits for the deriving type.
/// Implements the [`WidgetChildren`], [`WidgetConfig`], [`Layout`], [`Handler`]
/// and [`SendEvent`] traits only if not implemented explicitly within the
/// defining [`impl_scope!`].
///
/// Using the `derive` argument activates a special "thin wrapper" mode.
/// In this case, layout may optionally be defined explicitly. Non-layout
/// properties are not supported.
///
/// When not using `derive`, layout must be defined, either via the `layout`
/// argument or by implementing [`Layout`]. All other properties are optional.
///
/// ## Syntax
///
/// > _WidgetAttr_ :\
/// > &nbsp;&nbsp; `#` `[` _WidgetAttrArgs_? `]`
/// >
/// > _WidgetAttrArgs_ :\
/// > &nbsp;&nbsp; `{` (_WidgetAttrArg_ `;`) * `}`
///
/// Supported arguments (_WidgetAttrArg_) are:
///
/// -   `derive` `=` `self` `.` _Member_ — if present, identifies a struct or struct tuple field
///     implementing [`Widget`] over which the `Self` type implements [`Widget`]
/// -   `key_nav` `=` _Bool_ — whether this widget supports keyboard focus via
///     <kbd>Tab</kbd> key (method of [`WidgetConfig`]; default is `false`)
/// -   `hover_highlight` `=` _Bool_ — whether to redraw when cursor hover
///     status is gained/lost (method of [`WidgetConfig`]; default is `false`)
/// -   `cursor_icon` `=` _Expr_ — an expression yielding a [`CursorIcon`]
///     (method of [`WidgetConfig`]; default is `CursorIcon::Default`)
/// -   `layout` `=` _Layout_ — defines widget layout via an expression; see [`make_layout!`] for
///     documentation (method of [`Layout`]; defaults to an empty layout)
/// -   `find_id` `=` _Expr_ — override default implementation of `kas::Layout::find_id` to
///     return this expression when `self.rect().contains(coord)`
/// -   `msg` `=` _Type_ — set [`Handler::Msg`] associated type (default is [`VoidMsg`])
///
/// Assuming the deriving type is a `struct` or `tuple struct`, fields support
/// the following attributes:
///
/// -   `#[widget_core]`: required on one field of type [`CoreData`], unless `derive` argument is
///     used
/// -   `#[widget]`: marks the field as a [`Widget`] to be configured, enumerated by
///     [`WidgetChildren`] and included by glob layouts
///
/// The `#[widget]` attribute on fields may have arguments, affecting how the
/// implementation of [`SendEvent`] handles [`Response`] values from the child:
///
/// -   `#[widget(update = f)]` — when `Response::Update` is received, `self.f()` is called
/// -   `#[widget(use_msg = f)]` — when `Response::Msg(msg)` is received,
///     `self.f(msg)` is called and `Response::Used` is returned
/// -   `#[widget(map_msg = f)]` — when `Response::Msg(msg)` is received,
///     `Response::Msg(self.f(msg))` is returned
/// -   `#[widget(flatmap_msg = f)]` — when `Response::Msg(msg)` is received,
///     `self.f(msg)` is returned
/// -   `#[widget(discard_msg = f)]` — when `Response::Msg(msg)` is received,
///     `Response::Used` is returned
///
/// ## Layout
///
/// Widget layout must be described somehow
///
/// [`Widget`]: https://docs.rs/kas/0.11/kas/trait.Widget.html
/// [`WidgetCore`]: https://docs.rs/kas/0.11/kas/trait.WidgetCore.html
/// [`WidgetChildren`]: https://docs.rs/kas/0.11/kas/trait.WidgetChildren.html
/// [`WidgetConfig`]: https://docs.rs/kas/0.11/kas/trait.WidgetConfig.html
/// [`Layout`]: https://docs.rs/kas/0.11/kas/trait.Layout.html
/// [`Handler`]: https://docs.rs/kas/0.11/kas/event/trait.Handler.html
/// [`Handler::Msg`]: https://docs.rs/kas/0.11/kas/event/trait.Handler.html#associatedtype.Msg
/// [`SendEvent`]: https://docs.rs/kas/0.11/kas/event/trait.SendEvent.html
/// [`CursorIcon`]: https://docs.rs/kas/0.11/kas/event/enum.CursorIcon.html
/// [`VoidMsg`]: https://docs.rs/kas/0.11/kas/event/enum.VoidMsg.html
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
/// Implicit typing is emulated using type parameters plus "smart" bounds. These
/// don't always work and can result in terrible error messages. Another result
/// is that fields cannot be accessed outside the type definition except
/// through their type or a trait bound.
///
/// Syntax is similar to using [`widget`](macro@widget) within [`impl_scope!`], except that:
///
/// -   `#[derive(Debug)]` is added automatically
/// -   a `#[widget_core] core: kas::CoreData` field is added automatically
/// -   all fields must have an initializer, e.g. `ident: ty = value,`
/// -   the type of fields is optional: `ident = value,` works (but see note above)
/// -   the name of fields is optional: `_: ty = value,` and `_ = value,` are valid
/// -   instead of a type, a type bound may be used: `ident: impl Trait = value,`
/// -   instead of a widget type, only the widget's message type may be specified:
///     `ident -> MessageType = value,`
/// -   type generics may be used:
///     `#[widget] display: for<W: Widget<Msg = VoidMsg>> Frame<W> = value,`
///
/// Refer to [examples](https://github.com/search?q=make_widget+repo%3Akas-gui%2Fkas+path%3Aexamples&type=Code) for usage.
#[proc_macro_error]
#[proc_macro]
pub fn make_widget(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as args::MakeWidget);
    make_widget::make_widget(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Macro to make a `kas::layout::Layout`
///
/// Generates some type of layout, often over child widgets.
/// The widget's core data is required (usually a field named `core`).
///
/// # Syntax
///
/// > _MakeLayout_:\
/// > &nbsp;&nbsp; `make_layout` `!` `(` _CoreData_ `;` _Layout_ `)`
/// >
/// > _Layout_ :\
/// > &nbsp;&nbsp; &nbsp;&nbsp; _Single_ | _List_ | _Slice_ | _Grid_ | _Align_ | _Frame_
/// >
/// > _Single_ :\
/// > &nbsp;&nbsp; `single` | `self` `.` _Member_
/// >
/// > _List_ :\
/// > &nbsp;&nbsp; _ListPre_ `:` `*` | (`[` _Layout_ `]`)
/// >
/// > _ListPre_ :\
/// > &nbsp;&nbsp; `column` | `row` | `aligned_column` | `aligned_row` | `list` `(` _Direction_ `)`
/// >
/// > _Slice_ :\
/// > &nbsp;&nbsp; `slice` `(` _Direction_ `)` `:` `self` `.` _Member_
/// >
/// > _Direction_ :\
/// > &nbsp;&nbsp; `left` | `right` | `up` | `down`
/// >
/// > _Grid_ :\
/// > &nbsp;&nbsp; `grid` `:` `{` _GridCell_* `}`
/// >
/// > _GridCell_ :\
/// > &nbsp;&nbsp; _CellRange_ `,` _CellRange_ `:` _Layout_
/// >
/// > _CellRange_ :\
/// > &nbsp;&nbsp; _LitInt_ ( `..` `+`? _LitInt_ )?
///
/// > _Align_ :\
/// > &nbsp;&nbsp; `align` `(` _AlignType_ `)` `:` _Layout_
/// >
/// > _AlignType_ :\
/// > &nbsp;&nbsp; `center` | `stretch`
/// >
/// > _Frame_ :\
/// > &nbsp;&nbsp; `frame` `(` _Style_ `)` `:` _Layout_
///
/// ## Notes
///
/// Both _Single_ and _Slice_ variants match `self.MEMBER` where `MEMBER` is the
/// name of a field or number of a tuple field. More precisely, both match any
/// expression starting with `self` and append with `.as_widget_mut()`.
///
/// `row` and `column` are abbreviations for `list(right)` and `list(down)`
/// respectively. Glob syntax is allowed: `row: *` uses all children in a row
/// layout.
///
/// `aligned_column` and `aligned_row` use restricted list syntax (items must
/// be `row` or `column` respectively; glob syntax not allowed), but build a
/// grid layout. Essentially, they are syntax sugar for simple table layouts.
///
/// _Slice_ is a variant of _List_ over a single struct field which supports
/// `AsMut<W>` for some widget type `W`.
///
/// A _Grid_ is an aligned two-dimensional layout supporting item spans.
/// Contents are declared as a collection of cells. Cell location is specified
/// like `0, 1` (that is, col=0, row=1) with spans specified like `0..2, 1`
/// (thus cols={0, 1}, row=1) or `2..+2, 1` (cols={2,3}, row=1).
///
/// _Member_ is a field name (struct) or number (tuple struct).
///
/// # Example
///
/// ```none
/// make_layout!(self.core; row[self.a, self.b])
/// ```
///
/// # Grid
///
/// Grid cells are defined by `row, column` ranges, where the ranges are either
/// a half-open range or a single number (who's end is implicitly `start + 1`).
///
/// ```none
/// make_layout!(self.core; grid: {
///     0..2, 0: self.merged_title;
///     0, 1: self.a;
///     1, 1: self.b;
///     1, 2: self.c;
/// })
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn make_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as make_layout::Input);
    make_layout::make_layout(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implement `From<VoidMsg>`
///
/// Since `VoidMsg` is a void type (cannot exist at run-time), `From<VoidMsg>`
/// can safely be implemented for *any* type. (But due to the theoretical
/// possibility of avoid conflicting implementations, it is not implemented
/// automatically until Rust has some form of specialization.)
#[proc_macro_error]
#[proc_macro_derive(VoidMsg)]
pub fn derive_empty_msg(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;

    let toks = quote! {
        impl #impl_generics From<::kas::event::VoidMsg>
            for #name #ty_generics #where_clause
        {
            fn from(_: ::kas::event::VoidMsg) -> Self {
                unreachable!()
            }
        }
    };
    toks.into()
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
