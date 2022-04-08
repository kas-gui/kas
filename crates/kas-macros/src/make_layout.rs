// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{braced, bracketed, parenthesized, Expr, LitInt, Member, Token};

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
    custom_keyword!(list);
    custom_keyword!(slice);
    custom_keyword!(grid);
    custom_keyword!(single);
    custom_keyword!(default);
    custom_keyword!(top);
    custom_keyword!(bottom);
}

pub struct Input {
    pub core: Expr,
    pub layout: Tree,
}

#[derive(Debug)]
pub struct Tree(Layout);
impl Tree {
    pub fn generate<'a, I: ExactSizeIterator<Item = &'a Member>>(
        &'a self,
        children: I,
    ) -> Result<Toks> {
        self.0.generate(Some(children))
    }
}

#[derive(Debug)]
enum Layout {
    Align(Box<Layout>, Align),
    AlignSingle(Expr, Align),
    Single(Span),
    Widget(Expr),
    Frame(Box<Layout>, Expr),
    List(Direction, List),
    Slice(Direction, Expr),
    Grid(GridDimensions, Vec<(CellInfo, Layout)>),
}

#[derive(Debug)]
enum List {
    List(Vec<Layout>),
    Glob(Span),
}

#[derive(Debug)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
    Expr(Toks),
}

#[derive(Debug)]
enum Align {
    Default,
    Center,
    Stretch,
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Default)]
struct GridDimensions {
    cols: u32,
    col_spans: u32,
    rows: u32,
    row_spans: u32,
}
#[derive(Debug)]
struct CellInfo {
    col: u32,
    col_end: u32,
    row: u32,
    row_end: u32,
}
impl Parse for CellInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let row = input.parse::<LitInt>()?.base10_parse()?;
        let row_end = if input.peek(Token![..]) {
            let _ = input.parse::<Token![..]>();
            let lit = input.parse::<LitInt>()?;
            let end = lit.base10_parse()?;
            if row >= end {
                return Err(Error::new(lit.span(), format!("expected value > {}", row)));
            }
            end
        } else {
            row + 1
        };

        let _ = input.parse::<Token![,]>()?;

        let col = input.parse::<LitInt>()?.base10_parse()?;
        let col_end = if input.peek(Token![..]) {
            let _ = input.parse::<Token![..]>();
            let lit = input.parse::<LitInt>()?;
            let end = lit.base10_parse()?;
            if col >= end {
                return Err(Error::new(lit.span(), format!("expected value > {}", col)));
            }
            end
        } else {
            col + 1
        };

        Ok(CellInfo {
            row,
            row_end,
            col,
            col_end,
        })
    }
}
impl GridDimensions {
    fn update(&mut self, cell: &CellInfo) {
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

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let core = input.parse()?;
        let _: Token![;] = input.parse()?;
        let layout = input.parse()?;

        Ok(Input { core, layout })
    }
}

impl Parse for Tree {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Tree(input.parse()?))
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
            Ok(Layout::Widget(input.parse()?))
        } else if lookahead.peek(kw::single) {
            let tok: kw::single = input.parse()?;
            Ok(Layout::Single(tok.span()))
        } else if lookahead.peek(kw::frame) {
            let _: kw::frame = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let layout: Layout = inner.parse()?;
            let _: Token![,] = inner.parse()?;
            let style: Expr = inner.parse()?;
            Ok(Layout::Frame(Box::new(layout), style))
        } else if lookahead.peek(kw::column) {
            let _: kw::column = input.parse()?;
            let dir = Direction::Down;
            let _: Token![:] = input.parse()?;
            let list = parse_layout_list(input)?;
            Ok(Layout::List(dir, list))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let dir = Direction::Right;
            let _: Token![:] = input.parse()?;
            let list = parse_layout_list(input)?;
            Ok(Layout::List(dir, list))
        } else if lookahead.peek(kw::list) {
            let _: kw::list = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let _: Token![:] = input.parse()?;
            let list = parse_layout_list(input)?;
            Ok(Layout::List(dir, list))
        } else if lookahead.peek(kw::slice) {
            let _: kw::slice = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let _: Token![:] = input.parse()?;
            if input.peek(Token![self]) {
                Ok(Layout::Slice(dir, input.parse()?))
            } else {
                Err(Error::new(input.span(), "expected `self`"))
            }
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = input.parse()?;
            let _: Token![:] = input.parse()?;
            Ok(parse_grid(input)?)
        } else {
            Err(lookahead.error())
        }
    }
}

