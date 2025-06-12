// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use impl_tools_lib::scope::{Scope, ScopeAttr};
use impl_tools_lib::SimplePath;
use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Eq;
use syn::{Expr, Ident, Index, Member, Meta, Token};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(layout);
    custom_keyword!(Data);
}

#[derive(Debug)]
pub struct DataTy {
    pub kw: kw::Data,
    pub eq: Eq,
    pub ty: syn::Type,
}
impl ToTokens for DataTy {
    fn to_tokens(&self, tokens: &mut Toks) {
        self.kw.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

#[derive(Debug)]
pub struct Layout {
    #[allow(dead_code)]
    pub kw: kw::layout,
    #[allow(dead_code)]
    pub eq: Eq,
    pub tree: make_layout::Tree,
}

#[derive(Debug, Default)]
pub struct WidgetArgs {
    pub data_ty: Option<DataTy>,
    pub layout: Option<Layout>,
}

impl Parse for WidgetArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let mut data_ty = None;
        let mut layout = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::Data) && data_ty.is_none() {
                data_ty = Some(DataTy {
                    kw: content.parse()?,
                    eq: content.parse()?,
                    ty: content.parse()?,
                });
            } else if lookahead.peek(kw::layout) && layout.is_none() {
                layout = Some(Layout {
                    kw: content.parse()?,
                    eq: content.parse()?,
                    tree: content.parse()?,
                });
            } else {
                return Err(lookahead.error());
            }

            let _ = content.parse::<Token![;]>()?;
        }

        Ok(WidgetArgs { data_ty, layout })
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
        let args = match &attr.meta {
            Meta::Path(_) => WidgetArgs::default(),
            _ => attr.parse_args()?,
        };
        crate::widget::widget(span, args, scope)
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
