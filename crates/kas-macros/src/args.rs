// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, emit_warning};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq, For, Impl, Paren, Semi};
use syn::{braced, bracketed, parenthesized, parse_quote};
use syn::{
    AttrStyle, Attribute, ConstParam, Expr, Field, Fields, FieldsNamed, GenericParam, Generics,
    Ident, Index, ItemImpl, Lifetime, LifetimeDef, Lit, Member, Path, Token, Type, TypeParam,
    TypePath, TypeTraitObject, Visibility,
};

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

#[derive(Debug)]
pub struct Widget {
    pub attr_derive: WidgetDerive,
    pub attr_widget: WidgetArgs,
    pub attr_layout: Option<LayoutArgs>,
    pub attr_handler: Option<HandlerArgs>,
    pub extra_attrs: Vec<Attribute>,

    pub vis: Visibility,
    pub token: Token![struct],
    pub ident: Ident,
    pub generics: Generics,
    pub fields: Fields,
    pub semi_token: Option<Semi>,

    pub core_data: Option<Member>,
    pub layout_data: Option<Member>,
    pub inner: Option<(Member, Type)>,
    pub children: Vec<Child>,

    pub extra_impls: Vec<ItemImpl>,
}

impl Parse for Widget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attr_derive: Option<(Span, WidgetDerive)> = None;
        let mut attr_widget = None;
        let mut attr_layout = None;
        let mut attr_handler = None;
        let mut extra_attrs = Vec::new();

        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.drain(..) {
            if attr.path == parse_quote! { widget_derive } {
                if attr_derive.is_none() {
                    let span = attr.span();
                    let attr = syn::parse2(attr.tokens)?;
                    attr_derive = Some((span, attr));
                } else {
                    emit_error!(
                        attr.span(),
                        "multiple #[widget_derive(..)] attributes on type"
                    );
                }
            } else if attr.path == parse_quote! { widget } {
                if attr_widget.is_none() {
                    let _span = attr.span();
                    let w: WidgetArgs = syn::parse2(attr.tokens)?;
                    attr_widget = Some(w);
                } else {
                    emit_error!(attr.span(), "multiple #[widget(..)] attributes on type");
                }
            } else if attr.path == parse_quote! { layout } {
                if attr_layout.is_some() {
                    emit_error!(attr.span(), "multiple #[layout(..)] attributes on type");
                } else {
                    attr_layout = Some(syn::parse2(attr.tokens)?);
                }
            } else if attr.path == parse_quote! { handler } {
                if attr_handler.is_some() {
                    emit_error!(attr.span(), "multiple #[handler(..)] attributes on type");
                } else {
                    attr_handler = Some(syn::parse2(attr.tokens)?);
                }
            } else {
                extra_attrs.push(attr);
            }
        }

        let vis = input.parse::<Visibility>()?;
        let token = input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;

        let mut generics = input.parse::<Generics>()?;
        let (mut fields, semi_token) = {
            let mut lookahead = input.lookahead1();
            if lookahead.peek(Token![where]) {
                generics.where_clause = Some(input.parse()?);
                lookahead = input.lookahead1();
            }

            if generics.where_clause.is_none() && lookahead.peek(Paren) {
                let fields = input.parse()?;

                lookahead = input.lookahead1();
                if lookahead.peek(Token![where]) {
                    generics.where_clause = Some(input.parse()?);
                    lookahead = input.lookahead1();
                }

                if lookahead.peek(Token![;]) {
                    let semi = input.parse()?;
                    (Fields::Unnamed(fields), Some(semi))
                } else {
                    return Err(lookahead.error());
                }
            } else if lookahead.peek(Brace) {
                let content;
                let brace_token = braced!(content in input);
                let named = content.parse_terminated(Field::parse_named)?;
                let fields = Fields::Named(FieldsNamed { brace_token, named });
                (fields, None)
            } else {
                return Err(lookahead.error());
            }
        };

        let mut extra_impls = Vec::new();
        while !input.is_empty() {
            extra_impls.push(parse_impl(Some(&ident), input)?);
        }

        let mut core_data = None;
        let mut layout_data = None;
        let mut inner = None;
        let mut children = Vec::new();

        for (i, field) in fields.iter_mut().enumerate() {
            let mut other_attrs = Vec::with_capacity(field.attrs.len());
            for attr in field.attrs.drain(..) {
                if attr.path == parse_quote! { widget_core } {
                    if core_data.is_none() {
                        core_data = Some(member(i, field.ident.clone()));
                    } else {
                        emit_error!(attr.span(), "multiple fields marked with #[widget_core]");
                    }
                } else if attr.path == parse_quote! { layout_data } {
                    if layout_data.is_some() {
                        emit_error!(attr.span(), "multiple fields marked with #[layout_data]");
                    } else if field.ty != parse_quote! { <Self as kas::LayoutData>::Data }
                        && field.ty != parse_quote! { <Self as ::kas::LayoutData>::Data }
                        && field.ty != parse_quote! { <Self as LayoutData>::Data }
                    {
                        emit_warning!(
                            field.ty.span(),
                            "expected type `<Self as kas::LayoutData>::Data`"
                        );
                    } else {
                        layout_data = Some(member(i, field.ident.clone()));
                    }
                } else if attr.path == parse_quote! { widget_derive } {
                    if inner.is_none() {
                        inner = Some((member(i, field.ident.clone()), field.ty.clone()));
                    } else {
                        emit_error!(attr.span(), "multiple fields marked with #[widget_derive]");
                    }
                } else if attr.path == parse_quote! { widget } {
                    let ident = member(i, field.ident.clone());
                    let args = syn::parse2(attr.tokens)?;
                    children.push(Child { ident, args });
                } else {
                    other_attrs.push(attr);
                }
            }
            field.attrs = other_attrs;
        }

        let attr_widget = attr_widget.unwrap_or_default();

        if core_data.is_none() {
            if inner.is_none() {
                emit_error!(
                    fields.span(),
                    "require a field with #[widget_core] or a field with #[widget_derive] or both",
                );
            }
            if layout_data.is_some() || !children.is_empty() {
                emit_error!(
                    fields.span(),
                    "require a field with #[widget_core] when using #[layout_data] or #[widget]",
                );
            }
        }

        if core_data.is_some() && inner.is_some() && attr_derive.is_none() {
            emit_error!(
                fields.span(),
                "usage of #[widget_derive] field with #[widget_core] field and without #[widget_derive(..)] on struct has no effect",
            );
        }

        if let Some((ref span, _)) = attr_derive {
            if inner.is_none() {
                emit_error!(span, "usage of #[widget_derive(..)] on struct without a field marked with #[widget_derive]");
            }
        }

        let attr_derive = attr_derive.map(|(_, attr)| attr).unwrap_or_default();

        Ok(Widget {
            attr_derive,
            attr_widget,
            attr_layout,
            attr_handler,
            extra_attrs,
            vis,
            token,
            ident,
            generics,
            fields,
            semi_token,
            core_data,
            layout_data,
            inner,
            children,
            extra_impls,
        })
    }
}