fn parse_align(input: ParseStream) -> Result<Align> {
    let inner;
    let _ = parenthesized!(inner in input);

    let lookahead = inner.lookahead1();
    if lookahead.peek(kw::default) {
        let _: kw::default = inner.parse()?;
        Ok(Align::Default)
    } else if lookahead.peek(kw::center) {
        let _: kw::center = inner.parse()?;
        Ok(Align::Center)
    } else if lookahead.peek(kw::stretch) {
        let _: kw::stretch = inner.parse()?;
        Ok(Align::Stretch)
    } else if lookahead.peek(kw::top) {
        let _: kw::top = inner.parse()?;
        Ok(Align::Top)
    } else if lookahead.peek(kw::bottom) {
        let _: kw::bottom = inner.parse()?;
        Ok(Align::Bottom)
    } else if lookahead.peek(kw::left) {
        let _: kw::left = inner.parse()?;
        Ok(Align::Left)
    } else if lookahead.peek(kw::right) {
        let _: kw::right = inner.parse()?;
        Ok(Align::Right)
    } else {
        Err(lookahead.error())
    }
}

fn parse_layout_list(input: ParseStream) -> Result<List> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Token![*]) {
        let tok = input.parse::<Token![*]>()?;
        Ok(List::Glob(tok.span()))
    } else if lookahead.peek(syn::token::Bracket) {
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

        Ok(List::List(list))
    } else {
        Err(lookahead.error())
    }
}

fn parse_grid(input: ParseStream) -> Result<Layout> {
    let inner;
    let _ = braced!(inner in input);

    let mut dim = GridDimensions::default();
    let mut cells = vec![];
    while !inner.is_empty() {
        let info = inner.parse()?;
        dim.update(&info);
        let _: Token![:] = inner.parse()?;
        let layout = inner.parse()?;
        cells.push((info, layout));

        if inner.is_empty() {
            break;
        }

        let _: Token![;] = inner.parse()?;
    }

    Ok(Layout::Grid(dim, cells))
}

