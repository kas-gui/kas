// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::collection::{CellInfo, GridDimensions, NameGenerator};
use crate::widget_args::{Child, ChildIdent};
use proc_macro2::{Span, TokenStream as Toks};
use proc_macro_error2::emit_error;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{braced, bracketed, parenthesized, token};
use syn::{Expr, Ident, LitStr, Member, Token};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(align);
    custom_keyword!(pack);
    custom_keyword!(column);
    custom_keyword!(row);
    custom_keyword!(frame);
    custom_keyword!(list);
    custom_keyword!(grid);
    custom_keyword!(aligned_column);
    custom_keyword!(aligned_row);
    custom_keyword!(float);
    custom_keyword!(with_direction);
    custom_keyword!(with_style);
    custom_keyword!(with_background);
}

#[derive(Default)]
pub struct StorageFields {
    pub ty_toks: Toks,
    pub def_toks: Toks,
}

#[derive(Debug)]
pub struct Tree(Layout);
impl Tree {
    pub fn storage_fields(&self, children: &mut Vec<Child>) -> StorageFields {
        let mut fields = StorageFields::default();
        self.0.append_fields(&mut fields, children);
        fields
    }

    /// Yield an implementation of `fn size_rules`
    pub fn size_rules(&self, core_path: &Toks) -> Toks {
        self.0.size_rules(core_path)
    }

    /// Yield an implementation of `fn set_rect`
    pub fn set_rect(&self, core_path: &Toks) -> Toks {
        self.0.set_rect(core_path)
    }

    /// Yield an implementation of `fn try_probe`
    pub fn try_probe(&self, core_path: &Toks) -> Toks {
        let mut targets = Vec::new();
        self.0.probe_targets(core_path, &mut targets);
        let mut toks = Toks::new();
        for target in &targets {
            toks.append_all(quote! {
                if let Some(id) = ::kas::Layout::try_probe(&#target, coord) {
                    Some(id)
                } else
            });
        }
        toks.append_all(quote! { { None } });
        toks
    }

    /// Yield an implementation of `fn draw`
    pub fn draw(&self, core_path: &Toks) -> Toks {
        self.0.draw(core_path)
    }

