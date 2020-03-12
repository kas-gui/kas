// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use std::collections::HashMap;

use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq, Impl, Paren, Pound, RArrow, Struct, Underscore, Where};
use syn::{braced, bracketed, parenthesized, parse_quote};
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident,
    ImplItemMethod, Index, Lit, Member, Type, TypePath, TypeTraitObject,
};

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

pub struct Args {
    pub core: Member,
    pub layout_data: Option<Member>,
    pub widget: Option<WidgetArgs>,
    pub layout: Option<LayoutArgs>,
    pub handler: Vec<HandlerArgs>,
    pub children: Vec<Child>,
}

pub fn read_attrs(ast: &mut DeriveInput) -> Result<Args> {
    let not_struct_err = |span| {
        Err(Error::new(
            span,
            "cannot derive Widget on an enum, union or unit struct",
        ))
    };
    let (fields, span) = match &mut ast.data {
        Data::Struct(data) => match &mut data.fields {
            Fields::Named(FieldsNamed {
                brace_token: Brace { span },
                named: fields,
            })
            | Fields::Unnamed(FieldsUnnamed {
                paren_token: Paren { span },
                unnamed: fields,
            }) => (fields, span),
            Fields::Unit => return not_struct_err(data.struct_token.span()),
        },
        Data::Enum(data) => return not_struct_err(data.enum_token.span()),
        Data::Union(data) => return not_struct_err(data.union_token.span()),
    };

    let mut core = None;
    let mut layout_data = None;
    let mut children = vec![];

    for (i, field) in fields.iter_mut().enumerate() {
        for attr in field.attrs.drain(..) {
            if attr.path == parse_quote! { core } {
                if core.is_none() {
                    core = Some(member(i, field.ident.clone()));
                } else {
                    attr.span()
                        .unwrap()
                        .error("multiple fields marked with #[core]")
                        .emit();
                }
            } else if attr.path == parse_quote! { layout_data } {
                if layout_data.is_none() {
                    if field.ty != parse_quote! { <Self as kas::LayoutData>::Data }
                        && field.ty != parse_quote! { <Self as LayoutData>::Data }
                    {
                        field
                            .ty
                            .span()
                            .unwrap()
                            .warning("expected type `<Self as kas::LayoutData>::Data`")
                            .emit();
                    }
                    layout_data = Some(member(i, field.ident.clone()));
                } else {
                    attr.span()
                        .unwrap()
                        .error("multiple fields marked with #[layout_data]")
                        .emit();
                }
            } else if attr.path == parse_quote! { widget } {
                let ident = member(i, field.ident.clone());
                let args = syn::parse2(attr.tokens)?;
                children.push(Child { ident, args });
            }
        }
    }

    let mut widget = None;
    let mut layout = None;
    let mut handler = vec![];

    for attr in ast.attrs.drain(..) {
        if attr.path == parse_quote! { widget } {
            if widget.is_none() {
                widget = Some(syn::parse2(attr.tokens)?);
            } else {
                attr.span()
                    .unwrap()
                    .error("multiple #[widget(..)] attributes on type")
                    .emit()
            }
        } else if attr.path == parse_quote! { layout } {
            if layout.is_none() {
                layout = Some(syn::parse2(attr.tokens)?);
            } else {
                attr.span()
                    .unwrap()
                    .error("multiple #[layout(..)] attributes on type")
                    .emit()
            }
        } else if attr.path == parse_quote! { handler } {
            handler.push(syn::parse2(attr.tokens)?);
        }
    }

    if let Some(core) = core {
        Ok(Args {
            core,
            layout_data,
            widget,
            layout,
            handler,
            children,
        })
    } else {
        Err(Error::new(
            *span,
            "one field must be marked with #[core] when deriving Widget",
        ))
    }
}

fn member(index: usize, ident: Option<Ident>) -> Member {
    match ident {
        None => Member::Unnamed(Index {
            index: index as u32,
            span: Span::call_site(),
        }),
        Some(ident) => Member::Named(ident),
    }
}

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(area);
    custom_keyword!(layout);
    custom_keyword!(col);
    custom_keyword!(row);
    custom_keyword!(cspan);
    custom_keyword!(rspan);
    custom_keyword!(widget);
    custom_keyword!(handler);
    custom_keyword!(msg);
    custom_keyword!(generics);
    custom_keyword!(single);
    custom_keyword!(horizontal);
    custom_keyword!(vertical);
    custom_keyword!(grid);
    custom_keyword!(substitutions);
    custom_keyword!(halign);
    custom_keyword!(valign);
}

