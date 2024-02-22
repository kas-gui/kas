// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::collection::{NameGenerator, StorIdent};
use crate::widget::{self, Child, ChildIdent};
use proc_macro2::{Span, TokenStream as Toks};
use proc_macro_error::emit_error;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{braced, bracketed, parenthesized, parse_quote, token};
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
    custom_keyword!(non_navigable);
    custom_keyword!(px);
    custom_keyword!(em);
    custom_keyword!(style);
    custom_keyword!(color);
    custom_keyword!(map_any);
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
    pub fn storage_fields(&self, children: &mut Vec<Child>, data_ty: &Type) -> StorageFields {
        let mut fields = StorageFields::default();
        self.0.append_fields(&mut fields, children, data_ty);
        fields
    }

    // Required: `::kas::layout` must be in scope.
    pub fn layout_visitor(&self, core_path: &Toks) -> Result<Toks> {
        self.0.generate(core_path)
    }

    pub fn nav_next<'a, I: Clone + Iterator<Item = &'a Child>>(
        &self,
        children: I,
    ) -> std::result::Result<Toks, (Span, &'static str)> {
        match &self.0 {
            layout => {
                let mut v = Vec::new();
                layout.nav_next(children, &mut v).map(|()| {
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
        let mut children = Vec::new();
        let data_ty: syn::Type = syn::parse_quote! { _Data };
        let stor_defs = self.storage_fields(&mut children, &data_ty);
        let stor_ty = &stor_defs.ty_toks;
        let stor_def = &stor_defs.def_toks;

        let name = Ident::new(widget_name, Span::call_site());
        let core_path = quote! { self };
        let (impl_generics, impl_target) = if stor_defs.used_data_ty {
            (quote! { <_Data> }, quote! { #name <_Data> })
        } else {
            (quote! {}, quote! { #name })
        };

        let count = children.len();
        let num_children = quote! {
            fn num_children(&self) -> usize {
                #count
            }
        };

        let mut get_rules = quote! {};
        for (index, child) in children.iter().enumerate() {
            get_rules.append_all(child.ident.get_rule(&core_path, index));
        }

        let core_impl = widget::impl_core_methods(widget_name, &core_path);
        let widget_impl = widget::impl_widget(
            &impl_generics,
            &impl_target,
            &data_ty,
            &core_path,
            &children,
            true,
        );

        let layout_visitor = self.layout_visitor(&core_path)?;
        let nav_next = match self.nav_next(children.iter()) {
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

            impl #impl_generics ::kas::layout::LayoutVisitor for #impl_target {
                fn layout_visitor(&mut self) -> ::kas::layout::Visitor<impl ::kas::layout::Visitable> {
                    use ::kas::layout;
                    #layout_visitor
                }
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

                fn size_rules(
                    &mut self,
                    sizer: ::kas::theme::SizeCx,
                    axis: ::kas::layout::AxisInfo,
                ) -> ::kas::layout::SizeRules {
                    #[cfg(debug_assertions)]
                    #core_path.status.size_rules(&#core_path.id, axis);
                    ::kas::layout::LayoutVisitor::layout_visitor(self).size_rules(sizer, axis)
                }

                fn set_rect(
                    &mut self,
                    cx: &mut ::kas::event::ConfigCx,
                    rect: ::kas::geom::Rect,
                ) {
                    #[cfg(debug_assertions)]
                    #core_path.status.set_rect(&#core_path.id);

                    #core_path.rect = rect;
                    ::kas::layout::LayoutVisitor::layout_visitor(self).set_rect(cx, rect);
                }

                fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                    use ::kas::{Layout, LayoutExt, layout::LayoutVisitor};
                    #[cfg(debug_assertions)]
                    #core_path.status.require_rect(&#core_path.id);

                    if !self.rect().contains(coord) {
                        return None;
                    }
                    let coord = coord + self.translation();
                    self.layout_visitor()
                        .find_id(coord)
                        .or_else(|| Some(self.id()))
                }

                fn draw(&mut self, draw: ::kas::theme::DrawCx) {
                    #[cfg(debug_assertions)]
                    #core_path.status.require_rect(&#core_path.id);

                    ::kas::layout::LayoutVisitor::layout_visitor(self).draw(draw);
                }

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
        let mut core_gen = NameGenerator::default();
        let stor = core_gen.next();
        let list = parse_layout_items(inner, &mut core_gen, false)?;
        Ok(Tree(Layout::List(stor.into(), Direction::Down, list)))
    }

    /// Parse a row (contents only)
    pub fn row(inner: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        let stor = core_gen.next();
        let list = parse_layout_items(inner, &mut core_gen, false)?;
        Ok(Tree(Layout::List(stor.into(), Direction::Right, list)))
    }

    /// Parse an aligned column (contents only)
    pub fn aligned_column(inner: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        let stor = core_gen.next();
        Ok(Tree(parse_grid_as_list_of_lists::<kw::row>(
            stor.into(),
            inner,
            &mut core_gen,
            true,
            false,
        )?))
    }

    /// Parse an aligned row (contents only)
    pub fn aligned_row(inner: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        let stor = core_gen.next();
        Ok(Tree(parse_grid_as_list_of_lists::<kw::column>(
            stor.into(),
            inner,
            &mut core_gen,
            false,
            false,
        )?))
    }

    /// Parse direction, list
    pub fn list(inner: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        let stor = core_gen.next();
        let dir: Direction = inner.parse()?;
        let _: Token![,] = inner.parse()?;
        let list = parse_layout_list(inner, &mut core_gen, false)?;
        Ok(Tree(Layout::List(stor.into(), dir, list)))
    }

    /// Parse a float (contents only)
    pub fn float(inner: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        let list = parse_layout_items(inner, &mut core_gen, false)?;
        Ok(Tree(Layout::Float(list)))
    }

    /// Parse a grid (contents only)
    pub fn grid(inner: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        let stor = core_gen.next();
        Ok(Tree(parse_grid(stor.into(), inner, &mut core_gen, false)?))
    }
}

#[derive(Debug)]
struct ListItem<C> {
    cell: C,
    stor: Ident,
    layout: Layout,
}
#[derive(Debug)]
struct VisitableList<C>(Vec<ListItem<C>>);
trait GenerateItem: Sized {
    fn cell_info_type() -> Toks;
    fn generate_item(item: &ListItem<Self>, core_path: &Toks) -> Result<Toks>;
}
impl GenerateItem for () {
    fn cell_info_type() -> Toks {
        quote! { () }
    }

    fn generate_item(item: &ListItem<()>, core_path: &Toks) -> Result<Toks> {
        let layout = item.layout.generate(core_path)?;
        Ok(quote! { ((), #layout) })
    }
}
impl GenerateItem for CellInfo {
    fn cell_info_type() -> Toks {
        quote! { ::kas::layout::GridCellInfo }
    }

    fn generate_item(item: &ListItem<CellInfo>, core_path: &Toks) -> Result<Toks> {
        let (col, col_end) = (item.cell.col, item.cell.col_end);
        let (row, row_end) = (item.cell.row, item.cell.row_end);
        let layout = item.layout.generate(core_path)?;
        Ok(quote! {
            (
                layout::GridCellInfo {
                    col: #col,
                    col_end: #col_end,
                    row: #row,
                    row_end: #row_end,
                },
                #layout,
            )
        })
    }
}

#[derive(Debug)]
enum Layout {
    Align(Box<Layout>, Align),
    Pack(Box<Layout>, Pack),
    Single(ExprMember),
    Widget(Ident, Expr),
    Frame(StorIdent, Box<Layout>, Expr),
    Button(StorIdent, Box<Layout>, Expr),
    List(StorIdent, Direction, VisitableList<()>),
    Float(VisitableList<()>),
    Grid(StorIdent, GridDimensions, VisitableList<CellInfo>),
    Label(Ident, LitStr),
    NonNavigable(Box<Layout>),
    MapAny(Box<Layout>, MapAny),
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
        let mut core_gen = NameGenerator::default();
        Ok(Tree(Layout::parse(input, &mut core_gen, true)?))
    }
}

impl Layout {
    fn parse(input: ParseStream, core_gen: &mut NameGenerator, _recurse: bool) -> Result<Self> {
        #[cfg(feature = "recursive-layout-widgets")]
        let _recurse = true;

        #[cfg(not(feature = "recursive-layout-widgets"))]
        if input.peek2(Token![!]) {
            let input2 = input.fork();
            let mut temp_gen = NameGenerator::default();
            if Self::parse_macro_like(&input2, &mut temp_gen).is_ok() {
                loop {
                    if let Ok(dot_token) = input2.parse::<Token![.]>() {
                        if input2.peek(kw::map_any) {
                            let _ = MapAny::parse(dot_token, &input2)?;
                            continue;
                        } else if input2.peek(kw::align) {
                            let _ = Align::parse(dot_token, &input2)?;
                            continue;
                        } else if input2.peek(kw::pack) {
                            let _ = Pack::parse(dot_token, &input2, &mut temp_gen)?;
                            continue;
                        } else if let Ok(ident) = input2.parse::<Ident>() {
                            proc_macro_error::emit_warning!(
                                ident, "this method call is incompatible with feature `recursive-layout-widgets`";
                                note = "extract operand from layout expression or wrap with braces",
                            );
                        }
                    }

                    break;
                }
            }
        }

        let mut layout = if _recurse && input.peek2(Token![!]) {
            Self::parse_macro_like(input, core_gen)?
        } else if input.peek(Token![self]) {
            Layout::Single(input.parse()?)
        } else if input.peek(LitStr) {
            let ident = core_gen.next();
            Layout::Label(ident, input.parse()?)
        } else {
            let ident = core_gen.next();
            let expr = input.parse()?;
            return Ok(Layout::Widget(ident, expr));
        };

        loop {
            if let Ok(dot_token) = input.parse::<Token![.]>() {
                if input.peek(kw::map_any) {
                    let map_any = MapAny::parse(dot_token, input)?;
                    layout = Layout::MapAny(Box::new(layout), map_any);
                } else if input.peek(kw::align) {
                    let align = Align::parse(dot_token, input)?;
                    layout = Layout::Align(Box::new(layout), align);
                } else if input.peek(kw::pack) {
                    let pack = Pack::parse(dot_token, input, core_gen)?;
                    layout = Layout::Pack(Box::new(layout), pack);
                } else if let Ok(ident) = input.parse::<Ident>() {
                    emit_error!(
                        ident, "method not supported here";
                        note = "supported methods on layout objects: `map_any`, `align`, `pack`",
                    );

                    // Clear remainder of input stream to avoid a redundant error
                    let _ = input.step(|cursor| {
                        let mut rest = *cursor;
                        while let Some((_, next)) = rest.token_tree() {
                            rest = next;
                        }
                        Ok(((), rest))
                    })?;

                    // Continue with macro expansion to minimise secondary errors
                    return Ok(layout);
                } else {
                    // syn will report error due to unparsed tokens
                    return Ok(layout);
                }

                continue;
            }

            return Ok(layout);
        }
    }

    fn parse_macro_like(input: ParseStream, core_gen: &mut NameGenerator) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::frame) {
            let _: kw::frame = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.parse_or_next(input)?;

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, core_gen, true)?;

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
            let stor = core_gen.parse_or_next(input)?;

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, core_gen, true)?;

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
            let stor = core_gen.parse_or_next(input)?;
            let list = parse_layout_list(input, core_gen, true)?;
            Ok(Layout::List(stor, Direction::Down, list))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.parse_or_next(input)?;
            let list = parse_layout_list(input, core_gen, true)?;
            Ok(Layout::List(stor, Direction::Right, list))
        } else if lookahead.peek(kw::list) {
            let _: kw::list = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.parse_or_next(input)?;
            let inner;
            let _ = parenthesized!(inner in input);
            let dir: Direction = inner.parse()?;
            let _: Token![,] = inner.parse()?;
            let list = parse_layout_list(&inner, core_gen, true)?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::float) {
            let _: kw::float = input.parse()?;
            let _: Token![!] = input.parse()?;
            let list = parse_layout_list(input, core_gen, true)?;
            Ok(Layout::Float(list))
        } else if lookahead.peek(kw::aligned_column) {
            let _: kw::aligned_column = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.parse_or_next(input)?;

            let inner;
            let _ = bracketed!(inner in input);
            Ok(parse_grid_as_list_of_lists::<kw::row>(
                stor, &inner, core_gen, true, true,
            )?)
        } else if lookahead.peek(kw::aligned_row) {
            let _: kw::aligned_row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.parse_or_next(input)?;

            let inner;
            let _ = bracketed!(inner in input);
            Ok(parse_grid_as_list_of_lists::<kw::column>(
                stor, &inner, core_gen, false, true,
            )?)
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.parse_or_next(input)?;

            let inner;
            let _ = braced!(inner in input);
            Ok(parse_grid(stor, &inner, core_gen, true)?)
        } else if lookahead.peek(kw::non_navigable) {
            let _: kw::non_navigable = input.parse()?;
            let _: Token![!] = input.parse()?;

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, core_gen, true)?;
            Ok(Layout::NonNavigable(Box::new(layout)))
        } else {
            let ident = core_gen.next();
            let expr = input.parse()?;
            Ok(Layout::Widget(ident, expr))
        }
    }
}

fn parse_layout_list(
    input: ParseStream,
    core_gen: &mut NameGenerator,
    recurse: bool,
) -> Result<VisitableList<()>> {
    let inner;
    let _ = bracketed!(inner in input);
    parse_layout_items(&inner, core_gen, recurse)
}

fn parse_layout_items(
    inner: ParseStream,
    core_gen: &mut NameGenerator,
    recurse: bool,
) -> Result<VisitableList<()>> {
    let mut list = vec![];
    let mut gen2 = NameGenerator::default();
    while !inner.is_empty() {
        list.push(ListItem {
            cell: (),
            stor: gen2.next(),
            layout: Layout::parse(inner, core_gen, recurse)?,
        });

        if inner.is_empty() {
            break;
        }

        let _: Token![,] = inner.parse()?;
    }

    Ok(VisitableList(list))
}

fn parse_grid_as_list_of_lists<KW: Parse>(
    stor: StorIdent,
    inner: ParseStream,
    core_gen: &mut NameGenerator,
    row_major: bool,
    recurse: bool,
) -> Result<Layout> {
    let (mut col, mut row) = (0, 0);
    let mut dim = GridDimensions::default();
    let mut gen2 = NameGenerator::default();
    let mut cells = vec![];

    while !inner.is_empty() {
        let _ = inner.parse::<KW>()?;
        let _ = inner.parse::<Token![!]>()?;

        let inner2;
        let _ = bracketed!(inner2 in inner);

        while !inner2.is_empty() {
            let cell = CellInfo::new(col, row);
            dim.update(&cell);
            let layout = Layout::parse(&inner2, core_gen, recurse)?;
            cells.push(ListItem {
                cell,
                stor: gen2.next(),
                layout,
            });

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

    Ok(Layout::Grid(stor, dim, VisitableList(cells)))
}

fn parse_grid(
    stor: StorIdent,
    inner: ParseStream,
    core_gen: &mut NameGenerator,
    recurse: bool,
) -> Result<Layout> {
    let mut dim = GridDimensions::default();
    let mut gen2 = NameGenerator::default();
    let mut cells = vec![];
    while !inner.is_empty() {
        let cell = parse_cell_info(inner)?;
        dim.update(&cell);
        let _: Token![=>] = inner.parse()?;

        let layout;
        let require_comma;
        if inner.peek(syn::token::Brace) {
            let inner2;
            let _ = braced!(inner2 in inner);
            layout = Layout::parse(&inner2, core_gen, recurse)?;
            require_comma = false;
        } else {
            layout = Layout::parse(inner, core_gen, recurse)?;
            require_comma = true;
        }
        cells.push(ListItem {
            cell,
            stor: gen2.next(),
            layout,
        });

        if inner.is_empty() {
            break;
        }

        if let Err(e) = inner.parse::<Token![,]>() {
            if require_comma {
                return Err(e);
            }
        }
    }

    Ok(Layout::Grid(stor, dim, VisitableList(cells)))
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

#[derive(Debug)]
#[allow(unused)]
struct MapAny {
    pub dot_token: Token![.],
    pub kw: kw::map_any,
    pub paren_token: token::Paren,
}
impl MapAny {
    fn parse(dot_token: Token![.], input: ParseStream) -> Result<Self> {
        let _content;
        Ok(MapAny {
            dot_token,
            kw: input.parse()?,
            paren_token: parenthesized!(_content in input),
        })
    }
}

#[derive(Debug)]
#[allow(unused)]
struct Align {
    pub dot_token: Token![.],
    pub kw: kw::align,
    pub paren_token: token::Paren,
    pub hints: Expr,
}
impl Align {
    fn parse(dot_token: Token![.], input: ParseStream) -> Result<Self> {
        let kw = input.parse::<kw::align>()?;
        let content;
        let paren_token = parenthesized!(content in input);
        Ok(Align {
            dot_token,
            kw,
            paren_token,
            hints: content.parse()?,
        })
    }
}
impl ToTokens for Align {
    fn to_tokens(&self, tokens: &mut Toks) {
        self.dot_token.to_tokens(tokens);
        self.kw.to_tokens(tokens);
        self.paren_token.surround(tokens, |tokens| {
            self.hints.to_tokens(tokens);
        });
    }
}

#[derive(Debug)]
#[allow(unused)]
struct Pack {
    pub dot_token: Token![.],
    pub kw: kw::pack,
    pub paren_token: token::Paren,
    pub hints: Expr,
    pub stor: StorIdent,
}
impl Pack {
    fn parse(
        dot_token: Token![.],
        input: ParseStream,
        core_gen: &mut NameGenerator,
    ) -> Result<Self> {
        let kw = input.parse::<kw::pack>()?;
        let content;
        let paren_token = parenthesized!(content in input);
        Ok(Pack {
            dot_token,
            kw,
            paren_token,
            hints: content.parse()?,
            stor: core_gen.next().into(),
        })
    }

    fn to_tokens(&self, tokens: &mut Toks, core_path: &Toks) {
        self.dot_token.to_tokens(tokens);
        self.kw.to_tokens(tokens);
        self.paren_token.surround(tokens, |tokens| {
            self.hints.to_tokens(tokens);
            tokens.append_all(quote! { , &mut #core_path . });
            self.stor.to_tokens(tokens);
        });
    }
}

impl Layout {
    fn append_fields(&self, fields: &mut StorageFields, children: &mut Vec<Child>, data_ty: &Type) {
        match self {
            Layout::Align(layout, _) | Layout::NonNavigable(layout) => {
                layout.append_fields(fields, children, data_ty);
            }
            Layout::Single(_) => (),
            Layout::Pack(layout, pack) => {
                let stor = &pack.stor;
                fields
                    .ty_toks
                    .append_all(quote! { #stor: ::kas::layout::PackStorage, });
                fields
                    .def_toks
                    .append_all(quote! { #stor: Default::default(), });
                layout.append_fields(fields, children, data_ty);
            }
            Layout::Widget(ident, expr) => {
                children.push(Child::new_core(ident.clone().into()));
                fields
                    .ty_toks
                    .append_all(quote! { #ident: Box<dyn ::kas::Widget<Data = #data_ty>>, });
                let span = expr.span();
                fields
                    .def_toks
                    .append_all(quote_spanned! {span=> #ident: Box::new(#expr), });
                fields.used_data_ty = true;
            }
            Layout::Frame(stor, layout, _) | Layout::Button(stor, layout, _) => {
                fields
                    .ty_toks
                    .append_all(quote! { #stor: ::kas::layout::FrameStorage, });
                fields
                    .def_toks
                    .append_all(quote! { #stor: Default::default(), });
                layout.append_fields(fields, children, data_ty);
            }
            Layout::List(stor, _, VisitableList(list)) => {
                fields
                    .def_toks
                    .append_all(quote! { #stor: Default::default(), });

                let len = list.len();
                fields.ty_toks.append_all(if len > 16 {
                    quote! { #stor: ::kas::layout::DynRowStorage, }
                } else {
                    quote! { #stor: ::kas::layout::FixedRowStorage<#len>, }
                });
                for item in list {
                    item.layout.append_fields(fields, children, data_ty);
                }
            }
            Layout::Float(VisitableList(list)) => {
                for item in list {
                    item.layout.append_fields(fields, children, data_ty);
                }
            }
            Layout::Grid(stor, dim, VisitableList(list)) => {
                let (cols, rows) = (dim.cols as usize, dim.rows as usize);
                fields
                    .ty_toks
                    .append_all(quote! { #stor: ::kas::layout::FixedGridStorage<#cols, #rows>, });
                fields
                    .def_toks
                    .append_all(quote! { #stor: Default::default(), });

                for item in list {
                    item.layout.append_fields(fields, children, data_ty);
                }
            }
            Layout::Label(ident, text) => {
                children.push(Child::new_core(ident.clone().into()));
                let span = text.span();
                if *data_ty == syn::parse_quote! { () } {
                    fields
                        .ty_toks
                        .append_all(quote! { #ident: ::kas::hidden::StrLabel, });
                    fields.def_toks.append_all(
                        quote_spanned! {span=> #ident: ::kas::hidden::StrLabel::new(#text), },
                    );
                } else {
                    fields.ty_toks.append_all(
                        quote! { #ident: ::kas::hidden::MapAny<#data_ty, ::kas::hidden::StrLabel>, },
                    );
                    fields.def_toks.append_all(
                        quote_spanned! {span=> #ident: ::kas::hidden::MapAny::new(::kas::hidden::StrLabel::new(#text)), },
                    );
                    fields.used_data_ty = true;
                }
            }
            Layout::MapAny(layout, map_any) => {
                let start = children.len();
                layout.append_fields(fields, children, &parse_quote! { () });
                let map_expr: Expr = parse_quote! { &() };
                for child in &mut children[start..] {
                    if let Some(ref expr) = child.data_binding {
                        if *expr != map_expr {
                            emit_error!(map_any.kw, "invalid data type mapping")
                        }
                    } else {
                        child.data_binding = Some(map_expr.clone());
                    }
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
                let mut tokens = layout.generate(core_path)?;
                align.to_tokens(&mut tokens);
                tokens
            }
            Layout::Pack(layout, pack) => {
                let mut tokens = layout.generate(core_path)?;
                pack.to_tokens(&mut tokens, core_path);
                tokens
            }
            Layout::Single(expr) => quote! {
                layout::Visitor::single(&mut #expr)
            },
            Layout::Widget(ident, _) => quote! {
                layout::Visitor::single(&mut #core_path.#ident)
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
                let list = list.expand(core_path)?;
                quote! {{
                    let dir = #dir;
                    layout::Visitor::list(#list, dir, &mut #core_path.#stor)
                }}
            }
            Layout::Grid(stor, dim, list) => {
                let list = list.expand(core_path)?;
                quote! { layout::Visitor::grid(#list, #dim, &mut #core_path.#stor) }
            }
            Layout::Float(list) => {
                let list = list.expand(core_path)?;
                quote! { layout::Visitor::float(#list) }
            }
            Layout::Label(stor, _) => {
                quote! { layout::Visitor::single(&mut #core_path.#stor) }
            }
            Layout::NonNavigable(layout) | Layout::MapAny(layout, _) => {
                return layout.generate(core_path)
            }
        })
    }

    /// Create a Vec enumerating all children in navigation order
    ///
    /// -   `output`: the result
    /// -   `index`: the next widget's index
    fn nav_next<'a, I: Clone + Iterator<Item = &'a Child>>(
        &self,
        children: I,
        output: &mut Vec<usize>,
    ) -> std::result::Result<(), (Span, &'static str)> {
        match self {
            Layout::Align(layout, _)
            | Layout::Pack(layout, _)
            | Layout::Frame(_, layout, _)
            | Layout::MapAny(layout, _) => layout.nav_next(children, output),
            Layout::Button(_, _, _) | Layout::NonNavigable(_) => {
                // Internals of a button are not navigable
                Ok(())
            }
            Layout::Single(m) => {
                for (i, child) in children.enumerate() {
                    if let ChildIdent::Field(ref ident) = child.ident {
                        if m.member == *ident {
                            output.push(i);
                            return Ok(());
                        }
                    }
                }
                Err((m.member.span(), "child not found"))
            }
            Layout::Widget(ident, _) => {
                for (i, child) in children.enumerate() {
                    if let ChildIdent::CoreField(ref child_ident) = child.ident {
                        if let Member::Named(ref ci) = child_ident {
                            if *ident == *ci {
                                output.push(i);
                                return Ok(());
                            }
                        }
                    }
                }
                panic!("generated child not found")
            }
            Layout::List(_, dir, VisitableList(list)) => {
                let start = output.len();
                for item in list {
                    item.layout.nav_next(children.clone(), output)?;
                }
                match dir {
                    _ if output.len() <= start + 1 => Ok(()),
                    Direction::Right | Direction::Down => Ok(()),
                    Direction::Left | Direction::Up => Ok(output[start..].reverse()),
                    Direction::Expr(_) => Err((dir.span(), "`list(dir)` with non-static `dir`")),
                }
            }
            Layout::Grid(_, _, VisitableList(list)) => {
                // TODO: sort using CellInfo?
                for item in list {
                    item.layout.nav_next(children.clone(), output)?;
                }
                Ok(())
            }
            Layout::Float(VisitableList(list)) => {
                for item in list {
                    item.layout.nav_next(children.clone(), output)?;
                }
                Ok(())
            }
            Layout::Label(_, _) => Ok(()),
        }
    }

    fn span_in_layout(&self, ident: &Member) -> Option<Span> {
        match self {
            Layout::Align(layout, _)
            | Layout::Pack(layout, _)
            | Layout::Frame(_, layout, _)
            | Layout::Button(_, layout, _)
            | Layout::NonNavigable(layout)
            | Layout::MapAny(layout, _) => layout.span_in_layout(ident),
            Layout::Single(expr) => (expr.member == *ident).then(|| expr.span()),
            Layout::Widget(..) => None,
            Layout::List(_, _, VisitableList(list)) | Layout::Float(VisitableList(list)) => list
                .iter()
                .find_map(|item| item.layout.span_in_layout(ident)),
            Layout::Grid(_, _, VisitableList(list)) => list
                .iter()
                .find_map(|cell| cell.layout.span_in_layout(ident)),
            Layout::Label(..) => None,
        }
    }
}

impl<C: GenerateItem> VisitableList<C> {
    pub fn expand(&self, core_path: &Toks) -> Result<Toks> {
        if self.0.is_empty() {
            return Ok(quote! { () });
        }

        let name = Ident::new("_VisitableList", Span::call_site());
        let info_ty = C::cell_info_type();

        let mut item_names = Vec::with_capacity(self.0.len());
        let mut impl_generics = quote! {};
        let mut ty_generics = quote! {};
        let mut stor_ty = quote! {};
        let mut stor_def = quote! {};
        for (index, item) in self.0.iter().enumerate() {
            let span = Span::call_site(); // TODO: span of layout item
            item_names.push(item.stor.to_token_stream());

            let ty = Ident::new(&format!("_L{}", index), span);
            impl_generics.append_all(quote! {
                #ty: ::kas::layout::Visitable,
            });
            ty_generics.append_all(quote! { #ty, });

            let stor = &item.stor;
            stor_ty.append_all(quote! { #stor: (#info_ty, ::kas::layout::Visitor<#ty>), });
            let item = GenerateItem::generate_item(item, core_path)?;
            stor_def.append_all(quote_spanned! {span=> #stor: #item, });
        }

        let len = item_names.len();

        let mut get_mut_rules = quote! {};
        for (index, path) in item_names.iter().enumerate() {
            get_mut_rules.append_all(quote! {
                #index => Some((self.#path.0, &mut self.#path.1)),
            });
        }

        let toks = quote! {{
            struct #name <#impl_generics> {
                #stor_ty
            }

            impl<#impl_generics> ::kas::layout::VisitableList<#info_ty> for #name <#ty_generics> {
                fn len(&self) -> usize { #len }

                fn get_info_item(&mut self, index: usize) -> Option<(#info_ty, &mut dyn ::kas::layout::Visitable)> {
                    match index {
                        #get_mut_rules
                        _ => None,
                    }
                }
            }

            #name {
                #stor_def
            }
        }};
        // println!("{}", toks);
        Ok(toks)
    }
}
