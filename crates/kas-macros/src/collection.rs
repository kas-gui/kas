// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Collection macro

use proc_macro2::{Span, TokenStream as Toks};
use quote::{ToTokens, TokenStreamExt, quote, quote_spanned};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Expr, Ident, LitInt, LitStr, Token};
use syn::{braced, bracketed, parenthesized};

#[allow(non_camel_case_types)]
mod kw {
    syn::custom_keyword!(align);
    syn::custom_keyword!(pack);
    syn::custom_keyword!(aligned_column);
    syn::custom_keyword!(aligned_row);
    syn::custom_keyword!(column);
    syn::custom_keyword!(row);
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
}

#[derive(Copy, Clone, Debug)]
pub struct CellInfo {
    pub col: u32,
    pub last_col: u32,
    pub row: u32,
    pub last_row: u32,
}

impl CellInfo {
    pub fn new(col: u32, row: u32) -> Self {
        CellInfo {
            col,
            last_col: col,
            row,
            last_row: row,
        }
    }
}

impl Parse for CellInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        fn parse_last(input: ParseStream, start: u32) -> Result<u32> {
            if input.parse::<Token![..=]>().is_ok() {
                let lit = input.parse::<LitInt>()?;
                let n: u32 = lit.base10_parse()?;
                if n >= start {
                    Ok(n)
                } else {
                    Err(Error::new(lit.span(), format!("expected value >= {start}")))
                }
            } else if input.parse::<Token![..]>().is_ok() {
                let plus = input.parse::<Token![+]>();
                let lit = input.parse::<LitInt>()?;
                let n: u32 = lit.base10_parse()?;

                if plus.is_ok() {
                    Ok(start + n - 1)
                } else if n > start {
                    Ok(n - 1)
                } else {
                    Err(Error::new(lit.span(), format!("expected value > {start}")))
                }
            } else {
                Ok(start)
            }
        }

        let inner;
        let _ = parenthesized!(inner in input);

        let col = inner.parse::<LitInt>()?.base10_parse()?;
        let last_col = parse_last(&inner, col)?;

        let _ = inner.parse::<Token![,]>()?;

        let row = inner.parse::<LitInt>()?.base10_parse()?;
        let last_row = parse_last(&inner, row)?;

        Ok(CellInfo {
            row,
            last_row,
            col,
            last_col,
        })
    }
}

