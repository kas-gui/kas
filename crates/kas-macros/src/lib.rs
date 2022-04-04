// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS macros

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

/// Attribute to implement `kas::Widget`
///
/// TODO: doc
#[proc_macro_attribute]
#[proc_macro_error]
pub fn widget(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must be used within impl_scope! { ... }");
    item
}

/// Macro to create a widget with anonymous type
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-make_widget-macro).
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
/// > _AlignType_ :\
/// > &nbsp;&nbsp; `center` | `stretch`
/// >
/// > _Align_ :\
/// > &nbsp;&nbsp; `align` `(` _AlignType_ `)` `:` _Layout_
/// >
/// > _Direction_ :\
/// > &nbsp;&nbsp; `left` | `right` | `up` | `down` | `self` `.` _Member_
/// >
/// > _Field_ :\
/// > &nbsp;&nbsp; `self` `.` _Member_ | _Expr_
/// >
/// > _ListPre_ :\
/// > &nbsp;&nbsp; `column` | `row` | `list` `(` _Direction_ `)`
/// >
/// > _List_ :\
/// > &nbsp;&nbsp; _ListPre_ `:` `[` _Layout_ `]`
/// >
/// > _Slice_ :\
/// > &nbsp;&nbsp; `slice` `(` _Direction_ `)` `:` `self` `.` _Member_
/// >
/// > _Frame_ :\
/// > &nbsp;&nbsp; `frame` `(` _Layout_ `)`
/// >
/// > _Layout_ :\
/// > &nbsp;&nbsp; &nbsp;&nbsp; _Align_ | _Single_ | _List_ | _Slice_ | _Frame_
/// >
/// > _MakeLayout_:\
/// > &nbsp;&nbsp; `(` _CoreData_ `;` _Layout_ `)`
///
/// ## Notes
///
/// Fields are specified via `self.NAME`; referencing is implied (the macro
/// converts to `&mut self.NAME` or a suitable method call). Embedded field
/// access (e.g. `self.x.y`) is also supported.
///
/// `row` and `column` are abbreviations for `list(right)` and `list(down)`
/// respectively.
///
/// _Slice_ is a variant of _List_ over a single struct field, supporting
/// `AsMut<W>` for some widget type `W`.
///
/// _Member_ is a field name (struct) or number (tuple struct).
///
/// # Example
///
/// ```none
/// make_layout!(self.core; row[self.a, self.b])
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
/// [`WidgetChildren`]: https://docs.rs/kas/latest/kas/trait.WidgetChildren.html
#[proc_macro_error]
#[proc_macro]
pub fn widget_index(input: TokenStream) -> TokenStream {
    let input2 = input.clone();
    let _ = parse_macro_input!(input2 as widget_index::BaseInput);
    input
}
