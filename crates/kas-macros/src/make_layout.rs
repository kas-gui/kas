// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::TokenStream as Toks;
use quote::{quote, TokenStreamExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::{bracketed, parenthesized, Token};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(align);
    custom_keyword!(col);
    custom_keyword!(column);
    custom_keyword!(row);
    custom_keyword!(right);
    custom_keyword!(left);
    custom_keyword!(down);
    custom_keyword!(up);
    custom_keyword!(center);
    custom_keyword!(stretch);
    custom_keyword!(frame);
}

pub struct Input {
    core: syn::Expr,
    layout: Layout,
}

enum Layout {
    Align(Box<Layout>, Align),
    AlignSingle(syn::Expr, Align),
    Single(syn::Expr),
    Frame(Box<Layout>),
    List(List),
}

struct List {
    dir: Toks,
    list: Vec<Layout>,
}

enum Align {
    Center,
    Stretch,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let core = input.parse()?;
        let _: Token![;] = input.parse()?;
        let layout = input.parse()?;

        Ok(Input { core, layout })
    }
}

impl Parse for Layout {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::align) {
            let _: kw::align = input.parse()?;
            let align = parse_align(input)?;
            let _: Token![:] = input.parse()?;

            if input.peek(Token![self]) {
                Ok(Layout::AlignSingle(input.parse()?, align))
            } else {
                let layout: Layout = input.parse()?;
                Ok(Layout::Align(Box::new(layout), align))
            }
        } else if lookahead.peek(Token![self]) {
            Ok(Layout::Single(input.parse()?))
        } else if lookahead.peek(kw::frame) {
            let _: kw::frame = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let layout: Layout = inner.parse()?;
            Ok(Layout::Frame(Box::new(layout)))
        } else if lookahead.peek(kw::column) {
            let _: kw::column = input.parse()?;
            let dir = quote! { ::kas::dir::Down };
            let list = parse_layout_list(input)?;
            Ok(Layout::List(List { dir, list }))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let dir = quote! { ::kas::dir::Right };
            let list = parse_layout_list(input)?;
            Ok(Layout::List(List { dir, list }))
        } else {
            Err(lookahead.error())
        }
    }
}

fn parse_align(input: ParseStream) -> Result<Align> {
    let inner;
    let _ = parenthesized!(inner in input);

    let lookahead = inner.lookahead1();
    if lookahead.peek(kw::center) {
        let _: kw::center = inner.parse()?;
        Ok(Align::Center)
    } else if lookahead.peek(kw::stretch) {
        let _: kw::stretch = inner.parse()?;
        Ok(Align::Stretch)
    } else {
        Err(lookahead.error())
    }
}

fn parse_layout_list(input: ParseStream) -> Result<Vec<Layout>> {
    let inner;
    let _ = bracketed!(inner in input);

    let mut list = vec![];
    while !inner.is_empty() {
        list.push(inner.parse::<Layout>()?);

        if inner.is_empty() {
            break;
        }

        let _: Token![,] = inner.parse()?;
    }

    Ok(list)
}

impl quote::ToTokens for Align {
    fn to_tokens(&self, toks: &mut Toks) {
        toks.append_all(match self {
            Align::Center => quote! { ::kas::layout::AlignHints::CENTER },
            Align::Stretch => quote! { ::kas::layout::AlignHints::STRETCH },
        });
    }
}

impl Layout {
    fn generate(&self) -> Toks {
        match self {
            Layout::Align(layout, align) => {
                let inner = layout.generate();
                quote! { layout::Layout::align(#inner, #align) }
            }
            Layout::AlignSingle(expr, align) => {
                quote! { layout::Layout::align_single(#expr.as_widget_mut(), #align) }
            }
            Layout::Single(expr) => quote! {
                layout::Layout::single(#expr.as_widget_mut())
            },
            Layout::Frame(layout) => {
                let inner = layout.generate();
                quote! {
                    let (data, next) = _chain.storage::<::kas::layout::FrameStorage>();
                    _chain = next;
                    layout::Layout::frame(data, #inner)
                }
            }
            Layout::List(List { dir, list }) => {
                let len = list.len();
                let storage = if len > 16 {
                    quote! { ::kas::layout::DynRowStorage }
                } else {
                    quote! { ::kas::layout::FixedRowStorage<#len> }
                };
                // Get a storage slot from the chain. Order doesn't matter.
                let data = quote! { {
                    let (data, next) = _chain.storage::<#storage>();
                    _chain = next;
                    data
                } };

                let mut items = Toks::new();
                for item in list {
                    let item = item.generate();
                    items.append_all(quote! { #item, });
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };

                quote! { ::kas::layout::Layout::list(#iter, #dir, #data) }
            }
        }
    }
}

pub fn make_layout(input: Input) -> Toks {
    let core = &input.core;
    let layout = input.layout.generate();
    quote! { {
        let mut _chain = &mut #core.layout;
        #layout
    } }
}
