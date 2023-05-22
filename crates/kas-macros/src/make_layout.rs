// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Span, TokenStream as Toks};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{braced, bracketed, parenthesized, Expr, Ident, Lifetime, LitInt, LitStr, Member, Token};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(align);
    custom_keyword!(pack);
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
    custom_keyword!(float);
    custom_keyword!(margins);
    custom_keyword!(non_navigable);
    custom_keyword!(px);
    custom_keyword!(em);
}

#[derive(Debug)]
pub struct Tree(Layout);
impl Tree {
    /// If extra fields are needed for storage, return these: `(fields_ty, fields_init)`
    /// (e.g. `({ layout_frame: FrameStorage, }, { layout_frame: Default::default()), }`).
    pub fn storage_fields(&self, children: &mut Vec<Toks>) -> Option<(Toks, Toks)> {
        let (mut ty_toks, mut def_toks) = (Toks::new(), Toks::new());
        self.0.append_fields(&mut ty_toks, &mut def_toks, children);
        if ty_toks.is_empty() && def_toks.is_empty() {
            None
        } else {
            Some((ty_toks, def_toks))
        }
    }

    pub fn generate(&self, core: &Member) -> Result<Toks> {
        self.0.generate(core)
    }

    pub fn nav_next<'a, I: Clone + ExactSizeIterator<Item = &'a Member>>(
        &self,
        children: I,
    ) -> NavNextResult {
        match &self.0 {
            Layout::Slice(_, dir, _) => NavNextResult::Slice(dir.to_token_stream()),
            layout => {
                let mut v = Vec::new();
                let mut index = children.len();
                match layout.nav_next(children, &mut v, &mut index) {
                    Ok(()) => NavNextResult::List(v),
                    Err((span, msg)) => NavNextResult::Err(span, msg),
                }
            }
        }
    }

    /// If field `ident` is included in the layout, return Span of usage
    pub fn span_in_layout(&self, ident: &Member) -> Option<Span> {
        self.0.span_in_layout(ident)
    }
}

#[derive(Debug)]
pub enum NavNextResult {
    Err(Span, &'static str),
    Slice(Toks),
    List(Vec<usize>),
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
    Align(Box<Layout>, AlignHints),
    AlignSingle(ExprMember, AlignHints),
    Pack(StorIdent, Box<Layout>, AlignHints),
    Margins(Box<Layout>, Directions, Toks),
    Single(ExprMember),
    Widget(StorIdent, Expr),
    Frame(StorIdent, Box<Layout>, Expr),
    Button(StorIdent, Box<Layout>, Expr),
    List(StorIdent, Direction, Vec<Layout>),
    Float(Vec<Layout>),
    Slice(StorIdent, Direction, Expr),
    Grid(StorIdent, GridDimensions, Vec<(CellInfo, Layout)>),
    Label(StorIdent, LitStr),
    NonNavigable(Box<Layout>),
}

#[derive(Debug)]
struct ExprMember {
    self_: Token![self],
    p: Token![.],
    member: Member,
}

#[derive(Debug)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
    Expr(Expr),
}

