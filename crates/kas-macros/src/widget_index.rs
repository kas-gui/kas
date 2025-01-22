// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Span, TokenStream};
use proc_macro_error2::emit_error;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, Error, Member, Result, Token};

#[allow(non_camel_case_types)]
mod kw {
    syn::custom_keyword!(error_emitted);
    syn::custom_keyword!(expanded_result);
}

pub struct UnscopedInput(TokenStream);
impl Parse for UnscopedInput {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(kw::expanded_result) {
            // Okay: macro was expanded internally
            let _: kw::expanded_result = input.parse()?;
            Ok(UnscopedInput(input.parse()?))
        } else if input.peek(kw::error_emitted) {
            // An error was already emitted by the visitor
            let _ = input.parse::<kw::error_emitted>()?;
            Ok(UnscopedInput(input.parse()?))
        } else {
            let msg = "usage invalid outside of `impl_scope!` macro with `#[widget]` attribute";
            Err(Error::new(Span::call_site(), msg))
        }
    }
}
impl UnscopedInput {
    pub fn into_token_stream(self) -> TokenStream {
        self.0
    }
}

pub struct WidgetInput {
    pub ident: Member,
}

impl Parse for WidgetInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token![self]>()?;
        let _ = input.parse::<Token![.]>()?;
        let ident = input.parse()?;

        Ok(WidgetInput { ident })
    }
}

struct Visitor<'a, I: Clone + Iterator<Item = (usize, &'a Member)>> {
    children: I,
    path_rect: TokenStream,
}
impl<'a, I: Clone + Iterator<Item = (usize, &'a Member)>> VisitMut for Visitor<'a, I> {
    fn visit_macro_mut(&mut self, node: &mut syn::Macro) {
        // HACK: we cannot expand the macro here since we do not have an Expr
        // to replace. Instead we can only modify the macro's tokens.
        // WARNING: if the macro's tokens are modified before printing an error
        // message is emitted then the span of that error message is incorrect.

        if node.path == parse_quote! { widget_index } {
            let args = match syn::parse2::<WidgetInput>(node.tokens.clone()) {
                Ok(args) => args,
                Err(err) => {
                    emit_error!(node.tokens.span(), "{}", err);
                    node.tokens = parse_quote! { error_emitted 0 };
                    return;
                }
            };

            for (i, child) in self.children.clone() {
                if args.ident == *child {
                    node.tokens = parse_quote! { expanded_result #i };
                    return;
                }
            }

            emit_error!(args.ident.span(), "does not match any child widget");
            node.tokens = parse_quote! { error_emitted 0 };
            return;
        } else if node.path == parse_quote! { widget_set_rect } {
            let expr = match syn::parse2::<syn::Expr>(node.tokens.clone()) {
                Ok(expr) => expr,
                Err(err) => {
                    emit_error!(node.tokens.span(), "{}", err);
                    node.tokens = parse_quote! { error_emitted };
                    return;
                }
            };

            let path_rect = &self.path_rect;
            node.tokens = parse_quote! { expanded_result #path_rect = #expr };
            return;
        }

        visit_mut::visit_macro_mut(self, node);
    }
}

pub fn visit_impls<'a, I: Clone + Iterator<Item = (usize, &'a Member)>>(
    children: I,
    path_rect: TokenStream,
    impls: &mut [syn::ItemImpl],
) {
    let mut obj = Visitor {
        children,
        path_rect,
    };

    for impl_ in impls {
        obj.visit_item_impl_mut(impl_);
    }
}