impl Parse for Direction {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::right) {
            let _: kw::right = input.parse()?;
            Ok(Direction::Right)
        } else if lookahead.peek(kw::down) {
            let _: kw::down = input.parse()?;
            Ok(Direction::Down)
        } else if lookahead.peek(kw::left) {
            let _: kw::left = input.parse()?;
            Ok(Direction::Left)
        } else if lookahead.peek(kw::up) {
            let _: kw::up = input.parse()?;
            Ok(Direction::Up)
        } else if lookahead.peek(Token![self]) {
            Ok(Direction::Expr(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl quote::ToTokens for Align {
    fn to_tokens(&self, toks: &mut Toks) {
        toks.append_all(match self {
            Align::Default => quote! { layout::AlignHints::NONE },
            Align::Center => quote! { layout::AlignHints::CENTER },
            Align::Stretch => quote! { layout::AlignHints::STRETCH },
            Align::Top => quote! { layout::AlignHints::new(None, Some(layout::Align::TL))},
            Align::Bottom => quote! { layout::AlignHints::new(None, Some(layout::Align::BR)) },
            Align::Left => quote! { layout::AlignHints::new(Some(layout::Align::TL), None) },
            Align::Right => quote! { layout::AlignHints::new(Some(layout::Align::BR), None) },
        });
    }
}

impl quote::ToTokens for Direction {
    fn to_tokens(&self, toks: &mut Toks) {
        match self {
            Direction::Left => toks.append_all(quote! { ::kas::dir::Left }),
            Direction::Right => toks.append_all(quote! { ::kas::dir::Right }),
            Direction::Up => toks.append_all(quote! { ::kas::dir::Up }),
            Direction::Down => toks.append_all(quote! { ::kas::dir::Down }),
            Direction::Expr(expr) => expr.to_tokens(toks),
        }
    }
}

impl quote::ToTokens for GridDimensions {
    fn to_tokens(&self, toks: &mut Toks) {
        let (cols, rows) = (self.cols, self.rows);
        let (col_spans, row_spans) = (self.col_spans, self.row_spans);
        toks.append_all(quote! { layout::GridDimensions {
            cols: #cols,
            col_spans: #col_spans,
            rows: #rows,
            row_spans: #row_spans,
        } });
    }
}

impl Layout {
    // Optionally pass in the list of children, but not when already in a
    // multi-element layout (list/slice/grid).
    //
    // Required: `::kas::layout` must be in scope.
    fn generate<'a, I: ExactSizeIterator<Item = &'a Member>>(
        &'a self,
        children: Option<I>,
    ) -> Result<Toks> {
        Ok(match self {
            Layout::Align(layout, align) => {
                let inner = layout.generate(children)?;
                quote! { layout::Layout::align(#inner, #align) }
            }
            Layout::AlignSingle(expr, align) => {
                quote! { layout::Layout::align_single(#expr.as_widget_mut(), #align) }
            }
            Layout::Widget(expr) => quote! {
                layout::Layout::single(#expr.as_widget_mut())
            },
            Layout::Single(span) => {
                if let Some(mut iter) = children {
                    if iter.len() != 1 {
                        return Err(Error::new(
                            *span,
                            "layout `single`: widget does not have exactly one child",
                        ));
                    }
                    let child = iter.next().unwrap();
                    quote! {
                        layout::Layout::single(self.#child.as_widget_mut())
                    }
                } else {
                    return Err(Error::new(
                        *span,
                        "layout `single` is unavailable in this context",
                    ));
                }
            }
            Layout::Frame(layout, style) => {
                let inner = layout.generate(children)?;
                quote! {
                    let (data, next) = _chain.storage::<layout::FrameStorage>();
                    _chain = next;
                    layout::Layout::frame(data, #inner, #style)
                }
            }
            Layout::List(dir, list) => {
                let len;
                let mut items = Toks::new();
                match list {
                    List::List(list) => {
                        len = list.len();
                        for item in list {
                            let item = item.generate::<std::iter::Empty<&Member>>(None)?;
                            items.append_all(quote! {{ #item },});
                        }
                    }
                    List::Glob(span) => {
                        if let Some(iter) = children {
                            len = iter.len();
                            for member in iter {
                                items.append_all(quote! {
                                    layout::Layout::single(self.#member.as_widget_mut()),
                                });
                            }
                        } else {
                            return Err(Error::new(
                                *span,
                                "glob `*` is unavailable in this context",
                            ));
                        }
                    }
                }

                let storage = if len > 16 {
                    quote! { layout::DynRowStorage }
                } else {
                    quote! { layout::FixedRowStorage<#len> }
                };
                // Get a storage slot from the chain. Order doesn't matter.
                let data = quote! { {
                    let (data, next) = _chain.storage::<#storage>();
                    _chain = next;
                    data
                } };

                let iter = quote! { { let arr = [#items]; arr.into_iter() } };

                quote! { layout::Layout::list(#iter, #dir, #data) }
            }
            Layout::Slice(dir, expr) => {
                let data = quote! { {
                    let (data, next) = _chain.storage::<layout::DynRowStorage>();
                    _chain = next;
                    data
                } };
                quote! { layout::Layout::slice(&mut #expr, #dir, #data) }
            }
            Layout::Grid(dim, cells) => {
                let (cols, rows) = (dim.cols as usize, dim.rows as usize);
                let data = quote! { {
                    let (data, next) = _chain.storage::<layout::FixedGridStorage<#cols, #rows>>();
                    _chain = next;
                    data
                } };

                let mut items = Toks::new();
                for item in cells {
                    let (col, col_end) = (item.0.col, item.0.col_end);
                    let (row, row_end) = (item.0.row, item.0.row_end);
                    let layout = item.1.generate::<std::iter::Empty<&Member>>(None)?;
                    items.append_all(quote! {
                        (
                            layout::GridChildInfo {
                                col: #col,
                                col_end: #col_end,
                                row: #row,
                                row_end: #row_end,
                            },
                            #layout,
                        ),
                    });
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };

                quote! { layout::Layout::grid(#iter, #dim, #data) }
            }
        })
    }
}

pub fn make_layout(input: Input) -> Result<Toks> {
    let core = &input.core;
    let layout = input.layout.0.generate::<std::iter::Empty<&Member>>(None)?;
    Ok(quote! { {
        use ::kas::{WidgetCore, layout};
        let mut _chain = &mut #core.layout;
        #layout
    } })
}
