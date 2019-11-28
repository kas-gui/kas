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
        let ty = quote! { type Data = (); };
        Ok((fns, ty))
    } else if layout == "derive" {
        if !children.is_empty() {
            layout
                .span()
                .unwrap()
                .warning("`layout = derive` is inappropriate ...")
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
                tk.size_rules(self, axis)
            }
        };
        let ty = quote! { type Data = (); };
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

            fn set_rect(&mut self, tk: &mut dyn kas::TkWindow, mut rect: kas::geom::Rect) {
                use kas::Core;
                self.core_data_mut().rect = rect;
                let margins = tk.margins(self);
                rect.pos += margins.first;
                rect.size -= (margins.first + margins.last);
                self.#ident.set_rect(tk, rect);
            }
        };
        let ty = quote! { type Data = (); };
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
                    "expected one of: empty, derive, single, horizontal, vertical, grid; found {}",
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

impl Layout {
    pub fn has_horizontal(self) -> bool {
        self != Layout::Vertical
    }
    pub fn has_vertical(self) -> bool {
        self != Layout::Horizontal
    }
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
        match self.layout {
            Layout::Horizontal => {
                let col = self.cols as usize;
                self.cols += 1;
                self.rows = 1;

                self.size.append_all(quote! {
                    solver.for_child(#col, &mut self.#ident);
                });

                self.set_rect.append_all(quote! {
                    crect.size.0 = widths[#col];
                    self.#ident.set_rect(tk, crect);
                    crect.pos.0 += (crect.size.0 + margins.inter.0) as i32;
                });
            }
            Layout::Vertical => {
                self.cols = 1;
                let row = self.rows as usize;
                self.rows += 1;

                self.size.append_all(quote! {
                    solver.for_child(#row, &mut self.#ident);
                });

                self.set_rect.append_all(quote! {
                    crect.size.1 = heights[#row];
                    self.#ident.set_rect(tk, crect);
                    crect.pos.1 += (crect.size.1 + margins.inter.1) as i32;
                });
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

                let mut get_width = quote! { widths[#col] };
                for n in (col + 1)..(c1 as usize) {
                    get_width.append_all(quote! { + widths[#n] });
                }
                let mut get_height = quote! { heights[#row] };
                for n in (row + 1)..(r1 as usize) {
                    get_height.append_all(quote! { + heights[#n] });
                }

                self.size.append_all(quote! {
                    let child_info = kas::layout::GridChildInfo {
                        col: #col,
                        col_end: #c1 as usize,
                        col_span_index: #col_span_index,
                        row: #row,
                        row_end: #r1 as usize,
                        row_span_index: #row_span_index,
                    };
                    solver.for_child(child_info, &mut self.#ident);
                });

                self.set_rect.append_all(quote! {
                    crect.pos = rect.pos + Coord(col_pos[#col], row_pos[#row]);
                    crect.size = Size(#get_width, #get_height);
                    self.#ident.set_rect(tk, crect);
                });
            }
        }
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

        let horiz_type = if self.layout.has_horizontal() {
            quote! { [kas::layout::SizeRules; #cols + 1] }
        } else {
            quote! { () }
        };
        let vert_type = if self.layout.has_vertical() {
            quote! { [kas::layout::SizeRules; #rows + 1] }
        } else {
            quote! { () }
        };
        let data_type = quote! {type Data = (#horiz_type, #vert_type);};

        let size_pre = match self.layout {
            Layout::Horizontal => quote! {
                let mut widths = [0; #cols];
                let mut solver = kas::layout::FixedRowSolver::new(
                    false,
                    axis,
                    tk,
                    &mut widths[..],
                    &mut self.#data.0
                );
            },
            Layout::Vertical => quote! {
                let mut heights = [0; #rows];
                let mut solver = kas::layout::FixedRowSolver::new(
                    true,
                    axis,
                    tk,
                    &mut heights[..],
                    &mut self.#data.1
                );
            },
            Layout::Grid => quote! {
                let mut widths = [0; #cols];
                let mut heights = [0; #rows];
                let mut col_spans = [SizeRules::EMPTY; #num_col_spans];
                let mut row_spans = [SizeRules::EMPTY; #num_row_spans];
                let spans = match axis.vertical() {
                    false => &mut col_spans[..],
                    true => &mut row_spans[..],
                };
                let mut solver = kas::layout::FixedGridSolver::new(
                    axis,
                    tk,
                    &mut widths[..],
                    &mut heights[..],
                    &mut self.#data.0,
                    &mut self.#data.1,
                    spans
                );
            },
        };

        let size_post = match self.layout {
            Layout::Horizontal => quote! {
                let rules = solver.finish(iter::empty());
            },
            Layout::Vertical => quote! {
                let rules = solver.finish(iter::empty());
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
                    let rules = if !axis.vertical() {
                        solver.finish(iter::empty() #horiz)
                    } else {
                        solver.finish(iter::empty() #vert)
                    };
                }
            }
        };

        let mut set_rect_pre = quote! {
            let mut crect = rect;
        };
        if self.layout.has_horizontal() {
            set_rect_pre.append_all(quote! {
                let mut widths = [0; #cols];
                SizeRules::solve_seq(&mut widths, &self.#data.0, rect.size.0);
            });
        }
        if self.layout.has_vertical() {
            set_rect_pre.append_all(quote! {
                let mut heights = [0; #rows];
                SizeRules::solve_seq(&mut heights, &self.#data.1, rect.size.1);
            });
        }
        if let Layout::Grid = self.layout {
            set_rect_pre.append_all(quote! {
                let mut col_pos = [0; #cols];
                let mut row_pos = [0; #rows];
                let mut pos = 0;
                for n in 0..#cols {
                    col_pos[n] = pos;
                    pos += (widths[n] + margins.inter.0) as i32;
                }
                pos = 0;
                for n in 0..#rows {
                    row_pos[n] = pos;
                    pos += (heights[n] + margins.inter.1) as i32;
                }
            });
        }

        let fns = quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWindow, mut axis: kas::layout::AxisInfo) -> kas::layout::SizeRules {
                use std::iter;
                use kas::Core;
                use kas::geom::{Size};
                use kas::layout::{AxisInfo, Sizer, SizeRules};

                #size_pre
                solver.prepare();
                #size
                #size_post

                rules + tk.margins(self).size_rules(axis, #cols as u32, #rows as u32)
            }

            fn set_rect(&mut self, tk: &mut dyn kas::TkWindow, mut rect: kas::geom::Rect) {
                use kas::Core;
                use kas::geom::{Coord, Size, Rect};
                use kas::layout::SizeRules;
                self.core_data_mut().rect = rect;
                let margins = tk.margins(self);
                rect.pos += margins.first;
                rect.size -= (margins.first + margins.last);

                #set_rect_pre
                #set_rect
            }
        };

        (fns, data_type)
    }
}
