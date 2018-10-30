#![recursion_limit="128"]

extern crate proc_macro;

use std::env;
use self::proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Ident};
use syn::{parse_quote, parse_macro_input, bracketed, parenthesized};
use syn::parse::{Parse, ParseStream, Result};
use syn::token::{Comma, Eq};
use syn::punctuated::Punctuated;

#[proc_macro_attribute]
pub fn mygui_impl(args: TokenStream, mut input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ImplArgs);
    let input2 = input.clone();     // TODO: should we clone the input stream?
    let ast = parse_macro_input!(input2 as DeriveInput);
    
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    
    let c = if env::var("CARGO_PKG_NAME").unwrap() == "mygui" {
        quote!( crate )
    } else {
        quote!( mygui )
    };
    
    if let Some(core) = args.core {
        let core = core.core;
        let gen = TokenStream::from(quote! {
            impl #impl_generics #c::widget::Core
                for #name #ty_generics #where_clause
            {
                fn number(&self) -> u32 {
                    self.#core.number()
                }
                fn set_number(&mut self, number: u32) {
                    self.#core.set_number(number);
                }
                
                fn tkd(&self) -> #c::toolkit::TkData {
                    self.#core.tkd()
                }
                fn set_tkd(&mut self, tkd: #c::toolkit::TkData) {
                    self.#core.set_tkd(tkd)
                }
                
                fn rect(&self) -> &#c::widget::Rect {
                    self.#core.rect()
                }
                fn rect_mut(&mut self) -> &mut #c::widget::Rect {
                    self.#core.rect_mut()
                }
            }
        });
        input.extend(std::iter::once(gen));
    }
    
    if let Some(widget) = args.widget {
        let class = widget.class;
        let label = widget.label.unwrap_or_else(|| parse_quote!{ None });
        let children = widget.children;
        let count = children.iter().count();
        let child1 = children.iter();
        let child2 = children.iter();
        
        // TODO: iteration could generate nicer code if done without quote
        let gen = TokenStream::from(quote! {
            impl #impl_generics #c::widget::Widget
                    for #name #ty_generics #where_clause
            {
                fn class(&self) -> #c::widget::Class { #class }
                fn label(&self) -> Option<&str> { #label }

                fn len(&self) -> usize {
                    #count
                }
                fn get(&self, _index: usize) -> Option<&#c::widget::Widget> {
                    let _i = 0;
                    #(
                        if _index == _i {
                            return Some(&self.#child1);
                        }
                        let _i = _i + 1;
                    )*
                    return None;
                }
                fn get_mut(&mut self, _index: usize) -> Option<&mut #c::widget::Widget> {
                    let _i = 0;
                    #(
                        if _index == _i {
                            return Some(&mut self.#child2);
                        }
                        let _i = _i + 1;
                    )*
                    return None;
                }
            }
        });
        input.extend(std::iter::once(gen));
    }
    
    input
}

mod kw {
    use syn::custom_keyword;
    
    custom_keyword!(Core);
    custom_keyword!(Widget);
    custom_keyword!(class);
    custom_keyword!(label);
    custom_keyword!(children);
}

struct ImplArgs {
    core: Option<CoreArgs>,
    widget: Option<WidgetArgs>,
}

impl Parse for ImplArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ImplArgs { core: None, widget: None };
        
        loop {
            if input.is_empty() {
                break;
            }
            
            let lookahead = input.lookahead1();
            if args.core.is_none() && lookahead.peek(kw::Core) {
                args.core = Some(input.parse()?);
            } else if args.widget.is_none() && lookahead.peek(kw::Widget) {
                args.widget = Some(input.parse()?);
            } else {
                return Err(lookahead.error());
            }
            
            if input.is_empty() {
                break;
            }
            let _: Comma = input.parse()?;
        }
        
        Ok(args)
    }
}

struct CoreArgs {
    core: Ident
}

impl Parse for CoreArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let _: kw::Core = input.parse()?;
        let content;
        let _ = parenthesized!(content in input);
        let core: Ident = content.parse()?;
        Ok(CoreArgs { core })
    }
}

struct WidgetArgs {
    class: Expr,
    label: Option<Expr>,
    children: Punctuated<Ident, Comma>,
}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let _: kw::Widget = input.parse()?;
        let content;
        let _ = parenthesized!(content in input);
        
        let mut class = None;
        let mut label = None;
        let mut children = None;
        
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
            } else if children.is_none() && lookahead.peek(kw::children) {
                let _: kw::children = content.parse()?;
                let _: Eq = content.parse()?;
                let list;
                let _ = bracketed!(list in content);
                children = Some(list.parse_terminated(Ident::parse)?);
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
            children: children.ok_or_else(|| content.error("expected `children = ...`"))?,
        })
    }
}