impl ToTokens for CellInfo {
    fn to_tokens(&self, toks: &mut Toks) {
        let (col, last_col) = (self.col, self.last_col);
        let (row, last_row) = (self.row, self.last_row);
        toks.append_all(quote! {
            ::kas::layout::GridCellInfo {
                col: #col,
                last_col: #last_col,
                row: #row,
                last_row: #last_row,
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
        self.cols = self.cols.max(cell.last_col + 1);
        if cell.last_col > cell.col {
            self.col_spans += 1;
        }
        self.rows = self.rows.max(cell.last_row + 1);
        if cell.last_row > cell.row {
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
    Label(Ident, Toks, Toks),
    Widget(Ident, Expr),
}

impl Item {
    fn parse(input: ParseStream, names: &mut NameGenerator) -> Result<Self> {
        if input.peek(LitStr) {
            let text: LitStr = input.parse()?;
            let span = text.span();
            let mut ty = quote! { ::kas::widgets::Label<&'static str> };
            let mut def = quote_spanned! {span=> ::kas::widgets::Label::new(#text) };

            if input.peek(Token![.]) && input.peek2(kw::align) {
                let _: Token![.] = input.parse()?;
                let _: kw::align = input.parse()?;

                let inner;
                let _ = parenthesized!(inner in input);
                let hints: Expr = inner.parse()?;

                ty = quote! { ::kas::widgets::adapt::Align<#ty> };
                def = quote! { ::kas::widgets::adapt::Align::new(#def, #hints) };
            } else if input.peek(Token![.]) && input.peek2(kw::pack) {
                let _: Token![.] = input.parse()?;
                let _: kw::pack = input.parse()?;

                let inner;
                let _ = parenthesized!(inner in input);
                let hints: Expr = inner.parse()?;

                ty = quote! { ::kas::widgets::adapt::Pack<#ty> };
                def = quote! { ::kas::widgets::adapt::Pack::new(#def, #hints) };
            }

            Ok(Item::Label(names.next(), ty, def))
        } else {
            Ok(Item::Widget(names.next(), input.parse()?))
        }
    }
}

pub struct Collection(Vec<Item>);
pub struct CellCollection(Vec<CellInfo>, Collection);

impl Parse for Collection {
    fn parse(inner: ParseStream) -> Result<Self> {
        let mut names = NameGenerator::default();

        let mut items = vec![];
        while !inner.is_empty() {
            items.push(Item::parse(inner, &mut names)?);

            if inner.is_empty() {
                break;
            }

            let _: Token![,] = inner.parse()?;
        }

        Ok(Collection(items))
    }
}

impl Parse for CellCollection {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(kw::aligned_column) {
            let _: kw::aligned_column = input.parse()?;
            return Self::parse_aligned::<kw::row>(input, false);
        } else if input.peek(kw::aligned_row) {
            let _: kw::aligned_row = input.parse()?;
            return Self::parse_aligned::<kw::column>(input, true);
        }

        let mut names = NameGenerator::default();

        let mut cells = vec![];
        let mut items = vec![];
        while !input.is_empty() {
            cells.push(input.parse()?);
            let _: Token![=>] = input.parse()?;

            let item;
            let require_comma;
            if input.peek(syn::token::Brace) {
                let inner;
                let _ = braced!(inner in input);
                item = Item::parse(&inner, &mut names)?;
                require_comma = false;
            } else {
                item = Item::parse(input, &mut names)?;
                require_comma = true;
            }
            items.push(item);

            if input.is_empty() {
                break;
            }

            if let Err(e) = input.parse::<Token![,]>() {
                if require_comma {
                    return Err(e);
                }
            }
        }

        Ok(CellCollection(cells, Collection(items)))
    }
}

impl CellCollection {
    fn parse_aligned<Kw: Parse>(input: ParseStream, transmute: bool) -> Result<Self> {
        let mut names = NameGenerator::default();
        let mut cells = vec![];
        let mut items = vec![];

        let mut row = 0;
        while !input.is_empty() {
            let _: Kw = input.parse()?;
            let _: Token![!] = input.parse()?;

            let inner;
            let _ = bracketed!(inner in input);
            let mut col = 0;
            while !inner.is_empty() {
                let (mut a, mut b) = (col, row);
                if transmute {
                    (a, b) = (b, a);
                }
                cells.push(CellInfo::new(a, b));
                items.push(Item::parse(&inner, &mut names)?);

                if inner.is_empty() {
                    break;
                }
                let _: Token![,] = inner.parse()?;
                col += 1;
            }

            if input.is_empty() {
                break;
            }
            let _: Token![,] = input.parse()?;
            row += 1;
        }

        Ok(CellCollection(cells, Collection(items)))
    }
}

impl Collection {
    pub fn impl_parts(&self) -> (Toks, Toks, Toks, Toks, Toks) {
        let mut data_ty = None;
        for (index, item) in self.0.iter().enumerate() {
            if let Item::Widget(_, expr) = item {
                let ty = Ident::new(&format!("_W{index}"), expr.span());
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

        let mut get_tile_rules = quote! {};
        let mut get_mut_tile_rules = quote! {};
        let mut for_node_rules = quote! {};

        for (index, item) in self.0.iter().enumerate() {
            let path = match item {
                Item::Label(stor, ty, def) => {
                    if let Some(ref data_ty) = data_ty {
                        stor_ty.append_all(
                            quote! { #stor: ::kas::widgets::adapt::MapAny<#data_ty, #ty>, },
                        );
                        stor_def.append_all(
                            quote! { #stor: ::kas::widgets::adapt::MapAny::new(#def), },
                        );
                    } else {
                        stor_ty.append_all(quote! { #stor: #ty, });
                        stor_def.append_all(quote! { #stor: #def, });
                    }
                    stor.to_token_stream()
                }
                Item::Widget(stor, expr) => {
                    let span = expr.span();
                    let ty = Ident::new(&format!("_W{index}"), span);
                    stor_ty.append_all(quote! { #stor: #ty, });
                    stor_def.append_all(quote_spanned! {span=> #stor: Box::new(#expr), });
                    ty_generics.push(ty);

                    stor.to_token_stream()
                }
            };

            get_tile_rules.append_all(quote! {
                #index => Some(&self.#path),
            });
            get_mut_tile_rules.append_all(quote! {
                #index => Some(&mut self.#path),
            });
            for_node_rules.append_all(quote! {
                #index => Some(self.#path.as_node(data)),
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

        let collection = quote! {
            type Data = #data_ty;

            fn is_empty(&self) -> bool { #is_empty }
            fn len(&self) -> usize { #len }

            fn get_tile(&self, index: usize) -> Option<&dyn ::kas::Tile> {
                match index {
                    #get_tile_rules
                    _ => None,
                }
            }
            fn get_mut_tile(&mut self, index: usize) -> Option<&mut dyn ::kas::Tile> {
                match index {
                    #get_mut_tile_rules
                    _ => None,
                }
            }
            #[inline]
            fn child_node<'__n>(
                &'__n mut self,
                data: &'__n Self::Data,
                index: usize,
            ) -> Option<::kas::Node<'__n>> {
                use ::kas::Widget;
                match index {
                    #for_node_rules
                    _ => None,
                }
            }
        };

        (impl_generics, ty_generics, stor_ty, stor_def, collection)
    }

    pub fn expand(&self) -> Toks {
        let name = Ident::new("_Collection", Span::call_site());
        let (impl_generics, ty_generics, stor_ty, stor_def, collection) = self.impl_parts();

        let toks = quote! {{
            struct #name #impl_generics {
                #stor_ty
            }

            impl #impl_generics ::kas::Collection for #name #ty_generics {
                #collection
            }

            #name {
                #stor_def
            }
        }};
        // println!("{}", toks);
        toks
    }
}

impl CellCollection {
    pub fn expand(&self) -> Toks {
        let name = Ident::new("_Collection", Span::call_site());
        let (impl_generics, ty_generics, stor_ty, stor_def, collection) = self.1.impl_parts();

        let mut cell_info_rules = quote! {};
        let mut dim = GridDimensions::default();
        for (index, cell) in self.0.iter().enumerate() {
            cell_info_rules.append_all(quote! {
                #index => Some(#cell),
            });
            dim.update(cell);
        }

        let toks = quote! {{
            struct #name #impl_generics {
                #stor_ty
            }

            impl #impl_generics ::kas::Collection for #name #ty_generics {
                #collection
            }

            impl #impl_generics ::kas::CellCollection for #name #ty_generics {
                fn cell_info(&self, index: usize) -> Option<::kas::layout::GridCellInfo> {
                    match index {
                        #cell_info_rules
                        _ => None,
                    }
                }

                fn grid_dimensions(&self) -> ::kas::layout::GridDimensions {
                    #dim
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
