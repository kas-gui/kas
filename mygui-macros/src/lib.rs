#![recursion_limit="128"]

extern crate proc_macro;

use std::env;
use self::proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, parenthesized};
use syn::parse::{Parse, ParseStream, Result};
use syn::token::Comma;

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
    input
}

mod kw {
    use syn::custom_keyword;
    
    custom_keyword!(Core);
}

struct ImplArgs {
    core: Option<CoreArgs>,
}

impl Parse for ImplArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ImplArgs { core: None };
        
        loop {
            if input.is_empty() {
                break;
            }
            
            let lookahead = input.lookahead1();
            if args.core.is_none() && lookahead.peek(kw::Core) {
                args.core = Some(input.parse()?);
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