bitflags::bitflags! {
    // NOTE: this must match kas::dir::Directions!
    struct Directions: u8 {
        const LEFT = 0b0001;
        const RIGHT = 0b0010;
        const UP = 0b0100;
        const DOWN = 0b1000;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Align {
    None,
    Default,
    TL,
    Center,
    BR,
    Stretch,
}

#[derive(Debug, PartialEq, Eq)]
struct AlignHints(Align, Align);

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
                return Err(Error::new(lit.span(), format!("expected value > {start}")));
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
        let name = format!("_stor{}", self.0);
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
        } else if lookahead.peek(kw::pack) {
            let _: kw::pack = input.parse()?;
            let align = parse_align(input)?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;

            let layout = Layout::parse(input, gen)?;
            Ok(Layout::Pack(stor, Box::new(layout), align))
        } else if lookahead.peek(kw::margins) {
            let _ = input.parse::<kw::margins>()?;
            let inner;
            let _ = parenthesized!(inner in input);

            let mut dirs = Directions::all();
            if inner.peek2(Token![=]) {
                let ident = inner.parse::<Ident>()?;
                dirs = match ident {
                    id if id == "horiz" || id == "horizontal" => Directions::LEFT | Directions::RIGHT,
                    id if id == "vert" || id == "vertical" => Directions::UP | Directions::DOWN,
                    id if id == "left" => Directions::LEFT,
                    id if id == "right" => Directions::RIGHT,
                    id if id == "top" => Directions::UP,
                    id if id == "bottom" => Directions::DOWN,
                    _ => return Err(Error::new(ident.span(), "expected one of: horiz, horizontal, vert, vertical, left, right, top, bottom")),
                };
                let _ = inner.parse::<Token![=]>()?;
            }

            let lookahead = inner.lookahead1();
            let margins = if lookahead.peek(syn::LitFloat) {
                let val = inner.parse::<syn::LitFloat>()?;
                let digits = val.base10_digits();
                match val.suffix() {
                    "px" => quote! { Px(#digits) },
                    "em" => quote! { Em(#digits) },
                    _ => return Err(Error::new(val.span(), "expected suffix `px` or `em`")),
                }
            } else if lookahead.peek(Ident) {
                let ident = inner.parse::<Ident>()?;
                match ident {
                    id if id == "none" => quote! { None },
                    id if id == "inner" => quote! { Inner },
                    id if id == "tiny" => quote! { Tiny },
                    id if id == "small" => quote! { Small },
                    id if id == "large" => quote! { Large },
                    id if id == "text" => quote! { Text },
                    _ => {
                        return Err(Error::new(
                            ident.span(),
                            "expected one of: `none`, `inner`, `tiny`, `small`, `large`, `text` or a numeric value",
                        ))
                    }
                }
            } else {
                return Err(lookahead.error());
            };

            let _ = input.parse::<Token![:]>()?;
            let layout = Layout::parse(input, gen)?;
            Ok(Layout::Margins(Box::new(layout), dirs, margins))
        } else if lookahead.peek(Token![self]) {
            Ok(Layout::Single(input.parse()?))
        } else if lookahead.peek(kw::frame) {
            let _: kw::frame = input.parse()?;
            let style: Expr = if input.peek(syn::token::Paren) {
                let inner;
                let _ = parenthesized!(inner in input);
                inner.parse()?
            } else {
                syn::parse_quote! { ::kas::theme::FrameStyle::Frame }
            };
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
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let dir = Direction::Down;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let dir = Direction::Right;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::list) {
            let _: kw::list = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let _: Token![,] = inner.parse()?;
            let list = parse_layout_list(&inner, gen)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::float) {
            let _: kw::float = input.parse()?;
            let _: Token![!] = input.parse()?;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::Float(list))
        } else if lookahead.peek(kw::aligned_column) {
            let _: kw::aligned_column = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            Ok(parse_grid_as_list_of_lists::<kw::row>(
                stor, input, gen, true,
            )?)
        } else if lookahead.peek(kw::aligned_row) {
            let _: kw::aligned_row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            Ok(parse_grid_as_list_of_lists::<kw::column>(
                stor, input, gen, false,
            )?)
        } else if lookahead.peek(kw::slice) {
            let _: kw::slice = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let _: Token![,] = inner.parse()?;
            if inner.peek(Token![self]) {
                Ok(Layout::Slice(stor, dir, inner.parse()?))
            } else {
                Err(Error::new(inner.span(), "expected `self`"))
            }
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let _: Token![:] = input.parse()?;
            Ok(parse_grid(stor, input, gen)?)
        } else if lookahead.peek(kw::non_navigable) {
            let _: kw::non_navigable = input.parse()?;
            let _: Token![:] = input.parse()?;
            let layout = Layout::parse(input, gen)?;
            Ok(Layout::NonNavigable(Box::new(layout)))
        } else if lookahead.peek(LitStr) {
            let stor = gen.next();
            Ok(Layout::Label(stor, input.parse()?))
        } else {
            if let Ok(ident) = input.fork().parse::<Ident>() {
                if ident
                    .to_string()
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false)
                {
                    let stor = gen.next();
                    let expr = input.parse()?;
                    return Ok(Layout::Widget(stor, expr));
                }
            }
            Err(lookahead.error())
        }
    }
}

impl Align {
    fn parse(inner: ParseStream, first: bool) -> Result<Option<Self>> {
        let lookahead = inner.lookahead1();
        Ok(Some(if lookahead.peek(kw::default) {
            let _: kw::default = inner.parse()?;
            Align::Default
        } else if lookahead.peek(kw::center) {
            let _: kw::center = inner.parse()?;
            Align::Center
        } else if lookahead.peek(kw::stretch) {
            let _: kw::stretch = inner.parse()?;
            Align::Stretch
        } else if lookahead.peek(kw::top) {
            if first {
                return Ok(None);
            }
            let _: kw::top = inner.parse()?;
            Align::TL
        } else if lookahead.peek(kw::bottom) {
            if first {
                return Ok(None);
            }
            let _: kw::bottom = inner.parse()?;
            Align::BR
        } else if lookahead.peek(kw::left) && first {
            let _: kw::left = inner.parse()?;
            Align::TL
        } else if lookahead.peek(kw::right) && first {
            let _: kw::right = inner.parse()?;
            Align::BR
        } else {
            return Err(lookahead.error());
        }))
    }
}

fn parse_align(input: ParseStream) -> Result<AlignHints> {
    let inner;
    let _ = parenthesized!(inner in input);

    match Align::parse(&inner, true)? {
        None => {
            let first = Align::None;
            let second = Align::parse(&inner, false)?.unwrap();
            Ok(AlignHints(first, second))
        }
        Some(first) => {
            let second = if inner.parse::<Token![,]>().is_ok() {
                Align::parse(&inner, false)?.unwrap()
            } else if matches!(first, Align::TL | Align::BR) {
                Align::None
            } else {
                first
            };
            Ok(AlignHints(first, second))
        }
    }
}

fn parse_layout_list(input: ParseStream, gen: &mut NameGenerator) -> Result<Vec<Layout>> {
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

    Ok(list)
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
        let _ = inner.parse::<Token![!]>()?;

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

impl Parse for ExprMember {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ExprMember {
            self_: input.parse()?,
            p: input.parse()?,
            member: input.parse()?,
        })
    }
}

