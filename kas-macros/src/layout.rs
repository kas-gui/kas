// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use std::cmp::Ordering;

use crate::args::{Child, LayoutArgs, LayoutType, WidgetAttrArgs};
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::Member;

pub(crate) fn derive(
    children: &Vec<Child>,
    layout: LayoutArgs,
    data_field: &Option<Member>,
) -> Result<(TokenStream, TokenStream)> {
    let data = data_field.as_ref().ok_or_else(|| {
        Error::new(
            layout.span,
            "data field marked with #[layout_data] required when deriving Widget",
        )
    })?;

    let is_frame = layout.is_frame;
    if layout.layout == LayoutType::Single {
        if !children.len() == 1 {
            return Err(Error::new(
                layout.span,
                format_args!(
                    "expected 1 child when using layout 'single'; found {}",
                    children.len()
                ),
            ));
        }
        let ident = &children[0].ident;
        let fns = quote! {
            fn size_rules(
                &mut self,
                size_handle: &mut dyn kas::theme::SizeHandle,
                axis: kas::layout::AxisInfo
            )
                -> kas::layout::SizeRules
            {
                use kas::geom::Size;
                let frame_size = if #is_frame {
                    size_handle.outer_frame()
                } else {
                    (Size::ZERO, Size::ZERO)
                };
                self.#data = frame_size;
                self.#ident.size_rules(size_handle, axis)
                    + axis.extract_size(frame_size.0)
                    + axis.extract_size(frame_size.1)
            }

            fn set_rect(
                &mut self,
                size_handle: &mut dyn kas::theme::SizeHandle,
                rect: kas::geom::Rect)
            {
                use kas::WidgetCore;
                use kas::layout::RulesSetter;
                self.core_data_mut().rect = rect;

                let mut setter = <Self as kas::LayoutData>::Setter::new(
                    rect,
                    self.#data,
                    &mut (),
                );
                self.#ident.set_rect(size_handle, setter.child_rect(()));
            }

            fn draw(
                &self,
                draw_handle: &mut dyn kas::theme::DrawHandle,
                mgr: &kas::event::Manager
            ) {
                use kas::WidgetCore;
                if #is_frame {
                    draw_handle.outer_frame(self.core_data().rect);
                }
                self.#ident.draw(draw_handle, mgr);
            }
        };
        let ty = quote! {
            type Data = (kas::geom::Size, kas::geom::Size);
            type Solver = ();
            type Setter = kas::layout::SingleSetter;
        };
        Ok((fns, ty))
    } else {
        if is_frame {
            // TODO: support?
            return Err(Error::new(
                layout.span,
                "frame is (currently) only allowed for layout = single",
            ));
        }

        // TODO: this could be rewritten
        let mut impl_layout = ImplLayout::new(layout.layout, data);
        for child in children.iter() {
            impl_layout.child(&child.ident, &child.args)?;
        }
        Ok(impl_layout.finish())
    }
}

pub(crate) struct ImplLayout<'a> {
    layout: LayoutType,
    cols: u32,
    rows: u32,
    col_spans: Vec<(u32, u32, u32)>,
    row_spans: Vec<(u32, u32, u32)>,
    data: &'a Member,
    size: TokenStream,
    set_rect: TokenStream,
    draw: TokenStream,
}

impl<'a> ImplLayout<'a> {
    pub fn new(layout: LayoutType, data: &'a Member) -> Self {
        ImplLayout {
            layout,
            cols: 0,
            rows: 0,
            col_spans: vec![],
            row_spans: vec![],
            data,
            size: quote! {},
            set_rect: quote! {},
            draw: quote! {},
        }
    }

