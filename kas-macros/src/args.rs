// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, TokenStreamExt, ToTokens};
use syn::{Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident, Index, Lit, Member, TypeTraitObject, Type, TypePath, ImplItemMethod};
use syn::{parse_quote, braced, bracketed, parenthesized};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::token::{Brace, Colon, Comma, Eq, FatArrow, Impl, Paren, Pound, RArrow, Semi, Struct, Underscore, Where};
use syn::spanned::Spanned;

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

pub struct Args {
    pub core: Member,
    pub widget: WidgetArgs,
    pub handler: Option<HandlerArgs>,
    pub children: Vec<Child>,
}

pub fn read_attrs(ast: &mut DeriveInput) -> Result<Args> {
    let not_struct_err = |span| Err(Error::new(span,
            "cannot derive Widget on an enum, union or unit struct"));
    let (fields, span) = match &mut ast.data {
        Data::Struct(data) => {
            match &mut data.fields {
                Fields::Named(FieldsNamed {
                    brace_token: Brace { span },
                    named: fields,
                }) |
                Fields::Unnamed(FieldsUnnamed {
                    paren_token: Paren { span },
                    unnamed: fields,
                }) => {
                    (fields, span)
                },
                Fields::Unit => return not_struct_err(data.struct_token.span()),
            }
        },
        Data::Enum(data) => return not_struct_err(data.enum_token.span()),
        Data::Union(data) => return not_struct_err(data.union_token.span()),
    };
    
    let mut core = None;
    let mut children = vec![];
    
    for (i, field) in fields.iter_mut().enumerate() {
        for attr in field.attrs.drain(..) {
            if attr.path == parse_quote!{ core } {
                if core.is_none() {
                    core = Some(member(i, field.ident.clone()));
                } else {
                    attr.span()
                        .unstable()
                        .error("multiple fields marked with #[core]")
                        .emit();
                }
            } else if attr.path == parse_quote!{ widget } {
                let ident = member(i, field.ident.clone());
                let args = syn::parse2(attr.tts)?;
                children.push(Child{ ident, args });
            }
        }
    }
    
    let mut widget = None;
    let mut handler = None;
    
    for attr in ast.attrs.drain(..) {
        if attr.path == parse_quote!{ widget } {
            if widget.is_none() {
                widget = Some(syn::parse2(attr.tts)?);
            } else {
                attr.span()
                    .unstable()
                    .error("multiple #[widget(..)] attributes on type")
                    .emit()
            }
        } else if attr.path == parse_quote!{ handler } {
            if handler.is_none() {
                handler = Some(syn::parse2(attr.tts)?);
            } else {
                attr.span()
                    .unstable()
                    .error("multiple #[handler(..)] attributes on type")
                    .emit()
            }
        }
    }
    
    if let Some(core) = core {
        if let Some(widget) = widget {
            Ok(Args { core, widget, handler, children })
        } else {
            Err(Error::new(*span,
                "a type deriving Widget must be annotated with the #[widget]` attribute"))
        }
    } else {
        Err(Error::new(*span,
            "one field must be marked with #[core] when deriving Widget"))
    }
}

fn member(index: usize, ident: Option<Ident>) -> Member {
    match ident {
        None => Member::Unnamed(Index{
                index: index as u32,
                span: Span::call_site()
            }),
        Some(ident) => Member::Named(ident),
    }
}

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;
    
    custom_keyword!(class);
    custom_keyword!(layout);
    custom_keyword!(col);
    custom_keyword!(row);
    custom_keyword!(cspan);
    custom_keyword!(rspan);
    custom_keyword!(widget);
    custom_keyword!(handler);
    custom_keyword!(msg);
    custom_keyword!(generics);
}

#[derive(Debug)]
pub struct WidgetAttrArgs {
    pub col: Option<Lit>,
    pub row: Option<Lit>,
    pub cspan: Option<Lit>,
    pub rspan: Option<Lit>,
    pub handler: Option<Ident>,
}

pub struct GridPos(Lit, Lit, Lit, Lit);

#[cfg(not(feature = "cassowary"))]
impl WidgetAttrArgs {
    // If we have *any* position information, then yield a GridPos, filling in
    // missing information with defaults.
    pub fn as_pos(&self) -> Option<GridPos> {
        if self.col.is_some() || self.row.is_some() ||
            self.cspan.is_some() || self.rspan.is_some()
        {
            Some(GridPos(
                self.col.as_ref().cloned().unwrap_or_else(|| parse_quote!{ 0}),
                self.row.as_ref().cloned().unwrap_or_else(|| parse_quote!{ 0 }),
                self.cspan.as_ref().cloned().unwrap_or_else(|| parse_quote!{ 1 }),
                self.rspan.as_ref().cloned().unwrap_or_else(|| parse_quote!{ 1 }),
            ))
        } else {
            None
        }
    }
}

