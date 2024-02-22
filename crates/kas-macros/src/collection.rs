// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Collection macro

use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parenthesized;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Expr, Ident, Lifetime, LitInt, LitStr, Token};

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

#[derive(Copy, Clone, Debug)]
pub struct CellInfo {
    pub col: u32,
    pub col_end: u32,
    pub row: u32,
    pub row_end: u32,
}

impl CellInfo {
    pub fn new(col: u32, row: u32) -> Self {
        CellInfo {
            col,
            col_end: col + 1,
            row,
            row_end: row + 1,
        }
    }
}

impl Parse for CellInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        fn parse_end(input: ParseStream, start: u32) -> Result<u32> {
            if input.parse::<Token![..=]>().is_ok() {
                let lit = input.parse::<LitInt>()?;
                let n: u32 = lit.base10_parse()?;
                if n >= start {
                    Ok(n + 1)
                } else {
                    Err(Error::new(lit.span(), format!("expected value >= {start}")))
                }
            } else if input.parse::<Token![..]>().is_ok() {
                let plus = input.parse::<Token![+]>();
                let lit = input.parse::<LitInt>()?;
                let n: u32 = lit.base10_parse()?;

                if plus.is_ok() {
                    Ok(start + n)
                } else if n > start {
                    Ok(n)
                } else {
                    Err(Error::new(lit.span(), format!("expected value > {start}")))
                }
            } else {
                Ok(start + 1)
            }
        }

        let inner;
        let _ = parenthesized!(inner in input);

        let col = inner.parse::<LitInt>()?.base10_parse()?;
        let col_end = parse_end(&inner, col)?;

        let _ = inner.parse::<Token![,]>()?;

        let row = inner.parse::<LitInt>()?.base10_parse()?;
        let row_end = parse_end(&inner, row)?;

        Ok(CellInfo {
            row,
            row_end,
            col,
            col_end,
        })
    }
}

impl ToTokens for CellInfo {
    fn to_tokens(&self, toks: &mut Toks) {
        let (col, col_end) = (self.col, self.col_end);
        let (row, row_end) = (self.row, self.row_end);
        toks.append_all(quote! {
            ::kas::layout::GridCellInfo {
                col: #col,
                col_end: #col_end,
                row: #row,
                row_end: #row_end,
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct GridDimensions {
    pub cols: u32,
    col_spans: u32,
    pub rows: u32,
    row_spans: u32,
}

impl GridDimensions {
    pub fn update(&mut self, cell: &CellInfo) {
        self.cols = self.cols.max(cell.col_end);
        if cell.col_end - cell.col > 1 {
            self.col_spans += 1;
        }
        self.rows = self.rows.max(cell.row_end);
        if cell.row_end - cell.row > 1 {
            self.row_spans += 1;
        }
    }
}

impl ToTokens for GridDimensions {
    fn to_tokens(&self, toks: &mut Toks) {
        let (cols, rows) = (self.cols, self.rows);
        let (col_spans, row_spans) = (self.col_spans, self.row_spans);
        toks.append_all(quote! { ::kas::layout::GridDimensions {
            cols: #cols,
            col_spans: #col_spans,
            rows: #rows,
            row_spans: #row_spans,
        } });
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

        let len = self.0.len();
        let is_empty = match len {
            0 => quote! { true },
            _ => quote! { false },
        };

        let mut ty_generics = Punctuated::<Ident, Comma>::new();
        let mut stor_ty = quote! {};
        let mut stor_def = quote! {};

        let mut get_layout_rules = quote! {};
        let mut get_mut_layout_rules = quote! {};
        let mut for_node_rules = quote! {};

        for (index, item) in self.0.iter().enumerate() {
            let path = match item {
                Item::Label(stor, text) => {
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
                    stor.to_token_stream()
                }
                Item::Widget(stor, expr) => {
                    let span = expr.span();
                    let ty = Ident::new(&format!("_W{}", index), span);
                    stor_ty.append_all(quote! { #stor: #ty, });
                    stor_def.append_all(quote_spanned! {span=> #stor: Box::new(#expr), });
                    ty_generics.push(ty);

                    stor.to_token_stream()
                }
            };

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
