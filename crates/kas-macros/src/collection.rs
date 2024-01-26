// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Collection macro

use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Expr, Ident, Lifetime, LitStr, Token};

#[derive(Debug)]
pub enum StorIdent {
    Named(Ident, Span),
    Generated(Ident, Span),
}
impl From<Lifetime> for StorIdent {
    fn from(lt: Lifetime) -> StorIdent {
        let span = lt.span();
        StorIdent::Named(lt.ident, span)
    }
}
impl From<Ident> for StorIdent {
    fn from(ident: Ident) -> StorIdent {
        let span = ident.span();
        StorIdent::Generated(ident, span)
    }
}
impl ToTokens for StorIdent {
    fn to_tokens(&self, toks: &mut Toks) {
        match self {
            StorIdent::Named(ident, _) | StorIdent::Generated(ident, _) => ident.to_tokens(toks),
        }
    }
}

#[derive(Default)]
pub struct NameGenerator(usize);
impl NameGenerator {
    pub fn next(&mut self) -> Ident {
        let name = format!("_stor{}", self.0);
        self.0 += 1;
        let span = Span::call_site();
        Ident::new(&name, span)
    }

    pub fn parse_or_next(&mut self, input: ParseStream) -> Result<StorIdent> {
        if input.peek(Lifetime) {
            Ok(input.parse::<Lifetime>()?.into())
        } else {
            Ok(self.next().into())
        }
    }
}

pub enum Item {
    Label(Ident, LitStr),
    Widget(Ident, Expr),
}

impl Item {
    fn parse(input: ParseStream, gen: &mut NameGenerator) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(Item::Label(gen.next(), input.parse()?))
        } else {
            Ok(Item::Widget(gen.next(), input.parse()?))
        }
    }
}

pub struct Collection(Vec<Item>);

impl Parse for Collection {
    fn parse(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();

        let mut items = vec![];
        while !inner.is_empty() {
            items.push(Item::parse(inner, &mut gen)?);

            if inner.is_empty() {
                break;
            }

            let _: Token![,] = inner.parse()?;
        }

        Ok(Collection(items))
    }
}

impl Collection {
    pub fn expand(&self) -> Toks {
        let name = Ident::new("_Collection", Span::call_site());

        let mut data_ty = None;
        for (index, item) in self.0.iter().enumerate() {
            if let Item::Widget(_, expr) = item {
                let ty = Ident::new(&format!("_W{}", index), expr.span());
                data_ty = Some(quote! {<#ty as ::kas::Widget>::Data});
                break;
            }
        }

        let mut item_names = Vec::with_capacity(self.0.len());
        let mut ty_generics = Punctuated::<Ident, Comma>::new();
        let mut stor_ty = quote! {};
        let mut stor_def = quote! {};
        for (index, item) in self.0.iter().enumerate() {
            match item {
                Item::Label(stor, text) => {
                    item_names.push(stor.to_token_stream());
                    let span = text.span();
                    if let Some(ref data_ty) = data_ty {
                        stor_ty.append_all(
                            quote! { #stor: ::kas::hidden::MapAny<#data_ty, ::kas::hidden::StrLabel>, },
                        );
                        stor_def.append_all(
                            quote_spanned! {span=> #stor: ::kas::hidden::MapAny::new(::kas::hidden::StrLabel::new(#text)), },
                        );
                    } else {
                        stor_ty.append_all(quote! { #stor: ::kas::hidden::StrLabel, });
                        stor_def.append_all(
                            quote_spanned! {span=> #stor: ::kas::hidden::StrLabel::new(#text), },
                        );
                    }
                }
                Item::Widget(stor, expr) => {
                    let span = expr.span();
                    item_names.push(stor.to_token_stream());
                    let ty = Ident::new(&format!("_W{}", index), span);
                    stor_ty.append_all(quote! { #stor: #ty, });
                    stor_def.append_all(quote_spanned! {span=> #stor: Box::new(#expr), });
                    ty_generics.push(ty);
                }
            }
        }

        let data_ty = data_ty
            .map(|ty| quote! { #ty })
            .unwrap_or_else(|| quote! { () });

        let (impl_generics, ty_generics) = if ty_generics.is_empty() {
            (quote! {}, quote! {})
        } else {
            let mut toks = quote! {};
            let mut iter = ty_generics.iter();
            if let Some(ty) = iter.next() {
                toks = quote! { #ty: ::kas::Widget, }
            }
            for ty in iter {
                toks.append_all(quote!(
                    #ty: ::kas::Widget<Data = #data_ty>,
                ));
            }
            (quote! { <#toks> }, quote! { <#ty_generics> })
        };

        let len = item_names.len();
        let is_empty = match len {
            0 => quote! { true },
            _ => quote! { false },
        };

        let mut get_layout_rules = quote! {};
        let mut get_mut_layout_rules = quote! {};
        let mut for_node_rules = quote! {};
        for (index, path) in item_names.iter().enumerate() {
            get_layout_rules.append_all(quote! {
                #index => Some(&self.#path),
            });
            get_mut_layout_rules.append_all(quote! {
                #index => Some(&mut self.#path),
            });
            for_node_rules.append_all(quote! {
                #index => closure(self.#path.as_node(data)),
            });
        }

        let toks = quote! {{
            struct #name #impl_generics {
                #stor_ty
            }

            impl #impl_generics ::kas::Collection for #name #ty_generics {
                type Data = #data_ty;

                fn is_empty(&self) -> bool { #is_empty }
                fn len(&self) -> usize { #len }

                fn get_layout(&self, index: usize) -> Option<&dyn Layout> {
                    match index {
                        #get_layout_rules
                        _ => None,
                    }
                }
                fn get_mut_layout(&mut self, index: usize) -> Option<&mut dyn Layout> {
                    match index {
                        #get_mut_layout_rules
                        _ => None,
                    }
                }
                fn for_node(
                    &mut self,
                    data: &Self::Data,
                    index: usize,
                    closure: Box<dyn FnOnce(Node<'_>) + '_>,
                ) {
                    match index {
                        #for_node_rules
                        _ => (),
                    }
                }
            }

            #name {
                #stor_def
            }
        }};
        // println!("{}", toks);
        toks
    }
}
