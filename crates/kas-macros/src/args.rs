// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use proc_macro2::TokenStream;
use proc_macro_error::{abort, emit_error};
use quote::quote_spanned;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq, Paren, Semi};
use syn::{braced, bracketed, parenthesized, parse_quote};
use syn::{
    AttrStyle, Attribute, ConstParam, Expr, GenericParam, Generics, Ident, ItemImpl, Lifetime,
    LifetimeDef, Member, Path, Token, Type, TypeParam, TypePath, TypeTraitObject, Visibility,
};

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
}

fn parse_impl(in_ident: Option<&Ident>, input: ParseStream) -> Result<ItemImpl> {
    let mut attrs = input.call(Attribute::parse_outer)?;
    let defaultness: Option<Token![default]> = input.parse()?;
    let unsafety: Option<Token![unsafe]> = input.parse()?;
    let impl_token: Token![impl] = input.parse()?;

    let has_generics = input.peek(Token![<])
        && (input.peek2(Token![>])
            || input.peek2(Token![#])
            || (input.peek2(Ident) || input.peek2(Lifetime))
                && (input.peek3(Token![:])
                    || input.peek3(Token![,])
                    || input.peek3(Token![>])
                    || input.peek3(Token![=]))
            || input.peek2(Token![const]));
    let mut generics: Generics = if has_generics {
        input.parse()?
    } else {
        Generics::default()
    };

    let mut first_ty: Type = input.parse()?;
    let self_ty: Type;
    let trait_;

    let is_impl_for = input.peek(Token![for]);
    if is_impl_for {
        let for_token: Token![for] = input.parse()?;
        let mut first_ty_ref = &first_ty;
        while let Type::Group(ty) = first_ty_ref {
            first_ty_ref = &ty.elem;
        }
        if let Type::Path(_) = first_ty_ref {
            while let Type::Group(ty) = first_ty {
                first_ty = *ty.elem;
            }
            if let Type::Path(TypePath { qself: None, path }) = first_ty {
                trait_ = Some((None, path, for_token));
            } else {
                unreachable!();
            }
        } else {
            return Err(Error::new(for_token.span(), "for without target trait"));
        }
        self_ty = input.parse()?;
    } else {
        trait_ = None;
        self_ty = first_ty;
    }

    generics.where_clause = input.parse()?;

    if self_ty != parse_quote! { Self } {
        if let Some(ident) = in_ident {
            if !matches!(self_ty, Type::Path(TypePath {
                qself: None,
                path: Path {
                    leading_colon: None,
                    ref segments,
                }
            }) if segments.len() == 1 && segments.first().unwrap().ident == *ident)
            {
                abort!(
                    self_ty.span(),
                    format!(
                        "expected `Self` or `{0}` or `{0}<...>` or `Trait for Self`, etc",
                        ident
                    )
                );
            }
        } else {
            abort!(self_ty.span(), "expected `Self` or `Trait for Self`");
        }
    }

    let content;
    let brace_token = braced!(content in input);
    parse_attrs_inner(&content, &mut attrs)?;

    let mut items = Vec::new();
    while !content.is_empty() {
        items.push(content.parse()?);
    }

    Ok(ItemImpl {
        attrs,
        defaultness,
        unsafety,
        impl_token,
        generics,
        trait_,
        self_ty: Box::new(self_ty),
        brace_token,
        items,
    })
}

fn parse_attrs_inner(input: ParseStream, attrs: &mut Vec<Attribute>) -> Result<()> {
    while input.peek(Token![#]) && input.peek2(Token![!]) {
        let pound_token = input.parse()?;
        let style = AttrStyle::Inner(input.parse()?);
        let content;
        let bracket_token = bracketed!(content in input);
        let path = content.call(Path::parse_mod_style)?;
        let tokens = content.parse()?;
        attrs.push(Attribute {
            pound_token,
            style,
            bracket_token,
            path,
            tokens,
        });
    }
    Ok(())
}

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(layout);
    custom_keyword!(col);
    custom_keyword!(row);
    custom_keyword!(cspan);
    custom_keyword!(rspan);
    custom_keyword!(widget);
    custom_keyword!(handler);
    custom_keyword!(generics);
    custom_keyword!(single);
    custom_keyword!(right);
    custom_keyword!(left);
    custom_keyword!(down);
    custom_keyword!(up);
    custom_keyword!(grid);
    custom_keyword!(align);
    custom_keyword!(halign);
    custom_keyword!(valign);
    custom_keyword!(key_nav);
    custom_keyword!(hover_highlight);
    custom_keyword!(cursor_icon);
    custom_keyword!(handle);
    custom_keyword!(send);
    custom_keyword!(config);
    custom_keyword!(children);
    custom_keyword!(column);
    custom_keyword!(draw);
    custom_keyword!(derive);
}

#[derive(Debug, Default)]
pub struct WidgetArgs {
    pub key_nav: Option<TokenStream>,
    pub hover_highlight: Option<bool>,
    pub cursor_icon: Option<Expr>,
    pub derive: Option<Member>,
    pub layout: Option<make_layout::Tree>,
}

impl Parse for WidgetArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let mut key_nav = None;
        let mut hover_highlight = None;
        let mut cursor_icon = None;
        let mut kw_derive = None;
        let mut derive = None;
        let mut kw_layout = None;
        let mut layout = None;

        while !content.is_empty() {
            let mut item_cursor = content.cursor();

            let lookahead = content.lookahead1();
            if lookahead.peek(kw::key_nav) && key_nav.is_none() {
                let span = content.parse::<kw::key_nav>()?.span();
                let _: Eq = content.parse()?;
                let value = content.parse::<syn::LitBool>()?;
                key_nav = Some(quote_spanned! {span=>
                    fn key_nav(&self) -> bool { #value }
                });
            } else if lookahead.peek(kw::hover_highlight) && hover_highlight.is_none() {
                let _ = content.parse::<kw::hover_highlight>()?;
                let _: Eq = content.parse()?;
                let value = content.parse::<syn::LitBool>()?;
                hover_highlight = Some(value.value);
            } else if lookahead.peek(kw::cursor_icon) && cursor_icon.is_none() {
                let _ = content.parse::<kw::cursor_icon>()?;
                let _: Eq = content.parse()?;
                cursor_icon = Some(content.parse::<syn::Expr>()?);
            } else if lookahead.peek(kw::derive) && derive.is_none() {
                kw_derive = Some(content.parse::<kw::derive>()?);
                let _: Eq = content.parse()?;
                let _: Token![self] = content.parse()?;
                let _: Token![.] = content.parse()?;
                derive = Some(content.parse()?);
            } else if lookahead.peek(kw::layout) && layout.is_none() {
                kw_layout = Some(content.parse::<kw::layout>()?);
                let _: Eq = content.parse()?;
                layout = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if let Err(_) = content.parse::<Token![;]>() {
                let mut span = item_cursor.span();
                while let Some((_, next)) = item_cursor.token_tree() {
                    item_cursor = next;
                    if !item_cursor.eof() {
                        let next = item_cursor.span();
                        // NOTE: Span::join always returns None on stable rustc!
                        if let Some(joined) = span.join(next) {
                            span = joined;
                        } else {
                            span = next;
                        }
                    }
                }
                return Err(Error::new(span, "expected `;` after content"));
            }
        }

        if let Some(derive) = kw_derive {
            if let Some(layout) = kw_layout {
                emit_error!(
                    layout, "incompatible with derive";
                    note = derive.span() => "this derive"
                );
            }
        }

        Ok(WidgetArgs {
            key_nav,
            hover_highlight,
            cursor_icon,
            derive,
            layout,
        })
    }
}

#[derive(Debug)]
pub enum StructStyle {
    Unit(Semi),
    Tuple(Paren, Semi),
    Regular(Brace),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChildType {
    Fixed(Type), // fixed type
    // A given type using generics internally
    InternGeneric(Punctuated<GenericParam, Comma>, Type),
    // Generic, optionally with an additional trait bound.
    Generic(Option<TypeTraitObject>),
}

#[derive(Debug)]
pub struct SingletonField {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Option<Ident>,
    pub colon_token: Option<Colon>,
    pub ty: ChildType,
    pub value: Option<Expr>,
}

#[derive(Debug)]
pub struct ImplSingleton {
    pub attrs: Vec<Attribute>,
    pub token: Token![struct],
    pub generics: Generics,
    pub style: StructStyle,
    pub fields: Punctuated<SingletonField, Comma>,
    pub impls: Vec<ItemImpl>,
}

impl Parse for ImplSingleton {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let token = input.parse::<Token![struct]>()?;

        let mut generics = input.parse::<Generics>()?;

        let mut lookahead = input.lookahead1();
        if lookahead.peek(Token![where]) {
            generics.where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        let style;
        let fields;
        if generics.where_clause.is_none() && lookahead.peek(Paren) {
            let content;
            let paren_token = parenthesized!(content in input);
            fields = content.parse_terminated(SingletonField::parse_unnamed)?;

            lookahead = input.lookahead1();
            if lookahead.peek(Token![where]) {
                generics.where_clause = Some(input.parse()?);
                lookahead = input.lookahead1();
            }

            if lookahead.peek(Semi) {
                style = StructStyle::Tuple(paren_token, input.parse()?);
            } else {
                return Err(lookahead.error());
            }
        } else if lookahead.peek(Brace) {
            let content;
            let brace_token = braced!(content in input);
            style = StructStyle::Regular(brace_token);
            fields = content.parse_terminated(SingletonField::parse_named)?;
        } else if lookahead.peek(Semi) {
            style = StructStyle::Unit(input.parse()?);
            fields = Punctuated::new();
        } else {
            return Err(lookahead.error());
        }

        let mut impls = Vec::new();
        while !input.is_empty() {
            impls.push(parse_impl(None, input)?);
        }

        Ok(ImplSingleton {
            attrs,
            token,
            generics,
            style,
            fields,
            impls,
        })
    }
}

impl SingletonField {
    fn parse_ty(input: ParseStream) -> Result<ChildType> {
        if input.peek(Token![for]) {
            // internal generic
            let _: Token![for] = input.parse()?;

            // copied from syn::Generic's Parse impl
            let _: Token![<] = input.parse()?;

            let mut params = Punctuated::new();
            let mut allow_lifetime_param = true;
            let mut allow_type_param = true;
            loop {
                if input.peek(Token![>]) {
                    break;
                }

                let attrs = input.call(Attribute::parse_outer)?;
                let lookahead = input.lookahead1();
                if allow_lifetime_param && lookahead.peek(Lifetime) {
                    params.push_value(GenericParam::Lifetime(LifetimeDef {
                        attrs,
                        ..input.parse()?
                    }));
                } else if allow_type_param && lookahead.peek(Ident) {
                    allow_lifetime_param = false;
                    params.push_value(GenericParam::Type(TypeParam {
                        attrs,
                        ..input.parse()?
                    }));
                } else if lookahead.peek(Token![const]) {
                    allow_lifetime_param = false;
                    allow_type_param = false;
                    params.push_value(GenericParam::Const(ConstParam {
                        attrs,
                        ..input.parse()?
                    }));
                } else {
                    return Err(lookahead.error());
                }

                if input.peek(Token![>]) {
                    break;
                }
                let punct = input.parse()?;
                params.push_punct(punct);
            }

            let _: Token![>] = input.parse()?;

            let ty = input.parse()?;
            Ok(ChildType::InternGeneric(params, ty))
        } else if input.peek(Token![impl]) {
            // generic with trait bound
            let _: Token![impl] = input.parse()?;
            let bound: TypeTraitObject = input.parse()?;
            Ok(ChildType::Generic(Some(bound)))
        } else {
            Ok(ChildType::Fixed(input.parse()?))
        }
    }

    fn parse_named(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;

        let ident = if input.peek(Token![_]) {
            let _: Token![_] = input.parse()?;
            None
        } else {
            Some(input.parse::<Ident>()?)
        };

        let mut colon_token = None;

        // Note: Colon matches `::` but that results in confusing error messages
        let ty = if input.peek(Colon) && !input.peek2(Colon) {
            colon_token = Some(input.parse()?);
            Self::parse_ty(input)?
        } else {
            ChildType::Generic(None)
        };

        let mut value = None;
        if let Ok(_) = input.parse::<Eq>() {
            value = Some(input.parse()?);
        } else if !matches!(&ty, ChildType::Fixed(_)) {
            return Err(Error::new(
                input.span(),
                "require either a fixed type or a value assignment",
            ));
        }

        Ok(SingletonField {
            attrs,
            vis,
            ident,
            colon_token,
            ty,
            value,
        })
    }

    fn parse_unnamed(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;

        let mut ty = Self::parse_ty(input)?;
        if ty == ChildType::Fixed(parse_quote! { _ }) {
            ty = ChildType::Generic(None);
        }

        let mut value = None;
        if let Ok(_) = input.parse::<Eq>() {
            value = Some(input.parse()?);
        } else if !matches!(&ty, ChildType::Fixed(_)) {
            return Err(Error::new(
                input.span(),
                "require either a fixed type or a value assignment",
            ));
        }

        Ok(SingletonField {
            attrs,
            vis,
            ident: None,
            colon_token: None,
            ty,
            value,
        })
    }
}