#[derive(Debug)]
pub struct WidgetAttrArgs {
    pub col: Option<Lit>,
    pub row: Option<Lit>,
    pub cspan: Option<Lit>,
    pub rspan: Option<Lit>,
    pub halign: Option<Ident>,
    pub valign: Option<Ident>,
    pub handler: Option<Ident>,
}

#[derive(Debug)]
pub struct GridPos(pub u32, pub u32, pub u32, pub u32);

impl WidgetAttrArgs {
    // Parse widget position, filling in missing information with defaults.
    pub fn as_pos(&self) -> Result<GridPos> {
        fn parse_lit(lit: &Lit) -> Result<u32> {
            match lit {
                Lit::Int(li) => li.base10_parse(),
                _ => Err(Error::new(lit.span(), "expected integer literal")),
            }
        }

        Ok(GridPos(
            self.col.as_ref().map(parse_lit).unwrap_or(Ok(0))?,
            self.row.as_ref().map(parse_lit).unwrap_or(Ok(0))?,
            self.cspan.as_ref().map(parse_lit).unwrap_or(Ok(1))?,
            self.rspan.as_ref().map(parse_lit).unwrap_or(Ok(1))?,
        ))
    }

    fn match_align(ident: &Ident) -> Result<TokenStream> {
        Ok(match ident {
            ident if ident == "begin" => quote! { kas::Align::Begin },
            ident if ident == "centre" || ident == "center" => quote! { kas::Align::Centre },
            ident if ident == "end" => quote! { kas::Align::End },
            ident if ident == "stretch" => quote! { kas::Align::Stretch },
            ident => {
                return Err(Error::new(
                    ident.span(),
                    "expected one of `begin`, `centre`, `center`, `end`, `stretch`",
                ));
            }
        })
    }
    pub fn halign_toks(&self) -> Result<Option<TokenStream>> {
        if let Some(ref ident) = self.halign {
            Ok(Some(Self::match_align(ident)?))
        } else {
            Ok(None)
        }
    }
    pub fn valign_toks(&self) -> Result<Option<TokenStream>> {
        if let Some(ref ident) = self.valign {
            Ok(Some(Self::match_align(ident)?))
        } else {
            Ok(None)
        }
    }
}

impl Parse for WidgetAttrArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = WidgetAttrArgs {
            col: None,
            row: None,
            cspan: None,
            rspan: None,
            halign: None,
            valign: None,
            handler: None,
        };
        if input.is_empty() {
            return Ok(args);
        }

        let content;
        let _ = parenthesized!(content in input);

        loop {
            let lookahead = content.lookahead1();
            if args.col.is_none() && lookahead.peek(kw::col) {
                let _: kw::col = content.parse()?;
                let _: Eq = content.parse()?;
                args.col = Some(content.parse()?);
            } else if args.row.is_none() && lookahead.peek(kw::row) {
                let _: kw::row = content.parse()?;
                let _: Eq = content.parse()?;
                args.row = Some(content.parse()?);
            } else if args.cspan.is_none() && lookahead.peek(kw::cspan) {
                let _: kw::cspan = content.parse()?;
                let _: Eq = content.parse()?;
                args.cspan = Some(content.parse()?);
            } else if args.rspan.is_none() && lookahead.peek(kw::rspan) {
                let _: kw::rspan = content.parse()?;
                let _: Eq = content.parse()?;
                args.rspan = Some(content.parse()?);
            } else if args.halign.is_none() && lookahead.peek(kw::halign) {
                let _: kw::halign = content.parse()?;
                let _: Eq = content.parse()?;
                args.halign = Some(content.parse()?);
            } else if args.valign.is_none() && lookahead.peek(kw::valign) {
                let _: kw::valign = content.parse()?;
                let _: Eq = content.parse()?;
                args.valign = Some(content.parse()?);
            } else if args.handler.is_none() && lookahead.peek(kw::handler) {
                let _: kw::handler = content.parse()?;
                let _: Eq = content.parse()?;
                args.handler = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }

        Ok(args)
    }
}