impl Parse for WidgetAttrArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = WidgetAttrArgs {
            col: None, row: None,
            cspan: None, rspan: None,
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
        if self.col.is_some() || self.row.is_some() ||
            self.cspan.is_some() || self.rspan.is_some() ||
            self.handler.is_some()
        {
            let comma = TokenTree::from(Punct::new(',', Spacing::Alone));
            let mut args = TokenStream::new();
            if let Some(ref lit) = self.col {
                args.append_all(quote!{ col = #lit });
            }
            if let Some(ref lit) = self.row {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote!{ row = #lit });
            }
            if let Some(ref lit) = self.cspan {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote!{ cspan = #lit });
            }
            if let Some(ref lit) = self.rspan {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote!{ rspan = #lit });
            }
            if let Some(ref ident) = self.handler {
                if !args.is_empty() {
                    args.append(comma);
                }
                args.append_all(quote!{ handler = #ident });
            }
            tokens.append_all(quote!{ ( #args ) });
        }
    }
}

pub struct WidgetAttr {
    pub args: WidgetAttrArgs
}

impl ToTokens for WidgetAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let args = &self.args;
        tokens.append_all(quote!{ #[widget #args] });
    }
}

impl ToTokens for GridPos {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (c, r, cs, rs) = (&self.0, &self.1, &self.2, &self.3);
        tokens.append_all(quote!{ (#c, #r, #cs, #rs) });
    }
}

pub struct WidgetArgs {
    pub class: Expr,
    pub layout: Option<Ident>,
}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::new(Span::call_site(),
                "expected #[widget(class = ...)]; found #[widget]"));
        }
        
        let content;
        let _ = parenthesized!(content in input);
        
        let mut class = None;
        let mut layout = None;
        
        loop {
            let lookahead = content.lookahead1();
            if class.is_none() && lookahead.peek(kw::class) {
                let _: kw::class = content.parse()?;
                let _: Eq = content.parse()?;
                let expr: Expr = content.parse()?;
                class = Some(expr);
            } else if layout.is_none() && lookahead.peek(kw::layout) {
                let _: kw::layout = content.parse()?;
                let _: Eq = content.parse()?;
                let ident: Ident = content.parse()?;
                layout = Some(ident);
            } else {
                return Err(lookahead.error());
            }
            
            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }
        
        Ok(WidgetArgs {
            class: class.ok_or_else(|| content.error("expected `class = ...`"))?,
            layout,
        })
    }
}

pub struct HandlerArgs {
    pub msg: Type,
    pub generics: Generics,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut msg = parse_quote!{ () };
        let mut generics = Generics::default();
        
        if input.is_empty() {
            return Ok(HandlerArgs{ msg, generics });
        }
        
        let content;
        let _ = parenthesized!(content in input);
        
        // If we have a where clause, that will greedily consume remaining
        // input. Because of this, `msg = ...` must come first.
        
        if content.peek(kw::msg) {
            let _: kw::msg = content.parse()?;
            let _: Eq = content.parse()?;
            msg = content.parse()?;
            
            if content.peek(Comma) {
                let _: Comma = content.parse()?;
            }
        }
        
        if content.peek(kw::generics) {
            let _: kw::generics = content.parse()?;
            let _: Eq = content.parse()?;
            generics = content.parse()?;
            if content.peek(Where) {
                generics.where_clause = content.parse()?;
            }
        }
        
        Ok(HandlerArgs { msg, generics })
    }
}

pub enum Class {
    Container(Ident),
    Frame,
}

pub enum ChildType {
    Fixed(Type),     // fixed type
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

pub struct MakeWidget {
    // widget class
    pub class: Class,
    // msg type
    pub msg: Type,
    // child widgets and data fields
    pub fields: Vec<WidgetField>,
    // impl blocks on the widget
    pub impls: Vec<(Option<TypePath>, Vec<ImplItemMethod>)>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let class_name: Ident = input.parse()?;
        let class = if class_name == "container" {
            let content;
            let _ = parenthesized!(content in input);
            let layout: Ident = content.parse()?;
            Class::Container(layout)
        } else if class_name == "frame" {
            Class::Frame
        } else {
            return Err(Error::new(class_name.span(),
                    "make_widget only supports the following classes: container, frame"));
        };
        
        let _: FatArrow = input.parse()?;
        let msg: Type = input.parse()?;
        let _: Semi = input.parse()?;
        
        let _: Struct = input.parse()?;
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
        
        Ok(MakeWidget { class, msg, fields, impls })
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
                return Err(Error::new(arrow.span(),
                    "can only use `-> Msg` type restriction on widgets"))
            }
            let msg: Type = input.parse()?;
            match &mut ty {
                ChildType::Fixed(_) => {
                    return Err(Error::new(arrow.span(),
                        "cannot use `-> Msg` type restriction with fixed type"));
                }
                ChildType::Generic(ref mut gen_r, _) => {
                    *gen_r = Some(msg);
                }
            }
        }
        
        let _: Eq = input.parse()?;
        let value: Expr = input.parse()?;
        
        Ok(WidgetField{ widget_attr, ident, ty, value })
    }
}
