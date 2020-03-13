// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use std::cmp::Ordering;

use crate::args::{Child, LayoutArgs, LayoutType};
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::Member;

pub(crate) fn derive(
    children: &Vec<Child>,
    layout: LayoutArgs,
    data_field: &Option<Member>,
) -> Result<(TokenStream, TokenStream)> {
    let data = if let Some(ref field) = data_field {
        quote! { self.#field }
    } else {
        if layout.layout != LayoutType::Single {
            return Err(Error::new(
                layout.span,
                "data field marked with #[layout_data] required when deriving Widget",
            ));
        }
        quote! { () }
    };

    let find_id_area = layout.area.map(|area_widget| {
        quote! {
            if self.rect().contains(coord) {
                Some(self.#area_widget.id())
            }
        }
    });

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
    }

    let mut cols: usize = 0;
    let mut rows: usize = 0;
    let mut col_spans: Vec<(u32, u32, u32)> = vec![];
    let mut row_spans: Vec<(u32, u32, u32)> = vec![];
    let mut size = TokenStream::new();
    let mut set_rect = TokenStream::new();
    let mut draw = TokenStream::new();
    let mut find_id_else = TokenStream::new();

    for child in children.iter() {
        let ident = &child.ident;
        let args = &child.args;

        let child_info = match layout.layout {
            LayoutType::Single => quote! { () },
            LayoutType::Horizontal => {
                let col = cols;
                cols += 1;
                rows = 1;

                quote! { #col }
            }
            LayoutType::Vertical => {
                let row = rows;
                cols = 1;
                rows += 1;

                quote! { #row }
            }
            LayoutType::Grid => {
                let pos = args.as_pos()?;
                let (c0, c1) = (pos.0, pos.0 + pos.2);
                let (r0, r1) = (pos.1, pos.1 + pos.3);
                cols = cols.max(c1 as usize);
                rows = rows.max(r1 as usize);
                let col = c0 as usize;
                let row = r0 as usize;
                let col_span_index = get_span(&mut col_spans, c0, c1);
                let row_span_index = get_span(&mut row_spans, r0, r1);

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

        size.append_all(quote! {
            let child = &mut self.#ident;
            solver.for_child(
                &mut #data,
                #child_info,
                |axis| child.size_rules(size_handle, axis)
            );
        });

        set_rect.append_all(quote! { let mut align = kas::AlignHints::NONE; });
        if let Some(toks) = args.halign_toks()? {
            set_rect.append_all(quote! { align.horiz = Some(#toks); });
        }
        if let Some(toks) = args.valign_toks()? {
            set_rect.append_all(quote! { align.vert = Some(#toks); });
        }
        set_rect.append_all(quote! {
            self.#ident.set_rect(size_handle, setter.child_rect(#child_info), align);
        });

        draw.append_all(quote! {
            let c0 = self.#ident.rect().pos;
            let c1 = c0 + Coord::from(self.#ident.rect().size);
            if c0.0 <= pos1.0 && c1.0 >= pos0.0 && c0.1 <= pos1.1 && c1.1 >= pos0.1 {
                self.#ident.draw(draw_handle, mgr);
            }
        });

        // TODO: more efficient search strategy?
        find_id_else.append_all(quote! {
            if self.#ident.rect().contains(coord) {
                self.#ident.find_id(coord)
            } else
        });
    }

    let num_col_spans = col_spans.len() as usize;
    let num_row_spans = row_spans.len() as usize;

    // sort by end column, then by start column in reverse order
    col_spans.sort_by(|a, b| match a.1.cmp(&b.1) {
        Ordering::Equal => a.0.cmp(&b.0).reverse(),
        o @ _ => o,
    });
    row_spans.sort_by(|a, b| match a.1.cmp(&b.1) {
        Ordering::Equal => a.0.cmp(&b.0).reverse(),
        o @ _ => o,
    });

    let dim = match layout.layout {
        LayoutType::Single => quote! { () },
        LayoutType::Horizontal => quote! { (kas::Horizontal, #cols) },
        LayoutType::Vertical => quote! { (kas::Vertical, #rows) },
        LayoutType::Grid => quote! { (#cols, #rows) },
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

    let data_type = match layout.layout {
        LayoutType::Single => quote! {
            type Data = ();
            type Solver = kas::layout::SingleSolver;
            type Setter = kas::layout::SingleSetter;
        },
        LayoutType::Horizontal => quote! {
            type Data = kas::layout::FixedRowStorage::<
                [kas::layout::SizeRules; #cols + 1]
            >;
            type Solver = kas::layout::RowSolver::<
                #col_temp,
                Self::Data,
            >;
            type Setter = kas::layout::RowSetter::<
                kas::Horizontal,
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
                kas::Vertical,
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
    };

    let size_post = match layout.layout {
        LayoutType::Single | LayoutType::Horizontal | LayoutType::Vertical => quote! {
            let mut rules = solver.finish(&mut #data, iter::empty(), iter::empty());
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
                let mut rules = solver.finish(&mut #data,
                    iter::empty() #horiz, iter::empty() # vert);
            }
        }
    };

    let find_id_body = find_id_area.unwrap_or_else(|| {
        quote! {
            #find_id_else if self.rect().contains(coord) {
                Some(self.id())
            }
        }
    });

    let fns = quote! {
        fn size_rules(
            &mut self,
            size_handle: &mut dyn kas::draw::SizeHandle,
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
                &mut #data,
            );
            #size
            #size_post

            rules
        }

        fn set_rect(
            &mut self,
            size_handle: &mut dyn kas::draw::SizeHandle,
            rect: kas::geom::Rect,
            _: kas::AlignHints,
        ) {
            use kas::{WidgetCore, Widget};
            use kas::layout::{Margins, RulesSetter};
            self.core_data_mut().rect = rect;

            let mut setter = <Self as kas::LayoutData>::Setter::new(
                rect,
                #dim,
                &mut #data,
            );
            #set_rect
        }

        fn find_id(&self, coord: kas::geom::Coord) -> Option<kas::WidgetId> {
            use kas::WidgetCore;

            #find_id_body else {
                None
            }
        }

        fn draw(
            &self,
            draw_handle: &mut dyn kas::draw::DrawHandle,
            mgr: &kas::event::ManagerState
        ) {
            use kas::{geom::Coord, WidgetCore};

            let rect = draw_handle.target_rect();
            let pos0 = rect.pos;
            let pos1 = rect.pos + Coord::from(rect.size);
            #draw
        }
    };

    Ok((fns, data_type))
}

fn get_span(spans: &mut Vec<(u32, u32, u32)>, begin: u32, end: u32) -> usize {
    if end <= begin + 1 {
        return std::usize::MAX;
    }

    for s in spans.iter() {
        if s.0 == begin && s.1 == end {
            return s.2 as usize;
        }
    }

    let i = spans.len();
    spans.push((begin, end, i as u32));
    i
}
