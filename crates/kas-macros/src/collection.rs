// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Collection macro

use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Lifetime, LitStr, Token};

#[derive(Debug)]
pub enum StorIdent {
    Named(Ident, Span),
    Generated(String, Span),
}
impl From<Lifetime> for StorIdent {
    fn from(lt: Lifetime) -> StorIdent {
        let span = lt.span();
        StorIdent::Named(lt.ident, span)
    }
}
impl ToTokens for StorIdent {
    fn to_tokens(&self, toks: &mut Toks) {
        match self {
            StorIdent::Named(ident, _) => ident.to_tokens(toks),
            StorIdent::Generated(string, span) => Ident::new(string, *span).to_tokens(toks),
        }
    }
}

#[derive(Default)]
pub struct NameGenerator(usize);
impl NameGenerator {
    pub fn next(&mut self) -> StorIdent {
        let name = format!("_stor{}", self.0);
        self.0 += 1;
        StorIdent::Generated(name, Span::call_site())
    }

    pub fn parse_or_next(&mut self, input: ParseStream) -> Result<StorIdent> {
        if input.peek(Lifetime) {
            Ok(input.parse::<Lifetime>()?.into())
        } else {
            Ok(self.next())
        }
    }
}

pub enum Item {
    Label(StorIdent, LitStr),
    Widget(StorIdent, Expr),
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

#[derive(Default)]
pub struct StorageFields {
    pub ty_toks: Toks,
    pub def_toks: Toks,
}

impl Collection {
    fn storage_fields(
        &self,
        children: &mut Vec<Toks>,
        no_data: bool,
        data_ty: &Toks,
    ) -> StorageFields {
        let (mut ty_toks, mut def_toks) = (Toks::new(), Toks::new());
        for item in &self.0 {
            match item {
                Item::Label(stor, text) => {
                    children.push(stor.to_token_stream());
                    let span = text.span();
                    if no_data {
                        ty_toks.append_all(quote! { #stor: ::kas::hidden::StrLabel, });
                        def_toks.append_all(
                            quote_spanned! {span=> #stor: ::kas::hidden::StrLabel::new(#text), },
                        );
                    } else {
                        ty_toks.append_all(
                            quote! { #stor: ::kas::hidden::MapAny<#data_ty, ::kas::hidden::StrLabel>, },
                        );
                        def_toks.append_all(
                            quote_spanned! {span=> #stor: ::kas::hidden::MapAny::new(::kas::hidden::StrLabel::new(#text)), },
                        );
                    }
                }
                Item::Widget(stor, expr) => {
                    children.push(stor.to_token_stream());
                    ty_toks.append_all(quote! { #stor: Box<dyn ::kas::Widget<Data = #data_ty>>, });
                    let span = expr.span();
                    def_toks.append_all(quote_spanned! {span=> #stor: Box::new(#expr), });
                }
            }
        }

        StorageFields { ty_toks, def_toks }
    }

    pub fn expand(&self) -> Toks {
        let any_widgets = self.0.iter().any(|item| matches!(item, Item::Widget(_, _)));

        let name = Ident::new("_Collection", Span::call_site());
        let (data_ty, impl_generics, impl_target) = if any_widgets {
            (
                quote! { _Data },
                quote! { <_Data> },
                quote! { #name <_Data> },
            )
        } else {
            (quote! { () }, quote! {}, quote! { #name })
        };

        let mut children = Vec::new();
        let stor_defs = self.storage_fields(&mut children, !any_widgets, &data_ty);
        let stor_ty = &stor_defs.ty_toks;
        let stor_def = &stor_defs.def_toks;

        let len = children.len();
        let is_empty = match len {
            0 => quote! { true },
            _ => quote! { false },
        };

        let mut get_layout_rules = quote! {};
        let mut get_mut_layout_rules = quote! {};
        let mut for_node_rules = quote! {};
        for (index, path) in children.iter().enumerate() {
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

            impl #impl_generics ::kas::Collection for #impl_target {
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