    pub fn child(&mut self, ident: &Member, args: &WidgetAttrArgs) -> Result<()> {
        let data = self.data;

        let child_info = match self.layout {
            LayoutType::Horizontal => {
                let col = self.cols as usize;
                self.cols += 1;
                self.rows = 1;

                quote! { #col }
            }
            LayoutType::Vertical => {
                let row = self.rows as usize;
                self.cols = 1;
                self.rows += 1;

                quote! { #row }
            }
            LayoutType::Grid => {
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
            LayoutType::Single => unreachable!(),
        };

        self.size.append_all(quote! {
            let child = &mut self.#ident;
            solver.for_child(
                &mut self.#data,
                #child_info,
                |axis| child.size_rules(size_handle, axis)
            );
        });

        self.set_rect.append_all(quote! {
            self.#ident.set_rect(size_handle, setter.child_rect(#child_info));
        });

        self.draw.append_all(quote! {
            let c0 = self.#ident.rect().pos;
            let c1 = c0 + Coord::from(self.#ident.rect().size);
            if c0.0 <= pos1.0 && c1.0 >= pos0.0 && c0.1 <= pos1.1 && c1.1 >= pos0.1 {
                self.#ident.draw(draw_handle, mgr);
            }
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
        let draw = self.draw;

        // sort by end column, then by start column in reverse order
        col_spans.sort_by(|a, b| match a.1.cmp(&b.1) {
            Ordering::Equal => a.0.cmp(&b.0).reverse(),
            o @ _ => o,
        });
        row_spans.sort_by(|a, b| match a.1.cmp(&b.1) {
            Ordering::Equal => a.0.cmp(&b.0).reverse(),
            o @ _ => o,
        });

        let dim = match self.layout {
            LayoutType::Horizontal => quote! { (kas::layout::Horizontal, #cols) },
            LayoutType::Vertical => quote! { (kas::layout::Vertical, #rows) },
            LayoutType::Grid => quote! { (#cols, #rows) },
            LayoutType::Single => unreachable!(),
        };

        let col_temp = if cols > 16 {
            quote! { Vec<u32> }
        } else {
            quote! { [u32; #cols] }
        };
        let row_temp = if rows > 16 {
            quote! { Vec<u32> }
        } else {
            quote! { [u32; #rows] }
        };

        let data_type = match self.layout {
            LayoutType::Horizontal => quote! {
                type Data = kas::layout::FixedRowStorage::<
                    [kas::layout::SizeRules; #cols + 1]
                >;
                type Solver = kas::layout::RowSolver::<
                    #col_temp,
                    Self::Data,
                >;
                type Setter = kas::layout::RowSetter::<
                    kas::layout::Horizontal,
                    #col_temp,
                    Self::Data,
                >;
            },
            LayoutType::Vertical => quote! {
                type Data = kas::layout::FixedRowStorage::<
                    [kas::layout::SizeRules; #rows + 1],
                >;
                type Solver = kas::layout::RowSolver::<
                    #row_temp,
                    Self::Data,
                >;
                type Setter = kas::layout::RowSetter::<
                    kas::layout::Vertical,
                    #row_temp,
                    Self::Data,
                >;
            },
            LayoutType::Grid => quote! {
                type Data = kas::layout::FixedGridStorage::<
                    [kas::layout::SizeRules; #cols + 1],
                    [kas::layout::SizeRules; #rows + 1],
                >;
                type Solver = kas::layout::GridSolver::<
                    #col_temp,
                    #row_temp,
                    [kas::layout::SizeRules; #num_col_spans],
                    [kas::layout::SizeRules; #num_row_spans],
                    Self::Data,
                >;
                type Setter = kas::layout::GridSetter::<
                    #col_temp,
                    #row_temp,
                    Self::Data,
                >;
            },
            LayoutType::Single => unreachable!(),
        };

        let size_post = match self.layout {
            LayoutType::Horizontal | LayoutType::Vertical => quote! {
                let rules = solver.finish(&mut self.#data, iter::empty(), iter::empty());
            },
            LayoutType::Grid => {
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
            LayoutType::Single => unreachable!(),
        };

        let fns = quote! {
            fn size_rules(
                &mut self,
                size_handle: &mut dyn kas::theme::SizeHandle,
                mut axis: kas::layout::AxisInfo
            )
                -> kas::layout::SizeRules
            {
                use std::iter;
                use kas::WidgetCore;
                use kas::layout::RulesSolver;

                let mut solver = <Self as kas::LayoutData>::Solver::new(
                    axis,
                    #dim,
                    &mut self.#data,
                );
                #size
                #size_post

                rules
            }

            fn set_rect(
                &mut self,
                size_handle: &mut dyn kas::theme::SizeHandle,
                rect: kas::geom::Rect)
            {
                use kas::WidgetCore;
                use kas::layout::{Margins, RulesSetter};
                self.core_data_mut().rect = rect;

                let mut setter = <Self as kas::LayoutData>::Setter::new(
                    rect,
                    Margins::ZERO,
                    #dim,
                    &mut self.#data,
                );
                #set_rect
            }

            fn draw(
                &self,
                draw_handle: &mut dyn kas::theme::DrawHandle,
                mgr: &kas::event::Manager
            ) {
                use kas::{geom::Coord, WidgetCore};
                let rect = draw_handle.target_rect();
                let pos0 = rect.pos;
                let pos1 = rect.pos + Coord::from(rect.size);
                #draw
            }
        };

        (fns, data_type)
    }
}
