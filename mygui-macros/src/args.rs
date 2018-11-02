use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt, ToTokens};
use syn::{Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Ident, Index, Lit, Member};
use syn::{parse_quote, bracketed, parenthesized};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::token::{Brace, Comma, Eq, Paren};
use syn::spanned::Spanned;

pub struct Child {
    pub ident: Member,
    pub args: ChildArgs,
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
    custom_keyword!(pos);
}

pub struct GridPos(Lit, Lit, Lit, Lit);
pub struct ChildArgs {
    pub pos: Option<GridPos>,
}

impl Parse for ChildArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ChildArgs { pos: None };
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
            if args.pos.is_none() && lookahead.peek(kw::pos) {
                let _: kw::pos = content.parse()?;
                let _: Eq = content.parse()?;
                let items;
                let _ = bracketed!(items in content);
                
                let col: Lit = items.parse()?;
                let _: Comma = items.parse()?;
                let row: Lit = items.parse()?;
                let spans = if !items.is_empty() {
                    let _: Comma = items.parse()?;
                    let col: Lit = items.parse()?;
                    let _: Comma = items.parse()?;
                    let row: Lit = items.parse()?;
                    (col, row)
                } else {
                    let one: Lit = parse_quote!{ 1 };
                    (one.clone(), one)
                };
                args.pos = Some(GridPos(col, row, spans.0, spans.1));
            }
            
            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }
        
        Ok(args)
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
