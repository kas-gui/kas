// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{braced, bracketed, parenthesized, Expr, Ident, Lifetime, LitInt, LitStr, Member, Token};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(align);
    custom_keyword!(column);
    custom_keyword!(row);
    custom_keyword!(right);
    custom_keyword!(left);
    custom_keyword!(down);
    custom_keyword!(up);
    custom_keyword!(center);
    custom_keyword!(stretch);
    custom_keyword!(frame);
    custom_keyword!(button);
    custom_keyword!(list);
    custom_keyword!(slice);
    custom_keyword!(grid);
    custom_keyword!(default);
    custom_keyword!(top);
    custom_keyword!(bottom);
    custom_keyword!(aligned_column);
    custom_keyword!(aligned_row);
}

#[derive(Debug)]
pub struct Tree(Layout);
impl Tree {
    /// If extra fields are needed for storage, return these: `(fields_ty, fields_init)`
    /// (e.g. `({ layout_frame: FrameStorage, }, { layout_frame: Default::default()), }`).
    pub fn storage_fields(&self) -> Option<(Toks, Toks)> {
        let (mut ty_toks, mut def_toks) = (Toks::new(), Toks::new());
        self.0.append_fields(&mut ty_toks, &mut def_toks);
        if ty_toks.is_empty() && def_toks.is_empty() {
            None
        } else {
            Some((ty_toks, def_toks))
        }
    }

    pub fn generate<'a, I: ExactSizeIterator<Item = &'a Member>>(
        &'a self,
        core: &Member,
        children: I,
    ) -> Result<Toks> {
        self.0.generate(core, Some(children))
    }
}