/// Prints out the widget struct, but not other fields
impl ToTokens for Widget {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(self.extra_attrs.iter());
        self.vis.to_tokens(tokens);
        self.token.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.generics.to_tokens(tokens);
        match &self.fields {
            Fields::Named(fields) => {
                self.generics.where_clause.to_tokens(tokens);
                fields.to_tokens(tokens);
            }
            Fields::Unnamed(fields) => {
                fields.to_tokens(tokens);
                self.generics.where_clause.to_tokens(tokens);
                self.semi_token.unwrap_or_default().to_tokens(tokens);
            }
            Fields::Unit => {
                unreachable!()
                // self.generics.where_clause.to_tokens(tokens);
                // self.semi_token.unwrap_or_default().to_tokens(tokens);
            }
        }
    }
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
    Ok(while input.peek(Token![#]) && input.peek2(Token![!]) {
        let content;
        attrs.push(Attribute {
            pound_token: input.parse()?,
            style: AttrStyle::Inner(input.parse()?),
            bracket_token: bracketed!(content in input),
            path: content.call(Path::parse_mod_style)?,
            tokens: content.parse()?,
        });
    })
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
    custom_keyword!(flatmap_msg);
    custom_keyword!(map_msg);
    custom_keyword!(use_msg);
    custom_keyword!(discard_msg);
    custom_keyword!(update);
    custom_keyword!(msg);
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
    custom_keyword!(HasBool);
    custom_keyword!(HasStr);
    custom_keyword!(HasString);
    custom_keyword!(SetAccel);
    custom_keyword!(class_traits);
    custom_keyword!(Deref);
    custom_keyword!(DerefMut);
}

