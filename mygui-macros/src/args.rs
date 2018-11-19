use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, TokenStreamExt, ToTokens};
use syn::{Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident, Index, Lit, Member, Type, ImplItemMethod};
use syn::{parse_quote, bracketed, parenthesized};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::token::{Brace, Colon, Comma, Eq, FatArrow, Paren, Pound, RArrow, Semi, Underscore, Where};
use syn::spanned::Spanned;

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

pub struct Args {
    pub core: Member,
    pub layout: Option<LayoutArgs>,
    pub widget: Option<WidgetArgs>,
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
    
    let mut layout = None;
    let mut widget = None;
    let mut handler = None;
    
    for attr in ast.attrs.drain(..) {
        if attr.path == parse_quote!{ layout } {
            if layout.is_none() {
                let span = attr.span();
                let l: LayoutArgs = syn::parse2(attr.tts)?;
                if children.len() > 1 && l.layout.is_none() {
                    span
                        .unstable()
                        .error("layout description required with more than one child widget")
                        .emit()
                }
                layout = Some(l);
            } else {
                attr.span()
                    .unstable()
                    .error("multiple #[layout(..)] attributes on type")
                    .emit()
            }
        } else if attr.path == parse_quote!{ widget } {
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
        Ok(Args { core, layout, widget, handler, children })
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

mod kw {
    use syn::custom_keyword;
    
    custom_keyword!(class);
    custom_keyword!(label);
    custom_keyword!(col);
    custom_keyword!(row);
    custom_keyword!(cspan);
    custom_keyword!(rspan);
    custom_keyword!(widget);
    custom_keyword!(handler);
    custom_keyword!(response);
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

pub struct LayoutArgs {
    pub layout: Option<Ident>,
}

impl Parse for LayoutArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(LayoutArgs { layout: None });
        }
        
        let content;
        let _ = parenthesized!(content in input);
        
        let layout = Some(content.parse()?);
        Ok(LayoutArgs { layout })
    }
}

pub struct WidgetArgs {
    pub class: Expr,
    pub label: Option<Expr>,
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
        let mut label = None;
        
        loop {
            let lookahead = content.lookahead1();
            if class.is_none() && lookahead.peek(kw::class) {
                let _: kw::class = content.parse()?;
                let _: Eq = content.parse()?;
                let expr: Expr = content.parse()?;
                class = Some(expr);
            } else if label.is_none() && lookahead.peek(kw::label) {
                let _: kw::label = content.parse()?;
                let _: Eq = content.parse()?;
                let expr: Expr = content.parse()?;
                label = Some(expr);
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
            label,
        })
    }
}

pub struct HandlerArgs {
    pub response: Type,
    pub generics: Generics,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::new(Span::call_site(),
                "expected #[handler(response = ...)]; found #[handler]"));
        }
        
        let content;
        let _ = parenthesized!(content in input);
        
        // If we have a where clause, that will greedily consume remaining
        // input. Because of this, `response = ...` must come first.
        
        let _: kw::response = content.parse()?;
        let _: Eq = content.parse()?;
        let response: Type = content.parse()?;
        
        let generics = if content.peek(Comma) {
            let _: Comma = content.parse()?;
            let _: kw::generics = content.parse()?;
            let _: Eq = content.parse()?;
            let mut generics: Generics = content.parse()?;
            if content.peek(Where) {
                generics.where_clause = content.parse()?;
            }
            generics
        } else {
            Generics::default()
        };
        
        Ok(HandlerArgs { response, generics })
    }
}

pub enum ChildType {
    Generic,
    Type(Type),
    Response(Type),  // generic but with defined handler response type
}

pub struct WidgetField {
    pub widget_attr: Option<WidgetAttr>,
    pub ident: Option<Ident>,
    pub ty: ChildType,
    pub value: Expr,
}

pub struct MakeWidget {
    // layout direction
    pub layout: Ident,
    // response type
    pub response: Type,
    // child widgets and data fields
    pub fields: Vec<WidgetField>,
    // methods defined on the widget
    pub methods: Vec<ImplItemMethod>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let layout: Ident = input.parse()?;
        let _: FatArrow = input.parse()?;
        
        let response: Type = input.parse()?;
        let _: Semi = input.parse()?;
        
        let mut fields = vec![];
        loop {
            if input.peek(Semi) {
                break;
            }
            
            fields.push(input.parse::<WidgetField>()?);
            
            let lookahead = input.lookahead1();
            if lookahead.peek(Semi) {
                break;
            } else if lookahead.peek(Comma) {
                let _: Comma = input.parse()?;
                continue;
            } else {
                return Err(lookahead.error());
            }
        }
        
        let _: Semi = input.parse()?;
        
        let mut methods = vec![];
        loop {
            if input.is_empty() {
                break;
            }
            
            methods.push(input.parse::<ImplItemMethod>()?);
            
            if input.is_empty() {
                break;
            }
            let _: Comma = input.parse()?;
        }
        
        Ok(MakeWidget { layout, response, fields, methods })
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
        let ty = if input.peek(Colon) && !input.peek2(Colon) {
            let _: Colon = input.parse()?;
            ChildType::Type(input.parse()?)
        } else if input.peek(RArrow) {
            let arrow: RArrow = input.parse()?;
            if !widget_attr.is_some() {
                return Err(Error::new(arrow.span(),
                    "can only use `ident -> Response` type restriction on widgets"))
            }
            ChildType::Response(input.parse()?)
        } else {
            ChildType::Generic
        };
        
        let _: Eq = input.parse()?;
        let value: Expr = input.parse()?;
        
        Ok(WidgetField{ widget_attr, ident, ty, value })
    }
}