impl ToTokens for WidgetAttrArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.col.is_some()
            || self.row.is_some()
            || self.cspan.is_some()
            || self.rspan.is_some()
            || self.halign.is_some()
            || self.valign.is_some()
            || self.handler.is_some()
        {
            let comma = TokenTree::from(Punct::new(',', Spacing::Alone));
            let mut args = TokenStream::new();
            if let Some(ref lit) = self.col {
                args.append_all(quote! { col = #lit });
            }
            if let Some(ref lit) = self.row {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { row = #lit });
            }
            if let Some(ref lit) = self.cspan {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { cspan = #lit });
            }
            if let Some(ref lit) = self.rspan {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { rspan = #lit });
            }
            if let Some(ref ident) = self.halign {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { halign = #ident });
            }
            if let Some(ref ident) = self.valign {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { valign = #ident });
            }
            if let Some(ref ident) = self.handler {
                if !args.is_empty() {
                    args.append(comma);
                }
                args.append_all(quote! { handler = #ident });
            }
            tokens.append_all(quote! { ( #args ) });
        }
    }
}

#[derive(Debug)]
pub struct WidgetAttr {
    pub args: WidgetAttrArgs,
}

impl ToTokens for WidgetAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let args = &self.args;
        tokens.append_all(quote! { #[widget #args] });
    }
}

impl ToTokens for GridPos {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (c, r, cs, rs) = (&self.0, &self.1, &self.2, &self.3);
        tokens.append_all(quote! { (#c, #r, #cs, #rs) });
    }
}

pub struct WidgetArgs {}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(WidgetArgs {});
        }

        let content;
        let _ = parenthesized!(content in input);

        if !content.is_empty() {
            return Err(Error::new(content.span(), "unexpected content"));
        }

        Ok(WidgetArgs {})
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayoutType {
    Single,
    Horizontal,
    Vertical,
    Grid,
}

pub struct LayoutArgs {
    pub span: Span,
    pub layout: LayoutType,
    pub area: Option<Ident>,
}

impl Parse for LayoutArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::new(
                input.span(),
                "expected attribute parameters: `(..)`",
            ));
        }

        let span = input.span();

        let content;
        let _ = parenthesized!(content in input);

        let lookahead = content.lookahead1();
        let layout = if lookahead.peek(kw::single) {
            let _: kw::single = content.parse()?;
            LayoutType::Single
        } else if lookahead.peek(kw::horizontal) {
            let _: kw::horizontal = content.parse()?;
            LayoutType::Horizontal
        } else if lookahead.peek(kw::vertical) {
            let _: kw::vertical = content.parse()?;
            LayoutType::Vertical
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = content.parse()?;
            LayoutType::Grid
        } else {
            return Err(lookahead.error());
        };

        if content.peek(Comma) {
            let _: Comma = content.parse()?;
        }

        let mut area = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if area.is_none() && lookahead.peek(kw::area) {
                let _: kw::area = content.parse()?;
                let _: Eq = content.parse()?;
                area = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Comma) {
                let _: Comma = content.parse()?;
            }
        }

        Ok(LayoutArgs { span, layout, area })
    }
}

pub struct HandlerArgs {
    pub msg: Type,
    pub substitutions: HashMap<Ident, Type>,
    pub generics: Generics,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let (mut have_msg, mut have_subs, mut have_gen) = (false, false, false);
        let mut msg = parse_quote! { kas::event::VoidMsg };
        let mut substitutions = HashMap::new();
        let mut generics = Generics::default();

        if input.is_empty() {
            return Ok(HandlerArgs {
                msg,
                substitutions,
                generics,
            });
        }

        let content;
        let _ = parenthesized!(content in input);

        // If we have a where clause, that will greedily consume remaining
        // input. Because of this, `generics = ...` must come last.

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if !have_msg && lookahead.peek(kw::msg) {
                have_msg = true;
                let _: kw::msg = content.parse()?;
                let _: Eq = content.parse()?;
                msg = content.parse()?;
            } else if !have_subs && lookahead.peek(kw::substitutions) {
                have_subs = true;
                let _: kw::substitutions = content.parse()?;
                let _: Eq = content.parse()?;
                let content2;
                let _ = parenthesized!(content2 in content);
                while !content2.is_empty() {
                    // TODO: ideally we should support substitution of lifetime and
                    // const generic parameters too.
                    let ident: Ident = content2.parse()?;
                    let _: Eq = content2.parse()?;
                    let ty: Type = content2.parse()?;
                    if content2.peek(Comma) {
                        let _: Comma = content2.parse()?;
                    }
                    substitutions.insert(ident, ty);
                }
            } else if !have_gen && lookahead.peek(kw::generics) {
                have_gen = true;
                let _: kw::generics = content.parse()?;
                let _: Eq = content.parse()?;
                generics = content.parse()?;
                if content.peek(Where) {
                    generics.where_clause = content.parse()?;
                    // Last pass should consume all content
                    if !content.is_empty() {
                        return Err(Error::new(
                            content.span(),
                            "no content expected after where clause",
                        ));
                    }
                    break;
                }
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Comma) {
                let _: Comma = content.parse()?;
            }
        }

        Ok(HandlerArgs {
            msg,
            substitutions,
            generics,
        })
    }
}

pub enum ChildType {
    Fixed(Type), // fixed type
    // Generic, optionally with specified handler msg type,
    // optionally with an additional trait bound.
    Generic(Option<Type>, Option<TypeTraitObject>),
}

pub struct WidgetField {
    pub widget_attr: Option<WidgetAttr>,
    pub ident: Option<Ident>,
    pub ty: ChildType,
    pub value: Expr,
}

struct HandlerAttrToks {
    msg: Type,
}

impl Parse for HandlerAttrToks {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = parenthesized!(content in input);
        let _: kw::msg = content.parse()?;
        let _: Eq = content.parse()?;
        let msg = content.parse()?;

        Ok(HandlerAttrToks { msg })
    }
}

pub struct MakeWidget {
    // handler: Msg type
    pub handler_msg: Type,
    // additional attributes
    pub extra_attrs: TokenStream,
    pub generics: Generics,
    // child widgets and data fields
    pub fields: Vec<WidgetField>,
    // impl blocks on the widget
    pub impls: Vec<(Option<TypePath>, Vec<ImplItemMethod>)>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut handler_msg = None;
        let mut extra_attrs = TokenStream::new();
        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.drain(..) {
            if attr.path == parse_quote! { handler } {
                if handler_msg.is_some() {
                    return Err(Error::new(attr.span(), "duplicate `handler` attribute"));
                }
                let hat: HandlerAttrToks = syn::parse2(attr.tokens)?;
                handler_msg = Some(hat.msg);
            } else {
                extra_attrs.append_all(quote! { #attr });
            }
        }

        let handler_msg = if let Some(path) = handler_msg {
            path
        } else {
            return Err(Error::new(
                input.span(),
                "expected `#[handler ..]` attribute",
            ));
        };

        let _: Struct = input.parse()?;

        let mut generics: syn::Generics = input.parse()?;
        if input.peek(syn::token::Where) {
            generics.where_clause = Some(input.parse()?);
        }

        let content;
        let _ = braced!(content in input);
        let mut fields = vec![];

        while !content.is_empty() {
            fields.push(content.parse::<WidgetField>()?);

            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }

        let mut impls = vec![];
        while !input.is_empty() {
            let _: Impl = input.parse()?;

            let target = if input.peek(Brace) {
                None
            } else {
                Some(input.parse::<TypePath>()?)
            };

            let content;
            let _ = braced!(content in input);
            let mut methods = vec![];

            while !content.is_empty() {
                methods.push(content.parse::<ImplItemMethod>()?);
            }

            impls.push((target, methods));
        }

        Ok(MakeWidget {
            handler_msg,
            extra_attrs,
            generics,
            fields,
            impls,
        })
    }
}

impl Parse for WidgetField {
    fn parse(input: ParseStream) -> Result<Self> {
        let widget_attr = if input.peek(Pound) {
            let _: Pound = input.parse()?;
            let inner;
            let _ = bracketed!(inner in input);
            let _: kw::widget = inner.parse()?;
            let args = inner.parse::<WidgetAttrArgs>()?;
            Some(WidgetAttr { args })
        } else {
            None
        };

        let ident = {
            let lookahead = input.lookahead1();
            if lookahead.peek(Underscore) {
                let _: Underscore = input.parse()?;
                None
            } else if lookahead.peek(Ident) {
                Some(input.parse::<Ident>()?)
            } else {
                return Err(lookahead.error());
            }
        };

        // Note: Colon matches `::` but that results in confusing error messages
        let mut ty = if input.peek(Colon) && !input.peek2(Colon) {
            let _: Colon = input.parse()?;
            if input.peek(Impl) {
                // generic with trait bound, optionally with msg type
                let _: Impl = input.parse()?;
                let bound: TypeTraitObject = input.parse()?;
                ChildType::Generic(None, Some(bound))
            } else {
                ChildType::Fixed(input.parse()?)
            }
        } else {
            ChildType::Generic(None, None)
        };

        if input.peek(RArrow) {
            let arrow: RArrow = input.parse()?;
            if !widget_attr.is_some() {
                return Err(Error::new(
                    arrow.span(),
                    "can only use `-> Msg` type restriction on widgets",
                ));
            }
            let msg: Type = input.parse()?;
            match &mut ty {
                ChildType::Fixed(_) => {
                    return Err(Error::new(
                        arrow.span(),
                        "cannot use `-> Msg` type restriction with fixed type",
                    ));
                }
                ChildType::Generic(ref mut gen_r, _) => {
                    *gen_r = Some(msg);
                }
            }
        }

        let _: Eq = input.parse()?;
        let value: Expr = input.parse()?;

        Ok(WidgetField {
            widget_attr,
            ident,
            ty,
            value,
        })
    }
}
