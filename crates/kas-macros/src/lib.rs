// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS macros

#![recursion_limit = "128"]
#![allow(clippy::let_and_return)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::{emit_call_site_error, proc_macro_error};
use quote::quote;
use syn::parse_macro_input;
use syn::{GenericParam, Generics, ItemStruct};

mod args;
mod autoimpl;
mod make_layout;
mod make_widget;
pub(crate) mod where_clause;
mod widget;
mod widget_index;

/// A variant of the standard `derive` macro
///
/// This macro is similar to `#[derive(Trait)]`, but with a few differences.
///
/// If using `autoimpl` **and** `derive` macros, the `autoimpl` attribute must
/// come first (this limitation is already fixed in Rust nightly; see rust#81119).
///
/// Support is currently limited to structs, though in theory enums could be
/// supported too, at least for the "mutli-field traits".
///
/// Unlike `derive`, `autoimpl` is not extensible by third-party crates. The
/// "trait names" provided to `autoimpl` are matched directly, unlike
/// `derive(...)` arguments which are paths to [`proc_macro_derive`] instances.
/// Without language support for this there appears to be no option for
/// third-party extensions.
///
/// # Bounds
///
/// No bounds on generic parameters are assumed. For example, if a struct has
/// type parameter `X: 'static`, then `derive(Debug)` would assume the
/// bound `X: Debug + 'static` on the implementation (this may or may not be
/// desired). In contrast, `autoimpl(Debug)` will only assume bounds on the
/// struct itself (in this case `X: 'static`).
///
/// Additional bounds may be specified on the implementation in the form of a
/// `where` clause following the traits, e.g. `autoimpl(Debug where X: Debug)`.
///
/// A special type of bound is supported: `X: trait` — in this case `trait`
/// resolves to the trait currently being derived.
///
/// [`proc_macro_derive`]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-macros
///
/// # Multi-field traits
///
/// Some trait implementations make use of all fields by default. Individual
/// fields may be ignored via the `ignore self.x, self.y` syntax (after any `where`
/// clauses). The following traits may be derived this way:
///
/// -   `Clone` — implements `std::clone::Clone`; ignored fields are
///     initialised with `Default::default()`
/// -   `Debug` — implements `std::fmt::Debug`; ignored fields are not printed
///
/// # Single-field traits
///
/// Other trait implementations make use of a single field, identified via the
/// `on self.x` syntax (after any `where` clauses). The following traits may be
/// derived in this way:
///
/// -   `Deref` — implements `std::ops::Deref`
/// -   `DerefMut` — implements `std::ops::DerefMut`
/// -   `HasBool`, `HasStr`, `HasString`, `SetAccel` — implement the `kas::class` traits
/// -   `class_traits` — implements each `kas::class` trait (intended to be
///     used with a where clause like `where W: trait`)
///
/// # Examples
///
/// Basic usage: `#[autoimpl(Debug)]`
///
/// Implement `Clone` and `Debug` on a wrapper, with the required bounds:
/// ```rust
/// # use kas_macros::autoimpl;
/// #[autoimpl(Clone, Debug where T: trait)]
/// struct Wrapper<T>(pub T);
/// ```
///
/// Implement `Debug` with a custom bound and skipping an unformattable field:
/// ```rust
/// use kas_macros::autoimpl;
/// use std::fmt::Debug;
///
/// #[autoimpl(Debug where X: Debug ignore self.z)]
/// struct S<X, Z> {
///     x: X,
///     y: String,
///     z: Z,
/// }
/// ```
///
/// Implement `Deref` and `DerefMut`, dereferencing to the given field:
/// ```rust
/// # use kas_macros::autoimpl;
/// #[autoimpl(Deref, DerefMut on self.0)]
/// struct MyWrapper<T>(T);
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn autoimpl(attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse(attr) {
        Ok(attr) => {
            let mut toks = item.clone();
            let item = parse_macro_input!(item as ItemStruct);
            let impls = autoimpl::autoimpl(attr, item);
            toks.extend(TokenStream::from(impls));
            toks
        }
        Err(err) => {
            emit_call_site_error!(err);
            // Since autoimpl only adds implementations, we can safely output
            // the original item, thus reducing secondary errors:
            item
        }
    }
}

// Support impls on Self by replacing name and summing generics
fn extend_generics(generics: &mut Generics, in_generics: &Generics) {
    if generics.lt_token.is_none() {
        debug_assert!(generics.params.is_empty());
        debug_assert!(generics.gt_token.is_none());
        generics.lt_token = in_generics.lt_token;
        generics.params = in_generics.params.clone();
        generics.gt_token = in_generics.gt_token;
    } else if in_generics.lt_token.is_none() {
        debug_assert!(in_generics.params.is_empty());
        debug_assert!(in_generics.gt_token.is_none());
    } else {
        if !generics.params.empty_or_trailing() {
            generics.params.push_punct(Default::default());
        }
        generics
            .params
            .extend(in_generics.params.clone().into_pairs());
    }

    // Strip defaults which are legal on the struct but not on impls
    for param in &mut generics.params {
        match param {
            GenericParam::Type(p) => {
                p.eq_token = None;
                p.default = None;
            }
            GenericParam::Lifetime(_) => (),
            GenericParam::Const(p) => {
                p.eq_token = None;
                p.default = None;
            }
        }
    }

    if let Some(ref mut clause1) = generics.where_clause {
        if let Some(ref clause2) = in_generics.where_clause {
            if !clause1.predicates.empty_or_trailing() {
                clause1.predicates.push_punct(Default::default());
            }
            clause1
                .predicates
                .extend(clause2.predicates.clone().into_pairs());
        }
    } else {
        generics.where_clause = in_generics.where_clause.clone();
    }
}

/// Macro to derive widget traits
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-widget-macro).
#[proc_macro_error]
#[proc_macro]
pub fn widget(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as args::Widget);
    widget::widget(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
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

/// Macro to derive `From<VoidMsg>`
///
/// See documentation [ in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-derivevoidmsg-macro).
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
/// This macro is usable only within a [`widget!`] macro.
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
