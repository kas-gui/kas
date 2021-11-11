// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![recursion_limit = "128"]
#![allow(clippy::let_and_return)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::parse_macro_input;
use syn::{GenericParam, Generics, ItemStruct};

mod args;
mod autoimpl;
mod layout;
mod make_widget;
mod widget;

/// A variant of the standard `derive` macro
///
/// This macro behaves like `#[derive(Trait)]` except that:
///
/// -   Only a fixed list of traits is supported
/// -   It (currently) only supports struct types (including tuple and unit structs)
/// -   No bounds on generics (beyond those on the struct itself) are assumed
/// -   Bounds may be specified manually via `where ...`
/// -   Fields may be skipped, e.g. `skip a, b`
/// -   Certain traits may target a specific field via `on x`
///
/// The following traits are supported:
///
/// -   `Clone` — implements `std::clone::Clone`; any skipped field is
///     initialised with `Default::default()`
/// -   `Debug` — implements `std::fmt::Debug`; skipped fields are not output
///
/// # Examples
///
/// Basic usage: `#[autoimpl(Debug)]`
///
/// Implement `Debug` with a custom bound and skipping an unformattable field:
/// ```rust
/// #[autoimpl(Debug where X: Debug skip z)]
/// struct S<X, Z> {
///     x: X,
///     y: String,
///     z: Z,
/// }
/// ```
#[proc_macro_attribute]
pub fn autoimpl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut toks = item.clone();
    let attr = parse_macro_input!(attr as autoimpl::AutoImpl);
    let item = parse_macro_input!(item as ItemStruct);
    let impls = autoimpl::autoimpl(attr, item);
    toks.extend(TokenStream::from(impls));
    toks
}

// Support impls on Self by replacing name and summing generics
fn extend_generics(generics: &mut Generics, in_generics: &Generics) {
    if generics.lt_token.is_none() {
        debug_assert!(generics.params.is_empty());
        debug_assert!(generics.gt_token.is_none());
        generics.lt_token = in_generics.lt_token.clone();
        generics.params = in_generics.params.clone();
        generics.gt_token = in_generics.gt_token.clone();
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
    widget::widget(args).into()
}

/// Macro to create a widget with anonymous type
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-make_widget-macro).
#[proc_macro_error]
#[proc_macro]
pub fn make_widget(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as args::MakeWidget);
    make_widget::make_widget(args).into()
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