#[derive(Debug, Default)]
pub struct WidgetDerive {
    pub has_bool: bool,
    pub has_str: bool,
    pub has_string: bool,
    pub set_accel: bool,
    pub deref: bool,
    pub deref_mut: bool,
}

impl Parse for WidgetDerive {
    fn parse(input: ParseStream) -> Result<Self> {
        // Note: this parser deliberately cannot return WidgetDerive::default()
        let mut derive = WidgetDerive::default();
        let content;
        let _ = parenthesized!(content in input);

        loop {
            let lookahead = content.lookahead1();
            if !derive.has_bool && lookahead.peek(kw::HasBool) {
                let _: kw::HasBool = content.parse()?;
                derive.has_bool = true;
            } else if !derive.has_str && lookahead.peek(kw::HasStr) {
                let _: kw::HasStr = content.parse()?;
                derive.has_str = true;
            } else if !derive.has_string && lookahead.peek(kw::HasString) {
                let _: kw::HasString = content.parse()?;
                derive.has_string = true;
            } else if !derive.set_accel && lookahead.peek(kw::SetAccel) {
                let _: kw::SetAccel = content.parse()?;
                derive.set_accel = true;
            } else if !derive.has_bool
                && !derive.has_str
                && !derive.has_string
                && !derive.set_accel
                && lookahead.peek(kw::class_traits)
            {
                let _: kw::class_traits = content.parse()?;
                derive.has_bool = true;
                derive.has_str = true;
                derive.has_string = true;
                derive.set_accel = true;
            } else if !derive.deref && lookahead.peek(kw::Deref) {
                let _: kw::Deref = content.parse()?;
                derive.deref = true;
            } else if !derive.deref_mut && lookahead.peek(kw::DerefMut) {
                let _: kw::DerefMut = content.parse()?;
                derive.deref_mut = true;
            } else {
                return Err(lookahead.error());
            }

            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }

        Ok(derive)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Handler {
    None,
    Use(Ident),
    Map(Ident),
    FlatMap(Ident),
    Discard,
}
impl Handler {
    pub fn is_none(&self) -> bool {
        *self == Handler::None
    }
    pub fn any_ref(&self) -> Option<&Ident> {
        match self {
            Handler::None | Handler::Discard => None,
            Handler::Use(n) | Handler::Map(n) | Handler::FlatMap(n) => Some(n),
        }
    }
}

#[derive(Debug)]
pub struct WidgetAttrArgs {
    pub col: Option<Lit>,
    pub row: Option<Lit>,
    pub cspan: Option<Lit>,
    pub rspan: Option<Lit>,
    pub halign: Option<Ident>,
    pub valign: Option<Ident>,
    pub update: Option<Ident>,
    pub handler: Handler,
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

    fn match_align(ident: &Ident, horiz: bool) -> Result<TokenStream> {
        Ok(match ident {
            ident if ident == "default" => quote! { ::kas::layout::Align::Default },
            ident if horiz && ident == "left" => quote! { ::kas::layout::Align::TL },
            ident if !horiz && ident == "top" => quote! { ::kas::layout::Align::TL },
            ident if ident == "centre" || ident == "center" => {
                quote! { ::kas::layout::Align::Centre }
            }
            ident if horiz && ident == "right" => quote! { ::kas::layout::Align::BR },
            ident if !horiz && ident == "bottom" => quote! { ::kas::layout::Align::BR },
            ident if ident == "stretch" => quote! { ::kas::layout::Align::Stretch },
            ident => {
                return Err(Error::new(
                    ident.span(),
                    "expected one of `default`, `centre`, `stretch`, `top` or `bottom` (if vertical), `left` or `right` (if horizontal)",
                ));
            }
        })
    }
    pub fn halign_toks(&self) -> Result<Option<TokenStream>> {
        if let Some(ref ident) = self.halign {
            Ok(Some(Self::match_align(ident, true)?))
        } else {
            Ok(None)
        }
    }
    pub fn valign_toks(&self) -> Result<Option<TokenStream>> {
        if let Some(ref ident) = self.valign {
            Ok(Some(Self::match_align(ident, false)?))
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
            update: None,
            handler: Handler::None,
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
            } else if args.col.is_none() && lookahead.peek(kw::column) {
                let _: kw::column = content.parse()?;
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
            } else if args.halign.is_none() && args.valign.is_none() && lookahead.peek(kw::align) {
                let _: kw::align = content.parse()?;
                let _: Eq = content.parse()?;
                let ident: Ident = content.parse()?;
                if ident == "centre" || ident == "center" || ident == "stretch" {
                    args.halign = Some(ident.clone());
                    args.valign = Some(ident);
                } else {
                    return Err(Error::new(ident.span(), "expected `centre` or `center`"));
                }
            } else if args.halign.is_none() && lookahead.peek(kw::halign) {
                let _: kw::halign = content.parse()?;
                let _: Eq = content.parse()?;
                args.halign = Some(content.parse()?);
            } else if args.valign.is_none() && lookahead.peek(kw::valign) {
                let _: kw::valign = content.parse()?;
                let _: Eq = content.parse()?;
                args.valign = Some(content.parse()?);
            } else if args.update.is_none() && lookahead.peek(kw::update) {
                let _: kw::update = content.parse()?;
                let _: Eq = content.parse()?;
                args.update = Some(content.parse()?);
            } else if args.handler.is_none() && lookahead.peek(kw::flatmap_msg) {
                let _: kw::flatmap_msg = content.parse()?;
                let _: Eq = content.parse()?;
                args.handler = Handler::FlatMap(content.parse()?);
            } else if args.handler.is_none() && lookahead.peek(kw::map_msg) {
                let _: kw::map_msg = content.parse()?;
                let _: Eq = content.parse()?;
                args.handler = Handler::Map(content.parse()?);
            } else if args.handler.is_none() && lookahead.peek(kw::use_msg) {
                let _: kw::use_msg = content.parse()?;
                let _: Eq = content.parse()?;
                args.handler = Handler::Use(content.parse()?);
            } else if args.handler.is_none() && lookahead.peek(kw::discard_msg) {
                let _: kw::discard_msg = content.parse()?;
                args.handler = Handler::Discard;
            } else if lookahead.peek(kw::handler) {
                let tok: Ident = content.parse()?;
                return Err(Error::new(
                    tok.span(),
                    "handler is obsolete; replace with flatmap_msg, map_msg, use_msg or discard_msg",
                ));
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
            || !self.handler.is_none()
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
            if let Some(ref ident) = self.update {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { update = #ident });
            }
            if !self.handler.is_none() && !args.is_empty() {
                args.append(comma);
            }
            match &self.handler {
                Handler::None => (),
                Handler::Use(f) => args.append_all(quote! { use_msg = #f }),
                Handler::Map(f) => args.append_all(quote! { map_msg = #f }),
                Handler::FlatMap(f) => args.append_all(quote! { flatmap_msg = #f }),
                Handler::Discard => args.append_all(quote! { discard_msg }),
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

#[derive(Debug)]
pub struct WidgetArgs {
    pub config: Option<WidgetConfig>,
}

impl Default for WidgetArgs {
    fn default() -> Self {
        WidgetArgs {
            config: Some(WidgetConfig::default()),
        }
    }
}

#[derive(Debug)]
pub struct WidgetConfig {
    pub key_nav: bool,
    pub hover_highlight: bool,
    pub cursor_icon: Expr,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        WidgetConfig {
            key_nav: false,
            hover_highlight: false,
            cursor_icon: parse_quote! { ::kas::event::CursorIcon::Default },
        }
    }
}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut config = None;

        if !input.is_empty() {
            let content;
            let _ = parenthesized!(content in input);

            while !content.is_empty() {
                let lookahead = content.lookahead1();
                if lookahead.peek(kw::config) && config.is_none() {
                    let _: kw::config = content.parse()?;

                    let content2;
                    let _ = parenthesized!(content2 in content);

                    let mut conf = WidgetConfig::default();
                    let mut have_key_nav = false;
                    let mut have_hover_highlight = false;
                    let mut have_cursor_icon = false;

                    while !content2.is_empty() {
                        let lookahead = content2.lookahead1();
                        if lookahead.peek(kw::key_nav) && !have_key_nav {
                            let _: kw::key_nav = content2.parse()?;
                            let _: Eq = content2.parse()?;
                            let value: syn::LitBool = content2.parse()?;
                            conf.key_nav = value.value;
                            have_key_nav = true;
                        } else if lookahead.peek(kw::hover_highlight) && !have_hover_highlight {
                            let _: kw::hover_highlight = content2.parse()?;
                            let _: Eq = content2.parse()?;
                            let value: syn::LitBool = content2.parse()?;
                            conf.hover_highlight = value.value;
                            have_hover_highlight = true;
                        } else if lookahead.peek(kw::cursor_icon) && !have_cursor_icon {
                            let _: kw::cursor_icon = content2.parse()?;
                            let _: Eq = content2.parse()?;
                            conf.cursor_icon = content2.parse()?;
                            have_cursor_icon = true;
                        } else {
                            return Err(lookahead.error());
                        };

                        if content2.peek(Comma) {
                            let _: Comma = content2.parse()?;
                        }
                    }
                    config = Some(conf);
                } else {
                    return Err(lookahead.error());
                }

                if content.peek(Comma) {
                    let _: Comma = content.parse()?;
                }
            }
        }

        Ok(WidgetArgs { config })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayoutType {
    Single,
    Right,
    Left,
    Down,
    Up,
    Grid,
}

impl ToTokens for LayoutType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self {
            LayoutType::Single | LayoutType::Grid => unreachable!(),
            LayoutType::Right => quote! { ::kas::dir::Right },
            LayoutType::Left => quote! { ::kas::dir::Left },
            LayoutType::Down => quote! { ::kas::dir::Down },
            LayoutType::Up => quote! { ::kas::dir::Up },
        })
    }
}

#[derive(Debug)]
pub struct LayoutArgs {
    pub span: Span,
    pub layout: LayoutType,
    pub area: Option<Ident>,
    pub draw: Option<Ident>,
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
        } else if lookahead.peek(kw::row) {
            let _: kw::row = content.parse()?;
            LayoutType::Right
        } else if lookahead.peek(kw::right) {
            let _: kw::right = content.parse()?;
            LayoutType::Right
        } else if lookahead.peek(kw::left) {
            let _: kw::left = content.parse()?;
            LayoutType::Left
        } else if lookahead.peek(kw::col) {
            let _: kw::col = content.parse()?;
            LayoutType::Down
        } else if lookahead.peek(kw::column) {
            let _: kw::column = content.parse()?;
            LayoutType::Down
        } else if lookahead.peek(kw::down) {
            let _: kw::down = content.parse()?;
            LayoutType::Down
        } else if lookahead.peek(kw::up) {
            let _: kw::up = content.parse()?;
            LayoutType::Up
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
        let mut draw = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if area.is_none() && lookahead.peek(kw::area) {
                let _: kw::area = content.parse()?;
                let _: Eq = content.parse()?;
                area = Some(content.parse()?);
            } else if draw.is_none() && lookahead.peek(kw::draw) {
                let _: kw::draw = content.parse()?;
                let _: Eq = content.parse()?;
                draw = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Comma) {
                let _: Comma = content.parse()?;
            }
        }

        Ok(LayoutArgs {
            span,
            layout,
            area,
            draw,
        })
    }
}

#[derive(Debug)]
pub struct HandlerArgs {
    pub msg: Type,
}

impl HandlerArgs {
    pub fn new(msg: Type) -> Self {
        HandlerArgs { msg }
    }
}

impl Default for HandlerArgs {
    fn default() -> Self {
        let msg = parse_quote! { ::kas::event::VoidMsg };
        HandlerArgs::new(msg)
    }
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut msg = None;

        if !input.is_empty() {
            let content;
            let _ = parenthesized!(content in input);

            while !content.is_empty() {
                let lookahead = content.lookahead1();
                if msg.is_none() && lookahead.peek(kw::msg) {
                    let _: kw::msg = content.parse()?;
                    let _: Eq = content.parse()?;
                    msg = Some(content.parse()?);
                } else {
                    return Err(lookahead.error());
                }

                if content.peek(Comma) {
                    let _: Comma = content.parse()?;
                }
            }
        }

        if let Some(msg) = msg {
            Ok(HandlerArgs { msg })
        } else {
            Ok(HandlerArgs::default())
        }
    }
}

#[derive(Debug)]
pub enum ChildType {
    Fixed(Type), // fixed type
    // A given type using generics internally
    InternGeneric(Punctuated<GenericParam, Comma>, Type),
    // Generic, optionally with specified handler msg type,
    // optionally with an additional trait bound.
    Generic(Option<Type>, Option<TypeTraitObject>),
}

#[derive(Debug)]
pub struct WidgetField {
    pub widget_attr: Option<WidgetAttr>,
    pub ident: Option<Ident>,
    pub ty: ChildType,
    pub value: Expr,
}

#[derive(Debug)]
pub struct MakeWidget {
    // handler attribute
    pub handler: Option<HandlerArgs>,
    // additional attributes
    pub extra_attrs: TokenStream,
    pub generics: Generics,
    pub struct_span: Span,
    // child widgets and data fields
    pub fields: Vec<WidgetField>,
    // impl blocks on the widget
    pub impls: Vec<ItemImpl>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut handler = None;
        let mut extra_attrs = TokenStream::new();
        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.drain(..) {
            if attr.path == parse_quote! { handler } {
                if handler.is_some() {
                    return Err(Error::new(
                        attr.span(),
                        "multiple `handler` attributes not supported by make_widget!",
                    ));
                }
                handler = Some(syn::parse2(attr.tokens)?);
            } else {
                extra_attrs.append_all(quote! { #attr });
            }
        }

        let s: Token![struct] = input.parse()?;

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

        let mut impls = Vec::new();
        while !input.is_empty() {
            impls.push(parse_impl(None, input)?);
        }

        Ok(MakeWidget {
            handler,
            extra_attrs,
            generics,
            struct_span: s.span(),
            fields,
            impls,
        })
    }
}

impl Parse for WidgetField {
    fn parse(input: ParseStream) -> Result<Self> {
        let widget_attr = if input.peek(Token![#]) {
            let _: Token![#] = input.parse()?;
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
            if lookahead.peek(Token![_]) {
                let _: Token![_] = input.parse()?;
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
            if input.peek(For) {
                // internal generic
                let _: For = input.parse()?;

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
                ChildType::InternGeneric(params, ty)
            } else if input.peek(Impl) {
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

        if input.peek(Token![->]) {
            let arrow: Token![->] = input.parse()?;
            if widget_attr.is_none() {
                return Err(Error::new(
                    arrow.span(),
                    "can only use `-> Msg` type restriction on widgets",
                ));
            }
            let msg: Type = input.parse()?;
            match &mut ty {
                ChildType::Fixed(_) | ChildType::InternGeneric(_, _) => {
                    return Err(Error::new(
                        arrow.span(),
                        "cannot use `-> Msg` type restriction with fixed `type` or with `for<...> type`",
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
