// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::TokenStream;
use proc_macro_error::{emit_error, emit_warning};
use quote::{quote, quote_spanned, TokenStreamExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Fields, Ident, ItemStruct, LitInt, Member, Token, WhereClause};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(on);
    custom_keyword!(skip);
}

#[derive(Debug, Default)]
pub struct AutoImpl {
    pub targets: Punctuated<Ident, Comma>,
    pub clause: Option<WhereClause>,
    pub on: Option<Member>,
    pub skip: Punctuated<Member, Comma>,
}

impl Parse for AutoImpl {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut targets = Punctuated::new();
        let mut clause = None;
        let mut on = None;
        let mut skip = Punctuated::new();

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![where]) || lookahead.peek(kw::on) || lookahead.peek(kw::skip) {
                break;
            }

            if targets.empty_or_trailing() {
                if lookahead.peek(Ident) {
                    targets.push_value(input.parse()?);
                    continue;
                }
            } else if input.peek(Comma) {
                targets.push_punct(input.parse::<Comma>()?);
                continue;
            }
            return Err(lookahead.error());
        }

        let mut lookahead = input.lookahead1();
        if lookahead.peek(Token![where]) {
            clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        if input.peek(kw::on) {
            let _: kw::on = input.parse()?;
            on = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        if input.peek(kw::skip) {
            let _: kw::skip = input.parse()?;
            skip.push_value(input.parse()?);
            while !input.is_empty() {
                let lookahead = input.lookahead1();
                if skip.empty_or_trailing() {
                    if lookahead.peek(Ident) || lookahead.peek(LitInt) {
                        skip.push_value(input.parse()?);
                        continue;
                    }
                } else if lookahead.peek(Comma) {
                    skip.push_punct(input.parse()?);
                    continue;
                }
                return Err(lookahead.error());
            }
        }

        if !input.is_empty() {
            return Err(lookahead.error());
        }

        Ok(AutoImpl {
            targets,
            clause,
            on,
            skip,
        })
    }
}

pub fn autoimpl(attr: AutoImpl, mut item: ItemStruct) -> TokenStream {
    let ident = &item.ident;
    if let Some(x) = attr.clause {
        if let Some(ref mut y) = item.generics.where_clause {
            if !y.predicates.empty_or_trailing() {
                y.predicates.push_punct(Default::default());
            }
            y.predicates.extend(x.predicates.into_pairs());
        } else {
            item.generics.where_clause = Some(x);
        }
    }
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    for mem in attr.skip.iter() {
        match item.fields {
            Fields::Named(ref fields) => {
                if fields
                    .named
                    .iter()
                    .any(|field| *mem == Member::from(field.ident.clone().unwrap()))
                {
                    continue;
                }
            }
            Fields::Unnamed(ref fields) => {
                let len = fields.unnamed.len();
                if let Member::Unnamed(index) = mem {
                    if (index.index as usize) < len {
                        continue;
                    }
                }
            }
            Fields::Unit => (),
        }
        emit_error!(mem.span(), "not a struct field");
    }

    let skip = |item: &Member| -> bool { attr.skip.iter().any(|mem| *mem == *item) };

    let on_unused = true;
    let mut toks = TokenStream::new();

    for target in &attr.targets {
        let span = target.span();
        if target == "Clone" {
            let mut inner = quote! {};
            for (i, field) in item.fields.iter().enumerate() {
                let mem = if let Some(ref id) = field.ident {
                    inner.append_all(quote! { #id: });
                    Member::from(id.clone())
                } else {
                    Member::from(i)
                };

                if skip(&mem) {
                    inner.append_all(quote! { Default::default(), });
                } else {
                    inner.append_all(quote! { self.#mem.clone(), });
                }
            }
            let inner = match &item.fields {
                Fields::Named(_) => quote! { Self { #inner } },
                Fields::Unnamed(_) => quote! { Self( #inner ) },
                Fields::Unit => quote! { Self },
            };
            toks.append_all(quote_spanned! {span=>
                impl #impl_generics std::clone::Clone for #ident #ty_generics #where_clause {
                    fn clone(&self) -> Self {
                        #inner
                    }
                }
            });
        } else if target == "Debug" {
            let name = ident.to_string();
            let mut inner;
            match item.fields {
                Fields::Named(ref fields) => {
                    inner = quote! { f.debug_struct(#name) };
                    for field in fields.named.iter() {
                        let ident = field.ident.as_ref().unwrap();
                        if !skip(&ident.clone().into()) {
                            let name = ident.to_string();
                            inner.append_all(quote! {
                                .field(#name, &self.#ident)
                            });
                        }
                    }
                    if attr.skip.is_empty() {
                        inner.append_all(quote! { .finish() });
                    } else {
                        inner.append_all(quote! { .finish_non_exhaustive() });
                    };
                }
                Fields::Unnamed(ref fields) => {
                    inner = quote! { f.debug_tuple(#name) };
                    for i in 0..fields.unnamed.len() {
                        if !skip(&i.into()) {
                            inner.append_all(quote! {
                                .field(&self.#i)
                            });
                        }
                    }
                    inner.append_all(quote! { .finish() });
                }
                Fields::Unit => {
                    inner = quote! { #name };
                }
            }
            toks.append_all(quote_spanned! {span=>
                impl #impl_generics std::fmt::Debug for #ident #ty_generics #where_clause {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        #inner
                    }
                }
            });
        } else {
            emit_error!(span, "autoimpl: unsupported trait");
        }
    }

    if let Some(mem) = attr.on {
        if on_unused {
            emit_warning!(mem.span(), "autoimpl: no impl used this parameter");
        }
    }

    toks
}
