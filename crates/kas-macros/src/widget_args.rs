// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::SimplePath;
use impl_tools_lib::scope::{Scope, ScopeAttr};
use proc_macro2::{Span, TokenStream as Toks};
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Index, Member, Meta};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(layout);
    custom_keyword!(Data);
}

#[derive(Debug, Default)]
pub struct WidgetArgs {}

impl Parse for WidgetArgs {
    fn parse(_: ParseStream) -> Result<Self> {
        Ok(WidgetArgs {})
    }
}

pub fn member(index: usize, ident: Option<Ident>) -> Member {
    match ident {
        None => Member::Unnamed(Index {
            index: index as u32,
            span: Span::call_site(),
        }),
        Some(ident) => Member::Named(ident),
    }
}

pub struct AttrImplWidget;
impl ScopeAttr for AttrImplWidget {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["widget"])
    }

    fn apply(&self, attr: syn::Attribute, scope: &mut Scope) -> Result<()> {
        let span = attr.span();
        match &attr.meta {
            Meta::Path(_) => (),
            _ => {
                let _: WidgetArgs = attr.parse_args()?;
            }
        };
        crate::widget::widget(span, scope)
    }
}

#[derive(Debug)]
pub enum ChildIdent {
    /// Child is a direct field
    Field(Member),
    /// Child is a hidden field (under #core_path)
    CoreField(Member),
}
impl ChildIdent {
    pub fn get_rule(&self, core_path: &Toks, i: usize) -> Toks {
        match self {
            ChildIdent::Field(ident) => quote! { #i => Some(self.#ident.as_tile()), },
            ChildIdent::CoreField(ident) => quote! { #i => Some(#core_path.#ident.as_tile()), },
        }
    }
}

pub struct Child {
    pub ident: ChildIdent,
    pub attr_span: Option<Span>,
    pub data_binding: Option<Expr>,
}
impl Child {
    pub fn new_core(ident: Member) -> Self {
        Child {
            ident: ChildIdent::CoreField(ident),
            attr_span: None,
            data_binding: Some(syn::parse_quote! { &() }),
        }
    }
}