#[derive(Debug)]
enum StorIdent {
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

#[derive(Debug)]
enum Layout {
    Align(Box<Layout>, Align),
    AlignSingle(Expr, Align),
    Widget(Expr),
    Frame(StorIdent, Box<Layout>, Expr),
    Button(StorIdent, Box<Layout>, Expr),
    List(StorIdent, Direction, List),
    Slice(StorIdent, Direction, Expr),
    Grid(StorIdent, GridDimensions, Vec<(CellInfo, Layout)>),
    Label(StorIdent, LitStr),
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

#[derive(Copy, Clone, Debug)]
struct CellInfo {
    col: u32,
    col_end: u32,
    row: u32,
    row_end: u32,
}

impl CellInfo {
    fn new(col: u32, row: u32) -> Self {
        CellInfo {
            col,
            col_end: col + 1,
            row,
            row_end: row + 1,
        }
    }
}

fn parse_cell_info(input: ParseStream) -> Result<CellInfo> {
    fn parse_end(input: ParseStream, start: u32) -> Result<u32> {
        if input.parse::<Token![..]>().is_ok() {
            if input.parse::<Token![+]>().is_ok() {
                return Ok(start + input.parse::<LitInt>()?.base10_parse::<u32>()?);
            }

            let lit = input.parse::<LitInt>()?;
            let end = lit.base10_parse()?;
            if start >= end {
                return Err(Error::new(
                    lit.span(),
                    format!("expected value > {}", start),
                ));
            }
            Ok(end)
        } else {
            Ok(start + 1)
        }
    }

    let col = input.parse::<LitInt>()?.base10_parse()?;
    let col_end = parse_end(input, col)?;

    let _ = input.parse::<Token![,]>()?;

    let row = input.parse::<LitInt>()?.base10_parse()?;
    let row_end = parse_end(input, row)?;

    Ok(CellInfo {
        row,
        row_end,
        col,
        col_end,
    })
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

#[derive(Default)]
struct NameGenerator(usize);
impl NameGenerator {
    fn next(&mut self) -> StorIdent {
        let name = format!("stor{}", self.0);
        self.0 += 1;
        StorIdent::Generated(name, Span::call_site())
    }

    fn parse_or_next(&mut self, input: ParseStream) -> Result<StorIdent> {
        if input.peek(Lifetime) {
            Ok(input.parse::<Lifetime>()?.into())
        } else {
            Ok(self.next())
        }
    }
}

impl Parse for Tree {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        Ok(Tree(Layout::parse(input, &mut gen)?))
    }
}

impl Layout {
    fn parse(input: ParseStream, gen: &mut NameGenerator) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::align) {
            let _: kw::align = input.parse()?;
            let align = parse_align(input)?;
            let _: Token![:] = input.parse()?;

            if input.peek(Token![self]) {
                Ok(Layout::AlignSingle(input.parse()?, align))
            } else {
                let layout = Layout::parse(input, gen)?;
                Ok(Layout::Align(Box::new(layout), align))
            }
        } else if lookahead.peek(Token![self]) {
            Ok(Layout::Widget(input.parse()?))
        } else if lookahead.peek(kw::frame) {
            let _: kw::frame = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let style: Expr = inner.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            let layout = Layout::parse(input, gen)?;
            Ok(Layout::Frame(stor, Box::new(layout), style))
        } else if lookahead.peek(kw::button) {
            let _: kw::button = input.parse()?;
            let mut color: Expr = syn::parse_quote! { None };
            if input.peek(syn::token::Paren) {
                let inner;
                let _ = parenthesized!(inner in input);
                color = inner.parse()?;
            }
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            let layout = Layout::parse(input, gen)?;
            Ok(Layout::Button(stor, Box::new(layout), color))
        } else if lookahead.peek(kw::column) {
            let _: kw::column = input.parse()?;
            let dir = Direction::Down;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let dir = Direction::Right;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::list) {
            let _: kw::list = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::aligned_column) {
            let _: kw::aligned_column = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            Ok(parse_grid_as_list_of_lists::<kw::row>(
                stor, input, gen, true,
            )?)
        } else if lookahead.peek(kw::aligned_row) {
            let _: kw::aligned_row = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            Ok(parse_grid_as_list_of_lists::<kw::column>(
                stor, input, gen, false,
            )?)
        } else if lookahead.peek(kw::slice) {
            let _: kw::slice = input.parse()?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            if input.peek(Token![self]) {
                Ok(Layout::Slice(stor, dir, input.parse()?))
            } else {
                Err(Error::new(input.span(), "expected `self`"))
            }
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            Ok(parse_grid(stor, input, gen)?)
        } else if lookahead.peek(LitStr) {
            let stor = gen.next();
            Ok(Layout::Label(stor, input.parse()?))
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

fn parse_layout_list(input: ParseStream, gen: &mut NameGenerator) -> Result<List> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Token![*]) {
        let tok = input.parse::<Token![*]>()?;
        Ok(List::Glob(tok.span()))
    } else if lookahead.peek(syn::token::Bracket) {
        let inner;
        let _ = bracketed!(inner in input);

        let mut list = vec![];
        while !inner.is_empty() {
            list.push(Layout::parse(&inner, gen)?);

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

fn parse_grid_as_list_of_lists<KW: Parse>(
    stor: StorIdent,
    input: ParseStream,
    gen: &mut NameGenerator,
    row_major: bool,
) -> Result<Layout> {
    let inner;
    let _ = bracketed!(inner in input);

    let (mut col, mut row) = (0, 0);
    let mut dim = GridDimensions::default();
    let mut cells = vec![];

    while !inner.is_empty() {
        let _ = inner.parse::<KW>()?;
        let _ = inner.parse::<Token![:]>()?;

        let inner2;
        let _ = bracketed!(inner2 in inner);

        while !inner2.is_empty() {
            let info = CellInfo::new(col, row);
            dim.update(&info);
            let layout = Layout::parse(&inner2, gen)?;
            cells.push((info, layout));

            if inner2.is_empty() {
                break;
            }
            let _: Token![,] = inner2.parse()?;

            if row_major {
                col += 1;
            } else {
                row += 1;
            }
        }

        if inner.is_empty() {
            break;
        }
        let _: Token![,] = inner.parse()?;

        if row_major {
            col = 0;
            row += 1;
        } else {
            row = 0;
            col += 1;
        }
    }

    Ok(Layout::Grid(stor, dim, cells))
}

fn parse_grid(stor: StorIdent, input: ParseStream, gen: &mut NameGenerator) -> Result<Layout> {
    let inner;
    let _ = braced!(inner in input);

    let mut dim = GridDimensions::default();
    let mut cells = vec![];
    while !inner.is_empty() {
        let info = parse_cell_info(&inner)?;
        dim.update(&info);
        let _: Token![:] = inner.parse()?;
        let layout = Layout::parse(&inner, gen)?;
        cells.push((info, layout));

        if inner.is_empty() {
            break;
        }

        let _: Token![;] = inner.parse()?;
    }

    Ok(Layout::Grid(stor, dim, cells))
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

impl ToTokens for Align {
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

impl ToTokens for Direction {
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

impl ToTokens for GridDimensions {
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
    fn append_fields(&self, ty_toks: &mut Toks, def_toks: &mut Toks) {
        match self {
            Layout::Align(layout, _) => {
                layout.append_fields(ty_toks, def_toks);
            }
            Layout::AlignSingle(..) | Layout::Widget(_) => (),
            Layout::Frame(stor, layout, _) | Layout::Button(stor, layout, _) => {
                stor.to_tokens(ty_toks);
                ty_toks.append_all(quote! { : ::kas::layout::FrameStorage, });
                stor.to_tokens(def_toks);
                def_toks.append_all(quote! { : Default::default(), });
                layout.append_fields(ty_toks, def_toks);
            }
            Layout::List(stor, _, list) => {
                stor.to_tokens(ty_toks);
                stor.to_tokens(def_toks);
                def_toks.append_all(quote! { : Default::default(), });
                match list {
                    List::List(vec) => {
                        let len = vec.len();
                        ty_toks.append_all(if len > 16 {
                            quote! { : ::kas::layout::DynRowStorage, }
                        } else {
                            quote! { : ::kas::layout::FixedRowStorage<#len>, }
                        });
                        for item in vec {
                            item.append_fields(ty_toks, def_toks);
                        }
                    }
                    List::Glob(_) => {
                        // TODO(opt): use FixedRowStorage?
                        ty_toks.append_all(quote! { : ::kas::layout::DynRowStorage, });
                        // only simple items supported, so there is nothing to recurse
                    }
                }
            }
            Layout::Slice(stor, _, _) => {
                stor.to_tokens(ty_toks);
                ty_toks.append_all(quote! { : ::kas::layout::DynRowStorage, });
                stor.to_tokens(def_toks);
                def_toks.append_all(quote! { : Default::default(), });
            }
            Layout::Grid(stor, dim, cells) => {
                let (cols, rows) = (dim.cols as usize, dim.rows as usize);
                stor.to_tokens(ty_toks);
                ty_toks.append_all(quote! { : ::kas::layout::FixedGridStorage<#cols, #rows>, });
                stor.to_tokens(def_toks);
                def_toks.append_all(quote! { : Default::default(), });

                for (_info, layout) in cells {
                    layout.append_fields(ty_toks, def_toks);
                }
            }
            Layout::Label(stor, text) => {
                stor.to_tokens(ty_toks);
                ty_toks.append_all(quote! { : ::kas::component::Label<&'static str>, });
                stor.to_tokens(def_toks);
                def_toks.append_all(quote! { : ::kas::component::Label::new(#text, ::kas::theme::TextClass::Label(false)), });
            }
        }
    }

    // Optionally pass in the list of children, but not when already in a
    // multi-element layout (list/slice/grid).
    //
    // Required: `::kas::layout` must be in scope.
    fn generate<'a, I: ExactSizeIterator<Item = &'a Member>>(
        &'a self,
        core: &Member,
        children: Option<I>,
    ) -> Result<Toks> {
        Ok(match self {
            Layout::Align(layout, align) => {
                let inner = layout.generate(core, children)?;
                quote! { layout::Visitor::align(#inner, #align) }
            }
            Layout::AlignSingle(expr, align) => {
                quote! { layout::Visitor::align_single(&mut (#expr), #align) }
            }
            Layout::Widget(expr) => quote! {
                layout::Visitor::single(&mut (#expr))
            },
            Layout::Frame(stor, layout, style) => {
                let inner = layout.generate(core, children)?;
                quote! {
                    layout::Visitor::frame(&mut self.#core.#stor, #inner, #style)
                }
            }
            Layout::Button(stor, layout, color) => {
                let inner = layout.generate(core, children)?;
                quote! {
                    layout::Visitor::button(&mut self.#core.#stor, #inner, #color)
                }
            }
            Layout::List(stor, dir, list) => {
                let mut items = Toks::new();
                match list {
                    List::List(list) => {
                        for item in list {
                            let item = item.generate::<std::iter::Empty<&Member>>(core, None)?;
                            items.append_all(quote! {{ #item },});
                        }
                    }
                    List::Glob(span) => {
                        if let Some(iter) = children {
                            for member in iter {
                                items.append_all(quote! {
                                    layout::Visitor::single(&mut self.#member),
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

                let iter = quote! { { let arr = [#items]; arr.into_iter() } };

                quote! { layout::Visitor::list(#iter, #dir, &mut self.#core.#stor) }
            }
            Layout::Slice(stor, dir, expr) => {
                quote! { layout::Visitor::slice(&mut #expr, #dir, &mut self.#core.#stor) }
            }
            Layout::Grid(stor, dim, cells) => {
                let mut items = Toks::new();
                for item in cells {
                    let (col, col_end) = (item.0.col, item.0.col_end);
                    let (row, row_end) = (item.0.row, item.0.row_end);
                    let layout = item.1.generate::<std::iter::Empty<&Member>>(core, None)?;
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

                quote! { layout::Visitor::grid(#iter, #dim, &mut self.#core.#stor) }
            }
            Layout::Label(stor, _) => {
                quote! { layout::Visitor::single(&mut self.#core.#stor) }
            }
        })
    }
}