    /// Generate implementation of nav_next
    pub fn nav_next<'a, I: Clone + Iterator<Item = &'a Child>>(
        &self,
        children: I,
    ) -> std::result::Result<Toks, (Span, &'static str)> {
        let mut v = Vec::new();
        self.0.nav_next(children, &mut v).map(|()| {
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

#[derive(Debug)]
struct ListItem<C> {
    cell: C,
    layout: Layout,
}

#[derive(Debug)]
enum Layout {
    Align(Box<Layout>, Align),
    Pack(Box<Layout>, Pack),
    Single(ExprMember),
    Widget(Ident, Expr),
    Frame(Ident, Box<Layout>, Expr, Expr),
    List(Ident, Direction, Vec<ListItem<()>>),
    Float(Vec<ListItem<()>>),
    Grid(Ident, GridDimensions, Vec<ListItem<CellInfo>>),
    Label(Ident, LitStr),
}

#[derive(Debug)]
struct ExprMember {
    self_: Token![self],
    p: Token![.],
    member: Member,
}

#[allow(unused)]
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

impl Parse for Tree {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut core_gen = NameGenerator::default();
        Ok(Tree(Layout::parse(input, &mut core_gen)?))
    }
}

impl Layout {
    fn parse(input: ParseStream, core_gen: &mut NameGenerator) -> Result<Self> {
        let mut layout = if input.peek2(Token![!]) {
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
                if input.peek(kw::align) {
                    let align = Align::parse(dot_token, input)?;
                    layout = Layout::Align(Box::new(layout), align);
                } else if input.peek(kw::pack) {
                    let pack = Pack::parse(dot_token, input, core_gen)?;
                    layout = Layout::Pack(Box::new(layout), pack);
                } else if let Ok(ident) = input.parse::<Ident>() {
                    let note_msg = if matches!(&layout, &Layout::Frame(_, _, _, _)) {
                        "supported methods on layout objects: `align`, `pack`, `with_style`, `with_background`"
                    } else {
                        "supported methods on layout objects: `align`, `pack`"
                    };
                    emit_error!(
                        ident, "method not supported here";
                        note = note_msg,
                    );

                    // Clear remainder of input stream to avoid a redundant error
                    let turbofish = if input.peek(Token![::]) {
                        Some(syn::AngleBracketedGenericArguments::parse_turbofish(input)?)
                    } else {
                        None
                    };

                    if turbofish.is_some() || input.peek(syn::token::Paren) {
                        let inner;
                        let _ = parenthesized!(inner in input);
                        let _ = inner.parse_terminated(Expr::parse, Token![,])?;
                    }

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
            let stor = core_gen.next();

            let inner;
            let _ = parenthesized!(inner in input);
            let layout = Layout::parse(&inner, core_gen)?;

            let mut style = None;
            let mut bg = None;
            while input.peek(Token![.]) {
                if style.is_none() && input.peek2(kw::with_style) {
                    let _: Token![.] = input.parse()?;
                    let _: kw::with_style = input.parse()?;

                    let inner;
                    let _ = parenthesized!(inner in input);
                    style = Some(inner.parse()?);
                } else if bg.is_none() && input.peek2(kw::with_background) {
                    let _: Token![.] = input.parse()?;
                    let _: kw::with_background = input.parse()?;

                    let inner;
                    let _ = parenthesized!(inner in input);
                    bg = Some(inner.parse()?);
                } else {
                    break;
                }
            }

            let style =
                style.unwrap_or_else(|| syn::parse_quote! { ::kas::theme::FrameStyle::Frame });
            let bg = bg.unwrap_or_else(|| syn::parse_quote! { ::kas::theme::Background::Default });

            Ok(Layout::Frame(stor, Box::new(layout), style, bg))
        } else if lookahead.peek(kw::column) {
            let _: kw::column = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.next();
            let list = parse_layout_list(input, core_gen)?;
            Ok(Layout::List(stor, Direction::Down, list))
        } else if lookahead.peek(kw::row) {
            let _: kw::row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.next();
            let list = parse_layout_list(input, core_gen)?;
            Ok(Layout::List(stor, Direction::Right, list))
        } else if lookahead.peek(kw::list) {
            let _: kw::list = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.next();
            let list = parse_layout_list(input, core_gen)?;
            let _: Token![.] = input.parse()?;
            let _: kw::with_direction = input.parse()?;
            let args;
            let _ = parenthesized!(args in input);
            let dir: Direction = args.parse()?;
            Ok(Layout::List(stor, dir, list))
        } else if lookahead.peek(kw::float) {
            let _: kw::float = input.parse()?;
            let _: Token![!] = input.parse()?;
            let list = parse_layout_list(input, core_gen)?;
            Ok(Layout::Float(list))
        } else if lookahead.peek(kw::aligned_column) {
            let _: kw::aligned_column = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.next();

            let inner;
            let _ = bracketed!(inner in input);
            Ok(parse_grid_as_list_of_lists::<kw::row>(
                stor, &inner, core_gen, true,
            )?)
        } else if lookahead.peek(kw::aligned_row) {
            let _: kw::aligned_row = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.next();

            let inner;
            let _ = bracketed!(inner in input);
            Ok(parse_grid_as_list_of_lists::<kw::column>(
                stor, &inner, core_gen, false,
            )?)
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = input.parse()?;
            let _: Token![!] = input.parse()?;
            let stor = core_gen.next();

            let inner;
            let _ = braced!(inner in input);
            Ok(parse_grid(stor, &inner, core_gen)?)
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
) -> Result<Vec<ListItem<()>>> {
    let inner;
    let _ = bracketed!(inner in input);
    parse_layout_items(&inner, core_gen)
}

fn parse_layout_items(
    inner: ParseStream,
    core_gen: &mut NameGenerator,
) -> Result<Vec<ListItem<()>>> {
    let mut list = vec![];
    while !inner.is_empty() {
        list.push(ListItem {
            cell: (),
            layout: Layout::parse(inner, core_gen)?,
        });

        if inner.is_empty() {
            break;
        }

        let _: Token![,] = inner.parse()?;
    }

    Ok(list)
}

fn parse_grid_as_list_of_lists<KW: Parse>(
    stor: Ident,
    inner: ParseStream,
    core_gen: &mut NameGenerator,
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
            let cell = CellInfo::new(col, row);
            dim.update(&cell);
            let layout = Layout::parse(&inner2, core_gen)?;
            cells.push(ListItem { cell, layout });

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

fn parse_grid(stor: Ident, inner: ParseStream, core_gen: &mut NameGenerator) -> Result<Layout> {
    let mut dim = GridDimensions::default();
    let mut cells = vec![];
    while !inner.is_empty() {
        let cell = inner.parse()?;
        dim.update(&cell);
        let _: Token![=>] = inner.parse()?;

        let layout;
        let require_comma;
        if inner.peek(syn::token::Brace) {
            let inner2;
            let _ = braced!(inner2 in inner);
            layout = Layout::parse(&inner2, core_gen)?;
            require_comma = false;
        } else {
            layout = Layout::parse(inner, core_gen)?;
            require_comma = true;
        }
        cells.push(ListItem { cell, layout });

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
        Ok(Direction::Expr(input.parse()?))
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
    pub stor: Ident,
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
            stor: core_gen.next(),
        })
    }
}

impl Layout {
    fn append_fields(&self, fields: &mut StorageFields, children: &mut Vec<Child>) {
        match self {
            Layout::Align(layout, _) => {
                layout.append_fields(fields, children);
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
                layout.append_fields(fields, children);
            }
            Layout::Widget(ident, expr) => {
                children.push(Child::new_core(ident.clone().into()));
                fields
                    .ty_toks
                    .append_all(quote! { #ident: Box<dyn ::kas::Widget<Data = ()>>, });
                let span = expr.span();
                fields
                    .def_toks
                    .append_all(quote_spanned! {span=> #ident: Box::new(#expr), });
            }
            Layout::Frame(stor, layout, _, _) => {
                fields
                    .ty_toks
                    .append_all(quote! { #stor: ::kas::layout::FrameStorage, });
                fields
                    .def_toks
                    .append_all(quote! { #stor: Default::default(), });
                layout.append_fields(fields, children);
            }
            Layout::List(stor, _, list) => {
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
                    item.layout.append_fields(fields, children);
                }
            }
            Layout::Float(list) => {
                for item in list {
                    item.layout.append_fields(fields, children);
                }
            }
            Layout::Grid(stor, dim, list) => {
                let (cols, rows) = (dim.cols as usize, dim.rows as usize);
                fields
                    .ty_toks
                    .append_all(quote! { #stor: ::kas::layout::FixedGridStorage<#cols, #rows>, });
                fields
                    .def_toks
                    .append_all(quote! { #stor: Default::default(), });

                for item in list {
                    item.layout.append_fields(fields, children);
                }
            }
            Layout::Label(ident, text) => {
                children.push(Child::new_core(ident.clone().into()));
                let span = text.span();
                fields
                    .ty_toks
                    .append_all(quote! { #ident: ::kas::hidden::StrLabel, });
                fields.def_toks.append_all(
                    quote_spanned! {span=> #ident: ::kas::hidden::StrLabel::new(#text), },
                );
            }
        }
    }

    /// Yield an implementation of `fn size_rules`
    fn size_rules(&self, core_path: &Toks) -> Toks {
        match self {
            Layout::Align(layout, _) => layout.size_rules(core_path),
            Layout::Pack(layout, pack) => {
                let stor = &pack.stor;
                let inner = layout.size_rules(core_path);
                quote! {{
                    let rules = #inner;
                    #core_path.#stor.size.set_component(axis, rules.ideal_size());
                    rules
                }}
            }
            Layout::Single(expr) => quote! {
                ::kas::Layout::size_rules(&mut #expr, sizer, axis)
            },
            Layout::Widget(stor, _) | Layout::Label(stor, _) => quote! {
                ::kas::Layout::size_rules(&mut #core_path.#stor, sizer, axis)
            },
            Layout::Frame(stor, layout, style, _) => {
                let inner = layout.size_rules(core_path);
                quote! {
                    let child_rules = {
                        let sizer = sizer.re();
                        let axis = #core_path.#stor.child_axis(axis);
                        #inner
                    };
                    #core_path.#stor.size_rules(sizer, axis, child_rules, #style)
                }
            }
            Layout::List(stor, dir, list) => {
                let len = list.len();
                let mut toks = quote! {
                    let dim = (#dir, #len);
                    let mut solver = ::kas::layout::RowSolver::new(axis, dim, &mut #core_path.#stor);
                };
                for (index, item) in list.iter().enumerate() {
                    let inner = item.layout.size_rules(core_path);
                    toks.append_all(quote!{{
                        ::kas::layout::RulesSolver::for_child(&mut solver, &mut #core_path.#stor, #index, |axis| {
                            let sizer = sizer.re();
                            #inner
                        });
                    }});
                }
                toks.append_all(quote! {
                    ::kas::layout::RulesSolver::finish(solver, &mut #core_path.#stor)
                });
                toks
            }
            Layout::Float(list) => {
                let mut toks = quote! {
                    let mut rules = SizeRules::EMPTY;
                };
                for item in list {
                    let inner = item.layout.size_rules(core_path);
                    toks.append_all(quote! {
                        rules = rules.max({ #inner });
                    });
                }
                toks.append_all(quote! { rules });
                toks
            }
            Layout::Grid(stor, dim, list) => {
                let mut toks = quote! {
                    let dim = #dim;
                    let mut solver = ::kas::layout::GridSolver::<Vec<_>, Vec<_>, _>::new(axis, dim, &mut #core_path.#stor);
                };
                for item in list {
                    let inner = item.layout.size_rules(core_path);
                    let cell = &item.cell;
                    toks.append_all(quote!{{
                        ::kas::layout::RulesSolver::for_child(&mut solver, &mut #core_path.#stor, #cell, |axis| {
                            let sizer = sizer.re();
                            #inner
                        });
                    }});
                }
                toks.append_all(quote! {
                    ::kas::layout::RulesSolver::finish(solver, &mut #core_path.#stor)
                });
                toks
            }
        }
    }

    /// Yield an implementation of `fn set_rect`
    fn set_rect(&self, core_path: &Toks) -> Toks {
        match self {
            Layout::Align(layout, align) => {
                let align_hints = &align.hints;
                let inner = layout.set_rect(core_path);
                quote! {{
                    let hints = #align_hints.combine(hints);
                    #inner
                }}
            }
            Layout::Pack(layout, pack) => {
                let align_hints = &pack.hints;
                let stor = &pack.stor;
                let inner = layout.set_rect(core_path);
                quote! { {
                    let rect = #align_hints
                        .combine(hints)
                        .complete_default()
                        .aligned_rect(#core_path.#stor.size, rect);
                    #inner
                } }
            }
            Layout::Single(expr) => quote! {
                ::kas::Layout::set_rect(&mut #expr, cx, rect, hints);
            },
            Layout::Widget(stor, _) | Layout::Label(stor, _) => quote! {
                ::kas::Layout::set_rect(&mut #core_path.#stor, cx, rect, hints);
            },
            Layout::Frame(stor, layout, _, _) => {
                let inner = layout.set_rect(core_path);
                quote! {{
                    #core_path.#stor.rect = rect;
                    let rect = ::kas::geom::Rect {
                        pos: rect.pos + #core_path.#stor.offset,
                        size: rect.size - #core_path.#stor.size,
                    };
                    #inner
                }}
            }
            Layout::List(stor, dir, list) => {
                let len = list.len();
                let mut toks = quote! {
                    let dim = (#dir, #len);
                    let mut setter = ::kas::layout::RowSetter::<_, Vec<i32>, _>::new(rect, dim, &mut #core_path.#stor);
                };
                for (index, item) in list.iter().enumerate() {
                    let inner = item.layout.set_rect(core_path);
                    toks.append_all(quote!{{
                        let rect = ::kas::layout::RulesSetter::child_rect(&mut setter, &mut #core_path.#stor, #index);
                        #inner
                    }});
                }
                toks
            }
            Layout::Float(list) => {
                let mut toks = Toks::new();
                for item in list {
                    toks.append_all(item.layout.set_rect(core_path));
                }
                toks
            }
            Layout::Grid(stor, dim, list) => {
                let mut toks = quote! {
                    let dim = #dim;
                    let mut setter = ::kas::layout::GridSetter::<Vec<_>, Vec<_>, _>::new(rect, dim, &mut #core_path.#stor);
                };
                for item in list {
                    let inner = item.layout.set_rect(core_path);
                    let cell = &item.cell;
                    toks.append_all(quote!{{
                        let rect = ::kas::layout::RulesSetter::child_rect(&mut setter, &mut #core_path.#stor, #cell);
                        #inner
                    }});
                }
                toks
            }
        }
    }

    /// Yield an implementation of `fn draw`
    fn probe_targets(&self, core_path: &Toks, targets: &mut Vec<Toks>) {
        match self {
            Layout::Align(layout, _) | Layout::Pack(layout, _) | Layout::Frame(_, layout, _, _) => {
                layout.probe_targets(core_path, targets)
            }
            Layout::Single(expr) => targets.push(expr.to_token_stream()),
            Layout::Widget(stor, _) | Layout::Label(stor, _) => {
                targets.push(quote! { #core_path.#stor })
            }
            Layout::List(_, _, list) | Layout::Float(list) => {
                for item in list {
                    item.layout.probe_targets(core_path, targets);
                }
            }
            Layout::Grid(_, _, list) => {
                for item in list {
                    item.layout.probe_targets(core_path, targets);
                }
            }
        }
    }

    /// Yield an implementation of `fn draw`
    fn draw(&self, core_path: &Toks) -> Toks {
        match self {
            Layout::Align(layout, _) | Layout::Pack(layout, _) => layout.draw(core_path),
            Layout::Single(expr) => quote! {
                ::kas::Layout::draw(&#expr, draw.re());
            },
            Layout::Widget(stor, _) | Layout::Label(stor, _) => quote! {
                ::kas::Layout::draw(&#core_path.#stor, draw.re());
            },
            Layout::Frame(stor, layout, style, bg) => {
                let mut toks = quote! {
                    draw.frame(#core_path.#stor.rect, #style, #bg);
                };
                toks.append_all(layout.draw(core_path));
                toks
            }
            Layout::List(_, _, list) | Layout::Float(list) => {
                let mut toks = Toks::new();
                for item in list {
                    toks.append_all(item.layout.draw(core_path));
                }
                toks
            }
            Layout::Grid(_, _, list) => {
                let mut toks = Toks::new();
                for item in list {
                    toks.append_all(item.layout.draw(core_path));
                }
                toks
            }
        }
    }

    /// Create a Vec enumerating all children in navigation order
    ///
    /// -   `output`: the result
    fn nav_next<'a, I: Clone + Iterator<Item = &'a Child>>(
        &self,
        children: I,
        output: &mut Vec<usize>,
    ) -> std::result::Result<(), (Span, &'static str)> {
        match self {
            Layout::Align(layout, _) | Layout::Pack(layout, _) | Layout::Frame(_, layout, _, _) => {
                layout.nav_next(children, output)
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

                // Fallback case: m is not a widget therefore not a navigable child
                Ok(())
            }
            Layout::Widget(ident, _) => {
                for (i, child) in children.enumerate() {
                    if let ChildIdent::CoreField(Member::Named(ref ci)) = child.ident {
                        if *ident == *ci {
                            output.push(i);
                            return Ok(());
                        }
                    }
                }
                panic!("generated child not found")
            }
            Layout::List(_, dir, list) => {
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
            Layout::Grid(_, _, list) => {
                // TODO: sort using CellInfo?
                for item in list {
                    item.layout.nav_next(children.clone(), output)?;
                }
                Ok(())
            }
            Layout::Float(list) => {
                for item in list {
                    item.layout.nav_next(children.clone(), output)?;
                }
                Ok(())
            }
            Layout::Label(_, _) => Ok(()),
        }
    }
}
