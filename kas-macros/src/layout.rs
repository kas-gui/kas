// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use std::cmp::Ordering;

use crate::args::{Child, WidgetAttrArgs};
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::spanned::Spanned;
use syn::{Ident, Member};

pub(crate) fn derive(
    children: &Vec<Child>,
    layout: &Ident,
    data_field: &Option<Member>,
) -> Result<(TokenStream, TokenStream)> {
    if layout == "empty" {
        if !children.is_empty() {
            layout
                .span()
                .unwrap()
                .warning("`layout = empty` is inappropriate ...")
                .emit();
            children[0]
                .ident
                .span()
                .unwrap()
                .warning("... when a child widget is present")
                .emit();
        }
        let fns = quote! {
            fn size_rules(
                &mut self,
                tk: &mut dyn kas::TkWindow,
                axis: kas::layout::AxisInfo
            )
                -> kas::layout::SizeRules
            {
                (0, 0)
            }
        };
        let ty = quote! {
            type Data = ();
            type Solver = ();
            type Setter = ();
        };
        Ok((fns, ty))
    } else if layout == "single" {
        if !children.len() == 1 {
            return Err(Error::new(
                layout.span(),
                format_args!(
                    "expected 1 child when using `layout = single`; found {}",
                    children.len()
                ),
            ));
        }
        let ident = &children[0].ident;
        let fns = quote! {
            fn size_rules(
                &mut self,
                tk: &mut dyn kas::TkWindow,
                axis: kas::layout::AxisInfo
            )
                -> kas::layout::SizeRules
            {
                self.#ident.size_rules(tk, axis) + tk.margins(self).size_rules(axis, 0, 0)
            }

            fn set_rect(&mut self, tk: &mut dyn kas::TkWindow, rect: kas::geom::Rect) {
                use kas::Core;
                use kas::layout::RulesSetter;
                self.core_data_mut().rect = rect;

                let mut setter = <Self as kas::LayoutData>::Setter::new(
                    rect,
                    tk.margins(self),
                    &mut (),
                );
                self.#ident.set_rect(tk, setter.child_rect(()));
            }
        };
        let ty = quote! {
            type Data = ();
            type Solver = ();
            type Setter = kas::layout::SingleSetter;
        };
        Ok((fns, ty))
    } else {
        let lay = if layout == "horizontal" {
            Layout::Horizontal
        } else if layout == "vertical" {
            Layout::Vertical
        } else if layout == "grid" {
            Layout::Grid
        } else {
            return Err(Error::new(
                layout.span(),
                format_args!(
                    "expected one of: empty, single, horizontal, vertical, grid; found {}",
                    layout
                ),
            ));
        };

        let data = data_field.as_ref().ok_or_else(|| {
            Error::new(
                layout.span(),
                "data field marked with #[layout_data] required for this layout",
            )
        })?;

        // TODO: this could be rewritten
        let mut impl_layout = ImplLayout::new(lay, data);
        for child in children.iter() {
            impl_layout.child(&child.ident, &child.args)?;
        }
        Ok(impl_layout.finish())
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Layout {
    Horizontal,
    Vertical,
    Grid,
}

pub(crate) struct ImplLayout<'a> {
    layout: Layout,
    cols: u32,
    rows: u32,
    col_spans: Vec<(u32, u32, u32)>,
    row_spans: Vec<(u32, u32, u32)>,
    data: &'a Member,
    size: TokenStream,
    set_rect: TokenStream,
}

impl<'a> ImplLayout<'a> {
    pub fn new(layout: Layout, data: &'a Member) -> Self {
        ImplLayout {
            layout,
            cols: 0,
            rows: 0,
            col_spans: vec![],
            row_spans: vec![],
            data,
            size: quote! {},
            set_rect: quote! {},
        }
    }

    pub fn child(&mut self, ident: &Member, args: &WidgetAttrArgs) -> Result<()> {
        let data = self.data;

        let child_info = match self.layout {
            Layout::Horizontal => {
                let col = self.cols as usize;
                self.cols += 1;
                self.rows = 1;

                quote! { #col }
            }
            Layout::Vertical => {
                let row = self.rows as usize;
                self.cols = 1;
                self.rows += 1;

                quote! { #row }
            }
            Layout::Grid => {
                let pos = args.as_pos()?;
                let (c0, c1) = (pos.0, pos.0 + pos.2);
                let (r0, r1) = (pos.1, pos.1 + pos.3);
                self.cols = self.cols.max(c1);
                self.rows = self.rows.max(r1);
                let col = c0 as usize;
                let row = r0 as usize;
                let col_span_index = self.get_span(false, c0, c1);
                let row_span_index = self.get_span(true, r0, r1);

                quote! {
                    kas::layout::GridChildInfo {
                        col: #col,
                        col_end: #c1 as usize,
                        col_span_index: #col_span_index,
                        row: #row,
                        row_end: #r1 as usize,
                        row_span_index: #row_span_index,
                    }
                }
            }
        };

        self.size.append_all(quote! {
            let child = &mut self.#ident;
            solver.for_child(
                &mut self.#data,
                #child_info,
                |axis| child.size_rules(tk, axis)
            );
        });

        self.set_rect.append_all(quote! {
            self.#ident.set_rect(tk, setter.child_rect(#child_info));
        });

        Ok(())
    }
    // dir: horiz (false) or vert (true)
    fn get_span(&mut self, dir: bool, begin: u32, end: u32) -> usize {
        if end <= begin + 1 {
            return std::usize::MAX;
        }

        let list = if dir {
            &mut self.row_spans
        } else {
            &mut self.col_spans
        };

        for s in list.iter() {
            if s.0 == begin && s.1 == end {
                return s.2 as usize;
            }
        }

        let i = list.len();
        list.push((begin, end, i as u32));
        i
    }

    pub fn finish(self) -> (TokenStream, TokenStream) {
        let data = self.data;
        let cols = self.cols as usize;
        let rows = self.rows as usize;
        let mut col_spans = self.col_spans;
        let mut row_spans = self.row_spans;
        let num_col_spans = col_spans.len() as usize;
        let num_row_spans = row_spans.len() as usize;
        let size = self.size;
        let set_rect = self.set_rect;

        // sort by end column, then by start column in reverse order
        col_spans.sort_by(|a, b| match a.1.cmp(&b.1) {
            Ordering::Equal => a.0.cmp(&b.0).reverse(),
            o @ _ => o,
        });
        row_spans.sort_by(|a, b| match a.1.cmp(&b.1) {
            Ordering::Equal => a.0.cmp(&b.0).reverse(),
            o @ _ => o,
        });

        let data_type = match self.layout {
            Layout::Horizontal => quote! {
                type Data = kas::layout::FixedRowStorage::<
                    [kas::layout::SizeRules; #cols + 1]
                >;
                type Solver = kas::layout::FixedRowSolver::<
                    kas::layout::Horizontal,
                    [kas::layout::SizeRules; #cols + 1],
                    [u32; #cols],
                >;
                type Setter = kas::layout::FixedRowSetter::<
                    kas::layout::Horizontal,
                    [kas::layout::SizeRules; #cols + 1],
                    [u32; #cols],
                >;
            },
            Layout::Vertical => quote! {
                type Data = kas::layout::FixedRowStorage::<
                    [kas::layout::SizeRules; #rows + 1],
                >;
                type Solver = kas::layout::FixedRowSolver::<
                    kas::layout::Vertical,
                    [kas::layout::SizeRules; #rows + 1],
                    [u32; #rows],
                >;
                type Setter = kas::layout::FixedRowSetter::<
                    kas::layout::Vertical,
                    [kas::layout::SizeRules; #rows + 1],
                    [u32; #rows],
                >;
            },
            Layout::Grid => quote! {
                type Data = kas::layout::FixedGridStorage::<
                    [kas::layout::SizeRules; #cols + 1],
                    [kas::layout::SizeRules; #rows + 1],
                >;
                type Solver = kas::layout::FixedGridSolver::<
                    [kas::layout::SizeRules; #cols + 1],
                    [kas::layout::SizeRules; #rows + 1],
                    [u32; #cols],
                    [u32; #rows],
                    [kas::layout::SizeRules; #num_col_spans],
                    [kas::layout::SizeRules; #num_row_spans],
                >;
                type Setter = kas::layout::FixedGridSetter::<
                    [kas::layout::SizeRules; #cols + 1],
                    [kas::layout::SizeRules; #rows + 1],
                    [u32; #cols],
                    [u32; #rows],
                >;
            },
        };

        let size_post = match self.layout {
            Layout::Horizontal | Layout::Vertical => quote! {
                let rules = solver.finish(&mut self.#data, iter::empty(), iter::empty());
            },
            Layout::Grid => {
                let mut horiz = quote! {};
                let mut vert = quote! {};
                for span in &col_spans {
                    let start = span.0 as usize;
                    let end = span.1 as usize;
                    let ind = span.2 as usize;
                    horiz.append_all(quote! {
                        .chain(iter::once((#start, #end, #ind)))
                    });
                }
                for span in &row_spans {
                    let start = span.0 as usize;
                    let end = span.1 as usize;
                    let ind = span.2 as usize;
                    vert.append_all(quote! {
                        .chain(iter::once((#start, #end, #ind)))
                    });
                }

                quote! {
                    let rules = solver.finish(&mut self.#data,
                        iter::empty() #horiz, iter::empty() # vert);
                }
            }
        };

        let fns = quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWindow, mut axis: kas::layout::AxisInfo) -> kas::layout::SizeRules {
                use std::iter;
                use kas::Core;
                use kas::layout::RulesSolver;

                let mut solver = <Self as kas::LayoutData>::Solver::new(
                    axis,
                    &mut self.#data,
                );
                #size
                #size_post

                rules + tk.margins(self).size_rules(axis, #cols as u32, #rows as u32)
            }

            fn set_rect(&mut self, tk: &mut dyn kas::TkWindow, mut rect: kas::geom::Rect) {
                use kas::Core;
                use kas::layout::RulesSetter;
                self.core_data_mut().rect = rect;

                let mut setter = <Self as kas::LayoutData>::Setter::new(
                    rect,
                    tk.margins(self),
                    &mut self.#data,
                );
                #set_rect
            }
        };

        (fns, data_type)
    }
}
