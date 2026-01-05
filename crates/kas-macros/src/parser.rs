// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Common parsing code for collections and layouts

use crate::collection::{CellInfo, GridDimensions, NameGenerator};
use syn::parse::{ParseStream, Result};
use syn::{Error, Ident, Token};
use syn::{braced, bracketed};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(column);
    custom_keyword!(row);
}

pub trait Parser {
    type Output;

    fn parse(input: ParseStream, core_gen: &mut NameGenerator) -> Result<Self::Output>;
}

pub fn parse_grid<P: Parser>(
    inner: ParseStream,
    core_gen: &mut NameGenerator,
) -> Result<(GridDimensions, Vec<CellInfo>, Vec<P::Output>)> {
    let mut dim = GridDimensions::default();
    let mut infos = vec![];
    let mut items = vec![];
    while !inner.is_empty() {
        let mut require_comma = true;
        if inner.peek2(Token![!]) {
            let lookahead = inner.lookahead1();
            if lookahead.peek(kw::column) {
                let _: kw::column = inner.parse()?;
                let _: Token![!] = inner.parse()?;

                let inner2;
                let _ = bracketed!(inner2 in inner);
                let col = dim.cols;
                let mut row = 0;
                while !inner2.is_empty() {
                    if let Ok(_) = inner2.parse::<Token![_]>() {
                        // empty item
                    } else {
                        let layout = P::parse(&inner2, core_gen)?;
                        let cell = CellInfo::new(col, row);
                        dim.update(&cell);
                        infos.push(cell);
                        items.push(layout);
                    }
                    row += 1;

                    if inner2.is_empty() {
                        break;
                    }

                    if let Err(e) = inner2.parse::<Token![,]>() {
                        return Err(e);
                    }
                }
            } else if lookahead.peek(kw::row) {
                let _: kw::row = inner.parse()?;
                let _: Token![!] = inner.parse()?;

                let inner2;
                let _ = bracketed!(inner2 in inner);
                let mut col = 0;
                let row = dim.rows;
                while !inner2.is_empty() {
                    if let Ok(_) = inner2.parse::<Token![_]>() {
                        // empty item
                    } else {
                        let layout = P::parse(&inner2, core_gen)?;
                        let cell = CellInfo::new(col, row);
                        dim.update(&cell);
                        infos.push(cell);
                        items.push(layout);
                    }
                    col += 1;

                    if inner2.is_empty() {
                        break;
                    }

                    if let Err(e) = inner2.parse::<Token![,]>() {
                        return Err(e);
                    }
                }
            } else {
                let ident: Ident = inner.parse()?;
                let tok: Token![!] = inner.parse()?;
                let span = ident.span();
                let span = span.join(tok.span).unwrap_or(span);
                return Err(Error::new(span, "expected: `column!` or `row!`"));
            }
        } else {
            let cell = inner.parse()?;
            dim.update(&cell);
            let _: Token![=>] = inner.parse()?;

            let layout;
            if inner.peek(syn::token::Brace) {
                let inner2;
                let _ = braced!(inner2 in inner);
                layout = P::parse(&inner2, core_gen)?;
                require_comma = false;
            } else {
                layout = P::parse(inner, core_gen)?;
            }
            infos.push(cell);
            items.push(layout);
        }

        if inner.is_empty() {
            break;
        }

        if let Err(e) = inner.parse::<Token![,]>() {
            if require_comma {
                return Err(e);
            }
        }
    }

    Ok((dim, infos, items))
}
