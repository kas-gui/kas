// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, emit_error};
use quote::{ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq, For, Impl, Paren, Semi};
use syn::{braced, bracketed, parenthesized, parse_quote};
use syn::{
    AttrStyle, Attribute, ConstParam, Expr, Field, Fields, FieldsNamed, GenericParam, Generics,
    Ident, Index, ItemImpl, Lifetime, LifetimeDef, Member, Path, Token, Type, TypeParam, TypePath,
    TypeTraitObject, Visibility,
};

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

#[derive(Debug)]
pub struct Widget {
    pub attr_widget: WidgetArgs,
    pub extra_attrs: Vec<Attribute>,

    pub vis: Visibility,
    pub token: Token![struct],
    pub ident: Ident,
    pub generics: Generics,
    pub fields: Fields,
    pub semi_token: Option<Semi>,

    pub core_data: Option<Member>,
    pub children: Vec<Child>,

    pub extra_impls: Vec<ItemImpl>,
}

impl Parse for Widget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attr_widget = None;
        let mut extra_attrs = Vec::new();

        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.drain(..) {
            if attr.path == parse_quote! { widget } {
                if attr_widget.is_none() {
                    let _span = attr.span();
                    let w: WidgetArgs = syn::parse2(attr.tokens)?;
                    attr_widget = Some(w);
                } else {
                    emit_error!(attr.span(), "multiple #[widget(..)] attributes on type");
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
            if attr_widget.derive.is_none() {
                emit_error!(
                    fields.span(),
                    "require a field with #[widget_core] or #[widget(derive = FIELD)]",
                );
            }
            if !children.is_empty() {
                emit_error!(
                    fields.span(),
                    "require a field with #[widget_core] when using #[widget]",
                );
            }
        }

        if core_data.is_some() && attr_widget.derive.is_some() {
            emit_error!(
                fields.span(),
                "usage of field with #[widget_core] conflicts with #[widget(derive=FIELD)]",
            );
        }

        Ok(Widget {
            attr_widget,
            extra_attrs,
            vis,
            token,
            ident,
            generics,
            fields,
            semi_token,
            core_data,
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

    custom_keyword!(find_id);
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
    custom_keyword!(derive);
}

#[derive(Debug, Default)]
pub struct WidgetDerive;

impl Parse for WidgetDerive {
    fn parse(input: ParseStream) -> Result<Self> {
        // Note: this parser deliberately cannot return WidgetDerive::default()
        let derive = WidgetDerive::default();
        let content;
        let _ = parenthesized!(content in input);

        let lookahead = content.lookahead1();
        if !content.is_empty() {
            return Err(lookahead.error());
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
    fn is_none(&self) -> bool {
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
    pub update: Option<Ident>,
    pub handler: Handler,
}

impl Parse for WidgetAttrArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = WidgetAttrArgs {
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
            if args.update.is_none() && lookahead.peek(kw::update) {
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

macro_rules! property {
    ($name:ident : $ty:ty = $def:expr ; $kw:path : $input:ident => $parse:expr ;) => {
        #[derive(Debug)]
        pub struct $name {
            /// Some(span) if set, None if default
            pub span: Option<Span>,
            /// Value (default or set)
            pub value: $ty,
        }
        impl Default for $name {
            fn default() -> Self {
                $name {
                    span: None,
                    value: $def,
                }
            }
        }
        impl Parse for $name {
            fn parse($input: ParseStream) -> Result<Self> {
                let span = Some($input.parse::<$kw>()?.span());
                let _: Eq = $input.parse()?;
                let value = $parse;
                Ok($name { span, value })
            }
        }
    };
}
property!(
    KeyNav: bool = false;
    kw::key_nav : input => input.parse::<syn::LitBool>()?.value;
);
property!(
    HoverHighlight: bool = false;
    kw::hover_highlight : input => input.parse::<syn::LitBool>()?.value;
);
property!(
    CursorIcon: Expr = parse_quote! { ::kas::event::CursorIcon::Default };
    kw::cursor_icon : input => input.parse()?;
);
property!(
    FindId: Option<Expr> = None;
    kw::find_id : input => Some(input.parse()?);
);

#[derive(Debug, Default)]
pub struct WidgetArgs {
    pub key_nav: KeyNav,
    pub hover_highlight: HoverHighlight,
    pub cursor_icon: CursorIcon,
    pub derive: Option<Member>,
    pub layout: Option<make_layout::Tree>,
    pub find_id: FindId,
    pub msg: Option<Type>,
}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut key_nav = KeyNav::default();
        let mut hover_highlight = HoverHighlight::default();
        let mut cursor_icon = CursorIcon::default();
        let mut derive = None;
        let mut layout = None;
        let mut find_id = FindId::default();
        let mut msg = None;

        let content;
        let _ = braced!(content in input);

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::key_nav) && key_nav.span.is_none() {
                key_nav = content.parse()?;
            } else if lookahead.peek(kw::hover_highlight) && hover_highlight.span.is_none() {
                hover_highlight = content.parse()?;
            } else if lookahead.peek(kw::cursor_icon) && cursor_icon.span.is_none() {
                cursor_icon = content.parse()?;
            } else if lookahead.peek(kw::derive) && derive.is_none() {
                let _: kw::derive = content.parse()?;
                let _: Eq = content.parse()?;
                let _: Token![self] = content.parse()?;
                let _: Token![.] = content.parse()?;
                derive = Some(content.parse()?);
            } else if lookahead.peek(kw::layout) && layout.is_none() {
                let _: kw::layout = content.parse()?;
                let _: Eq = content.parse()?;
                layout = Some(content.parse()?);
            } else if content.peek(kw::find_id) {
                find_id = content.parse()?;
            } else if msg.is_none() && lookahead.peek(kw::msg) {
                let _: kw::msg = content.parse()?;
                let _: Eq = content.parse()?;
                msg = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            let _ = content.parse::<Token![;]>()?;
        }

        Ok(WidgetArgs {
            key_nav,
            hover_highlight,
            cursor_icon,
            derive,
            layout,
            find_id,
            msg,
        })
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
    pub widget_attr: Option<WidgetAttrArgs>,
    pub ident: Option<Ident>,
    pub ty: ChildType,
    pub value: Expr,
}

#[derive(Debug)]
pub struct MakeWidget {
    pub attr_widget: WidgetArgs,
    pub extra_attrs: Vec<Attribute>,

    pub token: Token![struct],
    pub generics: Generics,

    pub brace_token: Brace,
    pub fields: Punctuated<WidgetField, Comma>,

    pub extra_impls: Vec<ItemImpl>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attr_widget = None;
        let mut extra_attrs = Vec::new();

        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.drain(..) {
            if attr.path == parse_quote! { widget } {
                if attr_widget.is_none() {
                    let _span = attr.span();
                    let w: WidgetArgs = syn::parse2(attr.tokens)?;
                    attr_widget = Some(w);
                } else {
                    emit_error!(attr.span(), "multiple #[widget(..)] attributes on type");
                }
            } else {
                extra_attrs.push(attr);
            }
        }
        let token = input.parse::<Token![struct]>()?;

        let mut generics = input.parse::<Generics>()?;
        if input.peek(Token![where]) {
            generics.where_clause = Some(input.parse()?);
        }

        let content;
        let brace_token = braced!(content in input);
        let fields = content.parse_terminated(WidgetField::parse)?;

        let mut extra_impls = Vec::new();
        while !input.is_empty() {
            extra_impls.push(parse_impl(None, input)?);
        }

        let attr_widget = attr_widget.unwrap_or_default();

        Ok(MakeWidget {
            attr_widget,
            extra_attrs,

            token,
            generics,

            brace_token,
            fields,

            extra_impls,
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
            Some(args)
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
