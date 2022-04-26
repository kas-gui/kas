// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use impl_tools_lib::parse_attr_group;
use proc_macro2::Span;
use proc_macro_error::{abort, emit_error};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq};
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

    custom_keyword!(find_id);
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
}

impl Parse for WidgetArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let mut key_nav = KeyNav::default();
        let mut hover_highlight = HoverHighlight::default();
        let mut cursor_icon = CursorIcon::default();
        let mut derive = None;
        let mut layout = None;
        let mut find_id = FindId::default();

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
        })
    }
}

#[derive(Debug)]
pub enum ChildType {
    Fixed(Type), // fixed type
    // A given type using generics internally
    InternGeneric(Punctuated<GenericParam, Comma>, Type),
    // Generic, optionally with an additional trait bound.
    Generic(Option<TypeTraitObject>),
}

#[derive(Debug)]
pub struct WidgetField {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Option<Ident>,
    pub colon_token: Option<Colon>,
    pub ty: ChildType,
    pub value: Expr,
}

#[derive(Debug)]
pub struct MakeWidget {
    pub attr_widget: WidgetArgs,
    pub attrs: Vec<Attribute>,

    pub token: Token![struct],
    pub generics: Generics,

    pub brace_token: Brace,
    pub fields: Punctuated<WidgetField, Comma>,

    pub impls: Vec<ItemImpl>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let mut index = None;
        for (i, attr) in attrs.iter().enumerate() {
            if attr.path == parse_quote! { widget } {
                if index.is_none() {
                    index = Some(i);
                } else {
                    emit_error!(attr.span(), "multiple #[widget(..)] attributes on type");
                }
            }
        }

        let attr_widget;
        if let Some(i) = index {
            let attr = attrs.remove(i);
            let (_, tokens) = parse_attr_group(attr.tokens)?;
            attr_widget = syn::parse2(tokens)?;
        } else {
            attr_widget = Default::default();
        }

        let token = input.parse::<Token![struct]>()?;

        let mut generics = input.parse::<Generics>()?;
        if input.peek(Token![where]) {
            generics.where_clause = Some(input.parse()?);
        }

        let content;
        let brace_token = braced!(content in input);
        let fields = content.parse_terminated(WidgetField::parse)?;

        let mut impls = Vec::new();
        while !input.is_empty() {
            impls.push(parse_impl(None, input)?);
        }

        Ok(MakeWidget {
            attr_widget,
            attrs,

            token,
            generics,

            brace_token,
            fields,

            impls,
        })
    }
}

impl Parse for WidgetField {
    fn parse(input: ParseStream) -> Result<Self> {
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
                ChildType::InternGeneric(params, ty)
            } else if input.peek(Token![impl]) {
                // generic with trait bound
                let _: Token![impl] = input.parse()?;
                let bound: TypeTraitObject = input.parse()?;
                ChildType::Generic(Some(bound))
            } else {
                ChildType::Fixed(input.parse()?)
            }
        } else {
            ChildType::Generic(None)
        };

        let _: Eq = input.parse()?;
        let value: Expr = input.parse()?;

        Ok(WidgetField {
            attrs,
            vis,
            ident,
            colon_token,
            ty,
            value,
        })
    }
}
