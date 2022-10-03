// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::quote_spanned;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq, Paren, Semi};
use syn::{braced, bracketed, parenthesized, parse_quote};
use syn::{Attribute, Expr, Generics, Ident, ItemImpl, Member, Path, Token, Type};

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
            || (input.peek2(Ident) || input.peek2(syn::Lifetime))
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
            if let Type::Path(syn::TypePath { qself: None, path }) = first_ty {
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
            if !matches!(self_ty, Type::Path(syn::TypePath {
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
        let style = syn::AttrStyle::Inner(input.parse()?);
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
    custom_keyword!(navigable);
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

#[derive(Debug)]
pub enum StructStyle {
    Unit(Semi),
    Tuple(Paren, Semi),
    Regular(Brace),
}

#[derive(Debug)]
pub struct SingletonField {
    pub attrs: Vec<Attribute>,
    pub vis: syn::Visibility,
    pub ident: Option<Ident>,
    pub colon_token: Option<Colon>,
    pub ty: Type,
    pub assignment: Option<(Eq, Expr)>,
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
    fn check_is_fixed(ty: &Type, input_span: Span) -> Result<()> {
        let is_fixed = match ty {
            Type::ImplTrait(_) | Type::Infer(_) => false,
            ty => {
                struct IsFixed(bool);
                let mut checker = IsFixed(true);

                impl<'ast> syn::visit::Visit<'ast> for IsFixed {
                    fn visit_type(&mut self, node: &'ast Type) {
                        if matches!(node, Type::ImplTrait(_) | Type::Infer(_)) {
                            self.0 = false;
                        }
                    }
                }
                syn::visit::visit_type(&mut checker, &ty);

                checker.0
            }
        };

        if is_fixed {
            Ok(())
        } else {
            Err(Error::new(
                input_span,
                "require either a fixed type or a value assignment",
            ))
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
            input.parse()?
        } else {
            parse_quote! { _ }
        };

        let mut assignment = None;
        if let Ok(eq) = input.parse::<Eq>() {
            assignment = Some((eq, input.parse()?));
        } else {
            Self::check_is_fixed(&ty, input.span())?;
        }

        Ok(SingletonField {
            attrs,
            vis,
            ident,
            colon_token,
            ty,
            assignment,
        })
    }

    fn parse_unnamed(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;

        let ty = input.parse()?;

        let mut assignment = None;
        if let Ok(eq) = input.parse::<Eq>() {
            assignment = Some((eq, input.parse()?));
        } else {
            Self::check_is_fixed(&ty, input.span())?;
        }

        Ok(SingletonField {
            attrs,
            vis,
            ident: None,
            colon_token: None,
            ty,
            assignment,
        })
    }
}