impl ToTokens for ExprMember {
    fn to_tokens(&self, toks: &mut Toks) {
        self.self_.to_tokens(toks);
        self.p.to_tokens(toks);
        self.member.to_tokens(toks);
    }
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

impl ToTokens for Directions {
    fn to_tokens(&self, toks: &mut Toks) {
        let dirs = self.bits();
        toks.append_all(quote! { #dirs })
    }
}

impl ToTokens for AlignHints {
    fn to_tokens(&self, toks: &mut Toks) {
        fn align_toks(align: &Align) -> Toks {
            match align {
                Align::None => quote! { None },
                Align::Default => quote! { Some(layout::Align::Default) },
                Align::Center => quote! { Some(layout::Align::Center) },
                Align::Stretch => quote! { Some(layout::Align::Stretch) },
                Align::TL => quote! { Some(layout::Align::TL) },
                Align::BR => quote! { Some(layout::Align::BR) },
            }
        }
        let horiz = align_toks(&self.0);
        let vert = align_toks(&self.1);

        toks.append_all(quote! {
            layout::AlignHints::new(#horiz, #vert)
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
    fn append_fields(&self, ty_toks: &mut Toks, def_toks: &mut Toks, children: &mut Vec<Toks>) {
        match self {
            Layout::Align(layout, _) | Layout::NonNavigable(layout) => {
                layout.append_fields(ty_toks, def_toks, children);
            }
            Layout::AlignSingle(..) | Layout::Margins(..) | Layout::Single(_) => (),
            Layout::Pack(stor, layout, _) => {
                ty_toks.append_all(quote! { #stor: ::kas::layout::PackStorage, });
                def_toks.append_all(quote! { #stor: Default::default(), });
                layout.append_fields(ty_toks, def_toks, children);
            }
            Layout::Widget(stor, expr) => {
                children.push(stor.to_token_stream());
                ty_toks.append_all(quote! { #stor: Box<dyn ::kas::Widget>, });
                let span = expr.span();
                def_toks.append_all(quote_spanned! {span=> #stor: Box::new(#expr), });
            }
            Layout::Frame(stor, layout, _) | Layout::Button(stor, layout, _) => {
                ty_toks.append_all(quote! { #stor: ::kas::layout::FrameStorage, });
                def_toks.append_all(quote! { #stor: Default::default(), });
                layout.append_fields(ty_toks, def_toks, children);
            }
            Layout::List(stor, _, vec) => {
                def_toks.append_all(quote! { #stor: Default::default(), });

                let len = vec.len();
                ty_toks.append_all(if len > 16 {
                    quote! { #stor: ::kas::layout::DynRowStorage, }
                } else {
                    quote! { #stor: ::kas::layout::FixedRowStorage<#len>, }
                });
                for item in vec {
                    item.append_fields(ty_toks, def_toks, children);
                }
            }
            Layout::Float(vec) => {
                for item in vec {
                    item.append_fields(ty_toks, def_toks, children);
                }
            }
            Layout::Slice(stor, _, _) => {
                ty_toks.append_all(quote! { #stor: ::kas::layout::DynRowStorage, });
                def_toks.append_all(quote! { #stor: Default::default(), });
            }
            Layout::Grid(stor, dim, cells) => {
                let (cols, rows) = (dim.cols as usize, dim.rows as usize);
                ty_toks
                    .append_all(quote! { #stor: ::kas::layout::FixedGridStorage<#cols, #rows>, });
                def_toks.append_all(quote! { #stor: Default::default(), });

                for (_info, layout) in cells {
                    layout.append_fields(ty_toks, def_toks, children);
                }
            }
            Layout::Label(stor, text) => {
                children.push(stor.to_token_stream());
                let span = text.span();
                ty_toks.append_all(quote! { #stor: ::kas::hidden::StrLabel, });
                def_toks.append_all(
                    quote_spanned! {span=> #stor: ::kas::hidden::StrLabel::new(#text), },
                );
            }
        }
    }

    // Optionally pass in the list of children, but not when already in a
    // multi-element layout (list/slice/grid).
    //
    // Required: `::kas::layout` must be in scope.
    fn generate(&self, core: &Member) -> Result<Toks> {
        Ok(match self {
            Layout::Align(layout, align) => {
                let inner = layout.generate(core)?;
                quote! { layout::Visitor::align(#inner, #align) }
            }
            Layout::AlignSingle(expr, align) => {
                quote! { layout::Visitor::align_single(&mut #expr, #align) }
            }
            Layout::Pack(stor, layout, align) => {
                let inner = layout.generate(core)?;
                quote! { layout::Visitor::pack(&mut self.#core.#stor, #inner, #align) }
            }
            Layout::Margins(layout, dirs, selector) => {
                let inner = layout.generate(core)?;
                quote! { layout::Visitor::margins(
                    #inner,
                    ::kas::dir::Directions::from_bits(#dirs).unwrap(),
                    ::kas::theme::MarginStyle::#selector,
                ) }
            }
            Layout::Single(expr) => quote! {
                layout::Visitor::single(&mut #expr)
            },
            Layout::Widget(stor, _) => quote! {
                layout::Visitor::single(&mut self.#core.#stor)
            },
            Layout::Frame(stor, layout, style) => {
                let inner = layout.generate(core)?;
                quote! {
                    layout::Visitor::frame(&mut self.#core.#stor, #inner, #style)
                }
            }
            Layout::Button(stor, layout, color) => {
                let inner = layout.generate(core)?;
                quote! {
                    layout::Visitor::button(&mut self.#core.#stor, #inner, #color)
                }
            }
            Layout::List(stor, dir, list) => {
                let mut items = Toks::new();
                for item in list {
                    let item = item.generate(core)?;
                    items.append_all(quote! {{ #item },});
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };
                quote! {{
                    let dir = #dir;
                    layout::Visitor::list(#iter, dir, &mut self.#core.#stor)
                }}
            }
            Layout::Slice(stor, dir, expr) => {
                quote! {{
                    let dir = #dir;
                    layout::Visitor::slice(&mut #expr, dir, &mut self.#core.#stor)
                }}
            }
            Layout::Grid(stor, dim, cells) => {
                let mut items = Toks::new();
                for item in cells {
                    let (col, col_end) = (item.0.col, item.0.col_end);
                    let (row, row_end) = (item.0.row, item.0.row_end);
                    let layout = item.1.generate(core)?;
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
            Layout::Float(list) => {
                let mut items = Toks::new();
                for item in list {
                    let item = item.generate(core)?;
                    items.append_all(quote! {{ #item },});
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };
                quote! { layout::Visitor::float(#iter) }
            }
            Layout::Label(stor, _) => {
                quote! { layout::Visitor::component(&mut self.#core.#stor) }
            }
            Layout::NonNavigable(layout) => return layout.generate(core),
        })
    }

    /// Create a Vec enumerating all children in navigation order
    ///
    /// -   `output`: the result
    /// -   `index`: the next widget's index
    fn nav_next<'a, I: Clone + Iterator<Item = &'a Member>>(
        &self,
        children: I,
        output: &mut Vec<usize>,
        index: &mut usize,
    ) -> std::result::Result<(), (Span, &'static str)> {
        match self {
            Layout::Align(layout, _)
            | Layout::Pack(_, layout, _)
            | Layout::Margins(layout, _, _)
            | Layout::Frame(_, layout, _) => layout.nav_next(children, output, index),
            Layout::Button(_, layout, _) | Layout::NonNavigable(layout) => {
                // Internals of a button are not navigable, but we still need to increment index
                let start = output.len();
                layout.nav_next(children, output, index)?;
                output.truncate(start);
                Ok(())
            }
            Layout::AlignSingle(m, _) | Layout::Single(m) => {
                for (i, child) in children.enumerate() {
                    if m.member == *child {
                        output.push(i);
                        return Ok(());
                    }
                }
                Err((m.member.span(), "child not found"))
            }
            Layout::Widget(_, _) => {
                output.push(*index);
                *index += 1;
                Ok(())
            }
            Layout::List(_, dir, list) => {
                let start = output.len();
                for item in list {
                    item.nav_next(children.clone(), output, index)?;
                }
                match dir {
                    _ if output.len() <= start + 1 => Ok(()),
                    Direction::Right | Direction::Down => Ok(()),
                    Direction::Left | Direction::Up => Ok(output[start..].reverse()),
                    Direction::Expr(_) => Err((dir.span(), "`list(dir)` with non-static `dir`")),
                }
            }
            Layout::Slice(_, _, expr) => {
                Err((expr.span(), "`slice` combined with other layout components"))
            }
            Layout::Grid(_, _, cells) => {
                // TODO: sort using CellInfo?
                for (_, item) in cells {
                    item.nav_next(children.clone(), output, index)?;
                }
                Ok(())
            }
            Layout::Float(list) => {
                for item in list {
                    item.nav_next(children.clone(), output, index)?;
                }
                Ok(())
            }
            Layout::Label(_, _) => {
                *index += 1;
                Ok(())
            }
        }
    }

    fn span_in_layout(&self, ident: &Member) -> Option<Span> {
        match self {
            Layout::Align(layout, _)
            | Layout::Pack(_, layout, _)
            | Layout::Margins(layout, _, _)
            | Layout::Frame(_, layout, _)
            | Layout::Button(_, layout, _)
            | Layout::NonNavigable(layout) => layout.span_in_layout(ident),
            Layout::AlignSingle(expr, _) | Layout::Single(expr) => {
                (expr.member == *ident).then(|| expr.span())
            }
            Layout::Widget(..) => None,
            Layout::List(_, _, list) | Layout::Float(list) => {
                list.iter().find_map(|layout| layout.span_in_layout(ident))
            }
            Layout::Slice(..) => None,
            Layout::Grid(_, _, list) => list.iter().find_map(|cell| cell.1.span_in_layout(ident)),
            Layout::Label(..) => None,
        }
    }
}
