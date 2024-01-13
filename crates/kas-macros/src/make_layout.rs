// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::collection::{NameGenerator, StorIdent};
use crate::widget;
use proc_macro2::{Span, TokenStream as Toks};
use proc_macro_error::emit_error;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{braced, bracketed, parenthesized};
use syn::{Expr, Ident, LitInt, LitStr, Member, Token, Type};

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
    custom_keyword!(style);
    custom_keyword!(color);
}

#[derive(Default)]
pub struct StorageFields {
    pub ty_toks: Toks,
    pub def_toks: Toks,
    pub used_data_ty: bool,
}

#[derive(Debug)]
pub struct Tree(Layout);
impl Tree {
    pub fn storage_fields(&self, children: &mut Vec<Toks>, data_ty: &Type) -> StorageFields {
        let (mut ty_toks, mut def_toks) = (Toks::new(), Toks::new());
        let used_data_ty = self
            .0
            .append_fields(&mut ty_toks, &mut def_toks, children, data_ty);
        StorageFields {
            ty_toks,
            def_toks,
            used_data_ty,
        }
    }

    // Required: `::kas::layout` must be in scope.
    pub fn layout_visitor(&self, core_path: &Toks) -> Result<Toks> {
        self.0.generate(core_path)
    }

    // Excludes: fn nav_next
    pub fn layout_methods(&self, core_path: &Toks) -> Result<Toks> {
        let layout = self.0.generate(core_path)?;
        Ok(quote! {
            fn size_rules(
                &mut self,
                sizer: ::kas::theme::SizeCx,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                use ::kas::{Layout, layout};
                #[cfg(debug_assertions)]
                #core_path.status.size_rules(&#core_path.id, axis);

                (#layout).size_rules(sizer, axis)
            }

            fn set_rect(
                &mut self,
                cx: &mut ::kas::event::ConfigCx,
                rect: ::kas::geom::Rect,
            ) {
                use ::kas::{Layout, layout};
                #[cfg(debug_assertions)]
                #core_path.status.set_rect(&#core_path.id);

                #core_path.rect = rect;
                (#layout).set_rect(cx, rect);
            }

            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                use ::kas::{layout, Layout, LayoutExt};
                #[cfg(debug_assertions)]
                #core_path.status.require_rect(&#core_path.id);

                if !self.rect().contains(coord) {
                    return None;
                }
                let coord = coord + self.translation();
                (#layout).find_id(coord).or_else(|| Some(self.id()))
            }

            fn draw(&mut self, draw: ::kas::theme::DrawCx) {
                use ::kas::{Layout, layout};
                #[cfg(debug_assertions)]
                #core_path.status.require_rect(&#core_path.id);

                (#layout).draw(draw);
            }
        })
    }

    pub fn nav_next<'a, I: Clone + ExactSizeIterator<Item = &'a Member>>(
        &self,
        children: I,
    ) -> std::result::Result<Toks, (Span, &'static str)> {
        match &self.0 {
            layout => {
                let mut v = Vec::new();
                let mut index = children.len();
                layout.nav_next(children, &mut v, &mut index).map(|()| {
                    quote! {
                        fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
                            let mut iter = [#(#v),*].into_iter();
                            if !reverse {
                                if let Some(wi) = from {
                                    let _ = iter.find(|x| *x == wi);
                                }
                                iter.next()
                            } else {
                                let mut iter = iter.rev();
                                if let Some(wi) = from {
                                    let _ = iter.find(|x| *x == wi);
                                }
                                iter.next()
                            }
                        }
                    }
                })
            }
        }
    }

    /// If field `ident` is included in the layout, return Span of usage
    pub fn span_in_layout(&self, ident: &Member) -> Option<Span> {
        self.0.span_in_layout(ident)
    }

    /// Synthesize an entire widget from the layout
    pub fn expand_as_widget(self, widget_name: &str) -> Result<Toks> {
        let mut layout_children = Vec::new();
        let data_ty: syn::Type = syn::parse_quote! { _Data };
        let stor_defs = self.storage_fields(&mut layout_children, &data_ty);
        let stor_ty = &stor_defs.ty_toks;
        let stor_def = &stor_defs.def_toks;

        let name = Ident::new(widget_name, Span::call_site());
        let core_path = quote! { self };
        let (impl_generics, impl_target) = if stor_defs.used_data_ty {
            (quote! { <_Data> }, quote! { #name <_Data> })
        } else {
            (quote! {}, quote! { #name })
        };

        let count = layout_children.len();
        let num_children = quote! {
            fn num_children(&self) -> usize {
                #count
            }
        };

        let mut get_rules = quote! {};
        for (index, path) in layout_children.iter().enumerate() {
            get_rules.append_all(quote! {
                #index => Some(#core_path.#path.as_layout()),
            });
        }

        let core_impl = widget::impl_core_methods(widget_name, &core_path);
        let widget_impl = widget::impl_widget(
            &impl_generics,
            &impl_target,
            &data_ty,
            &core_path,
            &[],
            layout_children,
            true,
        );

        let layout_methods = self.layout_methods(&core_path)?;
        let nav_next = match self.nav_next(std::iter::empty()) {
            Ok(result) => Some(result),
            Err((span, msg)) => {
                emit_error!(span, "unable to generate `fn Layout::nav_next`: {}", msg);
                None
            }
        };

        let toks = quote! {{
            struct #name #impl_generics {
                rect: ::kas::geom::Rect,
                id: ::kas::Id,
                #[cfg(debug_assertions)]
                status: ::kas::WidgetStatus,
                #stor_ty
            }

            impl #impl_generics ::kas::Layout for #impl_target {
                #core_impl
                #num_children
                fn get_child(&self, index: usize) -> Option<&dyn ::kas::Layout> {
                    use ::kas::Layout;
                    match index {
                        #get_rules
                        _ => None,
                    }
                }

                #layout_methods
                #nav_next
            }

            impl #impl_generics ::kas::Events for #impl_target {
                fn steal_event(
                    &mut self,
                    _: &mut ::kas::event::EventCx,
                    _: &Self::Data,
                    _: &::kas::Id,
                    _: &::kas::event::Event,
                ) -> ::kas::event::IsUsed {
                    #[cfg(debug_assertions)]
                    #core_path.status.require_rect(&#core_path.id);
                    ::kas::event::Unused
                }

                fn handle_event(
                    &mut self,
                    _: &mut ::kas::event::EventCx,
                    _: &Self::Data,
                    _: ::kas::event::Event,
                ) -> ::kas::event::IsUsed {
                    #[cfg(debug_assertions)]
                    #core_path.status.require_rect(&#core_path.id);
                    ::kas::event::Unused
                }
            }

            #widget_impl

            #name {
                rect: Default::default(),
                id: Default::default(),
                #[cfg(debug_assertions)]
                status: Default::default(),
                #stor_def
            }
        }};
        // println!("{}", toks);
        Ok(toks)
    }

    /// Parse a column (contents only)
    pub fn column(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();
        let list = parse_layout_items(inner, &mut gen)?;
        Ok(Tree(Layout::List(stor, Direction::Down, list)))
    }

    /// Parse a row (contents only)
    pub fn row(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();
        let list = parse_layout_items(inner, &mut gen)?;
        Ok(Tree(Layout::List(stor, Direction::Right, list)))
    }

    /// Parse an aligned column (contents only)
    pub fn aligned_column(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();
        Ok(Tree(parse_grid_as_list_of_lists::<kw::row>(
            stor, inner, &mut gen, true,
        )?))
    }

    /// Parse an aligned row (contents only)
    pub fn aligned_row(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();
        Ok(Tree(parse_grid_as_list_of_lists::<kw::column>(
            stor, inner, &mut gen, false,
        )?))
    }

    /// Parse direction, list
    pub fn list(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();
        let dir: Direction = inner.parse()?;
        let _: Token![,] = inner.parse()?;
        let list = parse_layout_list(inner, &mut gen)?;
        Ok(Tree(Layout::List(stor, dir, list)))
    }

    /// Parse a float (contents only)
    pub fn float(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let list = parse_layout_items(inner, &mut gen)?;
        Ok(Tree(Layout::Float(list)))
    }

    /// Parse a grid (contents only)
    pub fn grid(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();
        Ok(Tree(parse_grid(stor, inner, &mut gen)?))
    }

    /// Parse align (contents only)
    // TODO: use WithAlign adapter?
    pub fn align(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();

        let align = parse_align(inner)?;
        let _: Token![,] = inner.parse()?;

        Ok(Tree(if inner.peek(Token![self]) {
            Layout::AlignSingle(inner.parse()?, align)
        } else {
            let layout = Layout::parse(inner, &mut gen)?;
            Layout::Align(Box::new(layout), align)
        }))
    }

    /// Parse pack (contents only)
    pub fn pack(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        let stor = gen.next();

        let align = parse_align(inner)?;
        let _: Token![,] = inner.parse()?;

        let layout = Layout::parse(inner, &mut gen)?;
        Ok(Tree(Layout::Pack(stor, Box::new(layout), align)))
    }

    /// Parse margins (contents only)
    pub fn margins(inner: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        Layout::margins_inner(inner, &mut gen).map(Tree)
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
    #[derive(Debug)]
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

impl Parse for Tree {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut gen = NameGenerator::default();
        Ok(Tree(Layout::parse(input, &mut gen)?))
    }
}

impl Layout {
    fn parse(input: ParseStream, gen: &mut NameGenerator) -> Result<Self> {
        if input.peek2(Token![!]) {
            Self::parse_macro_like(input, gen)
        } else if input.peek(Token![self]) {
            Ok(Layout::Single(input.parse()?))
        } else if input.peek(LitStr) {
            let stor = gen.next();
            Ok(Layout::Label(stor, input.parse()?))
        } else {
            let stor = gen.next();
            let expr = input.parse()?;
            Ok(Layout::Widget(stor, expr))
        }
    }

    fn parse_macro_like(input: ParseStream, gen: &mut NameGenerator) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::align) {
            let _: kw::align = input.parse()?;
            let _: Token![!] = input.parse()?;

            let inner;
            let _ = parenthesized!(inner in input);

            let align = parse_align(&inner)?;
            let _: Token![,] = inner.parse()?;

            if inner.peek(Token![self]) {
                Ok(Layout::AlignSingle(inner.parse()?, align))
            } else {
                let layout = Layout::parse(&inner, gen)?;
                Ok(Layout::Align(Box::new(layout), align))
            }
        } else if lookahead.peek(kw::pack) {
            let _: kw::pack = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;

            let inner;
            let _ = parenthesized!(inner in input);

            let align = parse_align(&inner)?;
            let _: Token![,] = inner.parse()?;

            let layout = Layout::parse(&inner, gen)?;
            Ok(Layout::Pack(stor, Box::new(layout), align))
        } else if lookahead.peek(kw::margins) {
            let _ = input.parse::<kw::margins>()?;
            let _: Token![!] = input.parse()?;

            let inner;
            let _ = parenthesized!(inner in input);
            Self::margins_inner(&inner, gen)
        } else if lookahead.peek(kw::frame) {
            let _: kw::frame = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, gen)?;

            let style: Expr = if !inner.is_empty() {
                let _: Token![,] = inner.parse()?;
                let _: kw::style = inner.parse()?;
                let _: Token![=] = inner.parse()?;
                inner.parse()?
            } else {
                syn::parse_quote! { ::kas::theme::FrameStyle::Frame }
            };

            Ok(Layout::Frame(stor, Box::new(layout), style))
        } else if lookahead.peek(kw::button) {
            let _: kw::button = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, gen)?;

            let color: Expr = if !inner.is_empty() {
                let _: Token![,] = inner.parse()?;
                let _: kw::color = inner.parse()?;
                let _: Token![=] = inner.parse()?;
                inner.parse()?
            } else {
                syn::parse_quote! { None }
            };

            Ok(Layout::Button(stor, Box::new(layout), color))
        } else if lookahead.peek(kw::column) {
            let _: kw::column = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, Direction::Down, list))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;
            let list = parse_layout_list(input, gen)?;
            Ok(Layout::List(stor, Direction::Right, list))
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

