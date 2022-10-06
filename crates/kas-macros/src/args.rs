// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{token::Eq, Member, Token};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(layout);
    custom_keyword!(navigable);
    custom_keyword!(hover_highlight);
    custom_keyword!(cursor_icon);
    custom_keyword!(derive);
}

#[derive(Debug)]
pub struct BoolToken {
    pub kw_span: Span,
    pub eq: Eq,
    pub lit: syn::LitBool,
}

#[derive(Debug)]
pub struct ExprToken {
    pub kw_span: Span,
    pub eq: Eq,
    pub expr: syn::Expr,
}

#[derive(Debug, Default)]
pub struct WidgetArgs {
    pub navigable: Option<TokenStream>,
    pub hover_highlight: Option<BoolToken>,
    pub cursor_icon: Option<ExprToken>,
    pub derive: Option<Member>,
    pub layout: Option<(Span, make_layout::Tree)>,
}

impl Parse for WidgetArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let mut navigable = None;
        let mut hover_highlight = None;
        let mut cursor_icon = None;
        let mut kw_derive = None;
        let mut derive = None;
        let mut layout = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::navigable) && navigable.is_none() {
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
                layout = Some((kw.span, content.parse()?));
            } else {
                return Err(lookahead.error());
            }

            let _ = content.parse::<Token![;]>()?;
        }

        if let Some(_derive) = kw_derive {
            if let Some((span, _)) = layout {
                return Err(Error::new(span, "incompatible with widget derive"));
                // note = derive.span() => "this derive"
            }
        }

        Ok(WidgetArgs {
            navigable,
            hover_highlight,
            cursor_icon,
            derive,
            layout,
        })
    }
}
