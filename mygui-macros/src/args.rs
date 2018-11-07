use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, TokenStreamExt, ToTokens};
use syn::{Block, Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Ident, Index, Lit, Member, Path};
use syn::{parse_quote, bracketed, parenthesized};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::token::{Brace, Bracket, Colon, Comma, Eq, FatArrow, Paren, Pound, Semi};
use syn::spanned::Spanned;

pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

pub struct Args {
    pub core: Member,
    pub layout: Option<LayoutArgs>,
    pub widget: Option<WidgetArgs>,
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
        }
    }
    
    if let Some(core) = core {
        Ok(Args { core, layout, widget, children })
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
}

pub struct WidgetAttrArgs {
    pub col: Option<Lit>,
    pub row: Option<Lit>,
    pub cspan: Option<Lit>,
    pub rspan: Option<Lit>,
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
        };
        if input.is_empty() {
            return Ok(args);
        }
        
        let content;
        let _ = parenthesized!(content in input);
        
        loop {
            if content.is_empty() {
                break;
            }
            
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
            self.cspan.is_some() || self.rspan.is_some()
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
                    args.append(comma);
                }
                args.append_all(quote!{ rspan = #lit });
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
        
        let layout = if content.is_empty() {
            None
        } else {
            Some(content.parse()?)
        };
        
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
            if content.is_empty() {
                break;
            }
            
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

pub enum ChildType {
    Generic,
    Path(Path),
    Response(Path),  // generic but with defined handler response type
}

pub struct WidgetField {
    pub widget_attr: Option<WidgetAttr>,
    pub ident: Option<Ident>,
    pub ty: ChildType,
    pub value: Expr,
    pub handler: Option<Block>,
}

pub struct MakeWidget {
    // layout direction
    pub layout: Ident,
    // response type
    pub response: Path,
    // child widgets and data fields
    pub fields: Vec<WidgetField>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let layout: Ident = input.parse()?;
        let _: FatArrow = input.parse()?;
        
        let response: Path = input.parse()?;
        let _: Semi = input.parse()?;
        
        let mut fields = vec![];
        loop {
            if input.is_empty() {
                break;
            }
            
            fields.push(input.parse::<WidgetField>()?);
            
            if input.is_empty() {
                break;
            }
            let _: Comma = input.parse()?;
        }
        
        Ok(MakeWidget { layout, fields, response })
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
        
        let ident = if input.peek2(Eq) ||
            input.peek2(Colon) && (input.peek3(Ident) || input.peek3(Bracket))
        {
            let ident: Ident = input.parse()?;
            Some(ident)
        } else {
            None
        };
        
        let ty = if ident.is_some() && input.peek(Colon) {
            let _: Colon = input.parse()?;
            if input.peek(Bracket) {
                let inner;
                let bracket = bracketed!(inner in input);
                if !widget_attr.is_some() {
                    return Err(Error::new(bracket.span,
                        "can only use [ResponseType] restriction on widgets"))
                }
                ChildType::Response(inner.parse()?)
            } else {
                ChildType::Path(input.parse()?)
            }
        } else {
            ChildType::Generic
        };
        
        if ident.is_some() {
            let _: Eq = input.parse()?;
        }
        
        let value: Expr = input.parse()?;
        
        let handler = if input.peek(FatArrow) {
            let _: FatArrow = input.parse()?;
            Some(input.parse::<Block>()?)
        } else {
            None
        };
        
        Ok(WidgetField{ widget_attr, ident, ty, value, handler })
    }
}
