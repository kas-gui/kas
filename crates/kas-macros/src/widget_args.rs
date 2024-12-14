// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use impl_tools_lib::scope::{Scope, ScopeAttr};
use impl_tools_lib::SimplePath;
use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, quote_spanned};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Eq;
use syn::{Expr, Ident, Index, Member, Meta, Token, Type};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(layout);
    custom_keyword!(navigable);
    custom_keyword!(hover_highlight);
    custom_keyword!(cursor_icon);
    custom_keyword!(derive);
    custom_keyword!(Data);
}

#[derive(Debug)]
pub struct BoolToken {
    #[allow(dead_code)]
    pub kw_span: Span,
    #[allow(dead_code)]
    pub eq: Eq,
    pub lit: syn::LitBool,
}

#[derive(Debug)]
pub struct ExprToken {
    #[allow(dead_code)]
    pub kw_span: Span,
    #[allow(dead_code)]
    pub eq: Eq,
    pub expr: syn::Expr,
}

#[derive(Debug, Default)]
pub struct WidgetArgs {
    pub data_ty: Option<Type>,
    pub navigable: Option<Toks>,
    pub hover_highlight: Option<BoolToken>,
    pub cursor_icon: Option<ExprToken>,
    pub derive: Option<Member>,
    pub layout: Option<(kw::layout, make_layout::Tree)>,
}

impl Parse for WidgetArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let mut data_ty = None;
        let mut navigable = None;
        let mut hover_highlight = None;
        let mut cursor_icon = None;
        let mut kw_derive = None;
        let mut derive = None;
        let mut layout = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::Data) && data_ty.is_none() {
                let kw = content.parse::<kw::Data>()?;
                let _: Eq = content.parse()?;
                data_ty = Some((kw, content.parse()?));
            } else if lookahead.peek(kw::navigable) && navigable.is_none() {
                let span = content.parse::<kw::navigable>()?.span();
                let _: Eq = content.parse()?;
                let value = content.parse::<syn::LitBool>()?;
                navigable = Some(quote_spanned! {span=>
                    fn navigable(&self) -> bool { #value }
                });
            } else if lookahead.peek(kw::hover_highlight) && hover_highlight.is_none() {
                hover_highlight = Some(BoolToken {
                    kw_span: content.parse::<kw::hover_highlight>()?.span(),
                    eq: content.parse()?,
                    lit: content.parse()?,
                });
            } else if lookahead.peek(kw::cursor_icon) && cursor_icon.is_none() {
                cursor_icon = Some(ExprToken {
                    kw_span: content.parse::<kw::cursor_icon>()?.span(),
                    eq: content.parse()?,
                    expr: content.parse()?,
                });
            } else if lookahead.peek(kw::derive) && derive.is_none() {
                kw_derive = Some(content.parse::<kw::derive>()?);
                let _: Eq = content.parse()?;
                let _: Token![self] = content.parse()?;
                let _: Token![.] = content.parse()?;
                derive = Some(content.parse()?);
            } else if lookahead.peek(kw::layout) && layout.is_none() {
                let kw = content.parse::<kw::layout>()?;
                let _: Eq = content.parse()?;
                layout = Some((kw, content.parse()?));
            } else {
                return Err(lookahead.error());
            }

            let _ = content.parse::<Token![;]>()?;
        }

        if let Some(_derive) = kw_derive {
            if let Some((kw, _)) = layout {
                return Err(Error::new(kw.span, "incompatible with widget derive"));
                // note = derive.span() => "this derive"
            }
            if let Some((kw, _)) = data_ty {
                return Err(Error::new(kw.span, "incompatible with widget derive"));
            }
        }

        Ok(WidgetArgs {
            data_ty: data_ty.map(|(_, ty)| ty),
            navigable,
            hover_highlight,
            cursor_icon,
            derive,
            layout,
        })
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
        if args.derive.is_some() {
            crate::widget_derive::widget(span, args, scope)
        } else {
            crate::widget::widget(span, args, scope)
        }
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
            ChildIdent::Field(ident) => quote! { #i => Some(self.#ident.as_layout()), },
            ChildIdent::CoreField(ident) => quote! { #i => Some(#core_path.#ident.as_layout()), },
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
            data_binding: None,
        }
    }
}