            let inner;
            let _ = bracketed!(inner in input);
            Ok(parse_grid_as_list_of_lists::<kw::row>(
                stor, &inner, gen, true,
            )?)
        } else if lookahead.peek(kw::aligned_row) {
            let _: kw::aligned_row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;

            let inner;
            let _ = bracketed!(inner in input);
            Ok(parse_grid_as_list_of_lists::<kw::column>(
                stor, &inner, gen, false,
            )?)
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = gen.parse_or_next(input)?;

            let inner;
            let _ = braced!(inner in input);
            Ok(parse_grid(stor, &inner, gen)?)
        } else if lookahead.peek(kw::non_navigable) {
            let _: kw::non_navigable = input.parse()?;
            let _: Token![!] = input.parse()?;

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, gen)?;
            Ok(Layout::NonNavigable(Box::new(layout)))
        } else {
            let stor = gen.next();
            let expr = input.parse()?;
            Ok(Layout::Widget(stor, expr))
        }
    }

    fn margins_inner(inner: ParseStream, gen: &mut NameGenerator) -> Result<Self> {
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
                _ => return Err(Error::new(
                    ident.span(),
                    "expected one of: horiz, horizontal, vert, vertical, left, right, top, bottom",
                )),
            };
            let _ = inner.parse::<Token![=]>()?;
        }

        let lookahead = inner.lookahead1();
        let margins = if lookahead.peek(syn::LitFloat) {
            let val = inner.parse::<syn::LitFloat>()?;
            let lookahead = inner.lookahead1();
            if lookahead.peek(kw::px) {
                let _ = inner.parse::<kw::px>()?;
                quote! { Px(#val) }
            } else if lookahead.peek(kw::em) {
                let _ = inner.parse::<kw::em>()?;
                quote! { Em(#val) }
            } else {
                return Err(lookahead.error());
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

        let _ = inner.parse::<Token![,]>()?;
        let layout = Layout::parse(inner, gen)?;

        Ok(Layout::Margins(Box::new(layout), dirs, margins))
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

fn parse_align(inner: ParseStream) -> Result<AlignHints> {
    if let Some(first) = Align::parse(inner, true)? {
        let second = if !inner.is_empty() && !inner.peek(Token![,]) {
            Align::parse(inner, false)?.unwrap()
        } else if matches!(first, Align::TL | Align::BR) {
            Align::None
        } else {
            first
        };
        return Ok(AlignHints(first, second));
    }

    let first = Align::None;
    let second = Align::parse(inner, false)?.unwrap();
    Ok(AlignHints(first, second))
}

fn parse_layout_list(input: ParseStream, gen: &mut NameGenerator) -> Result<Vec<Layout>> {
    let inner;
    let _ = bracketed!(inner in input);
    parse_layout_items(&inner, gen)
}

fn parse_layout_items(inner: ParseStream, gen: &mut NameGenerator) -> Result<Vec<Layout>> {
    let mut list = vec![];
    while !inner.is_empty() {
        list.push(Layout::parse(inner, gen)?);

        if inner.is_empty() {
            break;
        }

        let _: Token![,] = inner.parse()?;
    }

    Ok(list)
}

fn parse_grid_as_list_of_lists<KW: Parse>(
    stor: StorIdent,
    inner: ParseStream,
    gen: &mut NameGenerator,
    row_major: bool,
) -> Result<Layout> {
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

fn parse_grid(stor: StorIdent, inner: ParseStream, gen: &mut NameGenerator) -> Result<Layout> {
    let mut dim = GridDimensions::default();
    let mut cells = vec![];
    while !inner.is_empty() {
        let info = parse_cell_info(inner)?;
        dim.update(&info);
        let _: Token![=>] = inner.parse()?;

        let layout;
        let require_comma;
        if inner.peek(syn::token::Brace) {
            let inner2;
            let _ = braced!(inner2 in inner);
            layout = Layout::parse(&inner2, gen)?;
            require_comma = false;
        } else {
            layout = Layout::parse(inner, gen)?;
            require_comma = true;
        }
        cells.push((info, layout));

        if inner.is_empty() {
            break;
        }

        if let Err(e) = inner.parse::<Token![,]>() {
            if require_comma {
                return Err(e);
            }
        }
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
    fn append_fields(
        &self,
        ty_toks: &mut Toks,
        def_toks: &mut Toks,
        children: &mut Vec<Toks>,
        data_ty: &Type,
    ) -> bool {
        match self {
            Layout::Align(layout, _)
            | Layout::Margins(layout, ..)
            | Layout::NonNavigable(layout) => {
                layout.append_fields(ty_toks, def_toks, children, data_ty)
            }
            Layout::AlignSingle(..) | Layout::Single(_) => false,
            Layout::Pack(stor, layout, _) => {
                ty_toks.append_all(quote! { #stor: ::kas::layout::PackStorage, });
                def_toks.append_all(quote! { #stor: Default::default(), });
                layout.append_fields(ty_toks, def_toks, children, data_ty)
            }
            Layout::Widget(stor, expr) => {
                children.push(stor.to_token_stream());
                ty_toks.append_all(quote! { #stor: Box<dyn ::kas::Widget<Data = #data_ty>>, });
                let span = expr.span();
                def_toks.append_all(quote_spanned! {span=> #stor: Box::new(#expr), });
                true
            }
            Layout::Frame(stor, layout, _) | Layout::Button(stor, layout, _) => {
                ty_toks.append_all(quote! { #stor: ::kas::layout::FrameStorage, });
                def_toks.append_all(quote! { #stor: Default::default(), });
                layout.append_fields(ty_toks, def_toks, children, data_ty)
            }
            Layout::List(stor, _, vec) => {
                def_toks.append_all(quote! { #stor: Default::default(), });

                let len = vec.len();
                ty_toks.append_all(if len > 16 {
                    quote! { #stor: ::kas::layout::DynRowStorage, }
                } else {
                    quote! { #stor: ::kas::layout::FixedRowStorage<#len>, }
                });
                let mut used_data_ty = false;
                for item in vec {
                    used_data_ty |= item.append_fields(ty_toks, def_toks, children, data_ty);
                }
                used_data_ty
            }
            Layout::Float(vec) => {
                let mut used_data_ty = false;
                for item in vec {
                    used_data_ty |= item.append_fields(ty_toks, def_toks, children, data_ty);
                }
                used_data_ty
            }
            Layout::Grid(stor, dim, cells) => {
                let (cols, rows) = (dim.cols as usize, dim.rows as usize);
                ty_toks
                    .append_all(quote! { #stor: ::kas::layout::FixedGridStorage<#cols, #rows>, });
                def_toks.append_all(quote! { #stor: Default::default(), });

                let mut used_data_ty = false;
                for (_info, layout) in cells {
                    used_data_ty |= layout.append_fields(ty_toks, def_toks, children, data_ty);
                }
                used_data_ty
            }
            Layout::Label(stor, text) => {
                children.push(stor.to_token_stream());
                let span = text.span();
                if *data_ty == syn::parse_quote! { () } {
                    ty_toks.append_all(quote! { #stor: ::kas::hidden::StrLabel, });
                    def_toks.append_all(
                        quote_spanned! {span=> #stor: ::kas::hidden::StrLabel::new(#text), },
                    );
                    false
                } else {
                    ty_toks.append_all(
                        quote! { #stor: ::kas::hidden::MapAny<#data_ty, ::kas::hidden::StrLabel>, },
                    );
                    def_toks.append_all(
                        quote_spanned! {span=> #stor: ::kas::hidden::MapAny::new(::kas::hidden::StrLabel::new(#text)), },
                    );
                    true
                }
            }
        }
    }

    // Optionally pass in the list of children, but not when already in a
    // multi-element layout (list/slice/grid).
    //
    // Required: `::kas::layout` must be in scope.
    fn generate(&self, core_path: &Toks) -> Result<Toks> {
        Ok(match self {
            Layout::Align(layout, align) => {
                let inner = layout.generate(core_path)?;
                quote! { layout::Visitor::align(#inner, #align) }
            }
            Layout::AlignSingle(expr, align) => {
                quote! { layout::Visitor::align_single(&mut #expr, #align) }
            }
            Layout::Pack(stor, layout, align) => {
                let inner = layout.generate(core_path)?;
                quote! { layout::Visitor::pack(&mut #core_path.#stor, #inner, #align) }
            }
            Layout::Margins(layout, dirs, selector) => {
                let inner = layout.generate(core_path)?;
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
                layout::Visitor::single(&mut #core_path.#stor)
            },
            Layout::Frame(stor, layout, style) => {
                let inner = layout.generate(core_path)?;
                quote! {
                    layout::Visitor::frame(&mut #core_path.#stor, #inner, #style)
                }
            }
            Layout::Button(stor, layout, color) => {
                let inner = layout.generate(core_path)?;
                quote! {
                    layout::Visitor::button(&mut #core_path.#stor, #inner, #color)
                }
            }
            Layout::List(stor, dir, list) => {
                let mut items = Toks::new();
                for item in list {
                    let item = item.generate(core_path)?;
                    items.append_all(quote! { ::kas::layout::Visitor::boxed(#item), });
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };
                quote! {{
                    let dir = #dir;
                    layout::Visitor::list(#iter, dir, &mut #core_path.#stor)
                }}
            }
            Layout::Grid(stor, dim, cells) => {
                let mut items = Toks::new();
                for item in cells {
                    let (col, col_end) = (item.0.col, item.0.col_end);
                    let (row, row_end) = (item.0.row, item.0.row_end);
                    let layout = item.1.generate(core_path)?;
                    items.append_all(quote! {
                        (
                            layout::GridChildInfo {
                                col: #col,
                                col_end: #col_end,
                                row: #row,
                                row_end: #row_end,
                            },
                            ::kas::layout::Visitor::boxed(#layout),
                        ),
                    });
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };

                quote! { layout::Visitor::grid(#iter, #dim, &mut #core_path.#stor) }
            }
            Layout::Float(list) => {
                let mut items = Toks::new();
                for item in list {
                    let item = item.generate(core_path)?;
                    items.append_all(quote! { ::kas::layout::Visitor::boxed(#item), });
                }
                let iter = quote! { { let arr = [#items]; arr.into_iter() } };
                quote! { layout::Visitor::float(#iter) }
            }
            Layout::Label(stor, _) => {
                quote! { layout::Visitor::single(&mut #core_path.#stor) }
            }
            Layout::NonNavigable(layout) => return layout.generate(core_path),
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
            Layout::Grid(_, _, list) => list.iter().find_map(|cell| cell.1.span_in_layout(ident)),
            Layout::Label(..) => None,
        }
    }
}
