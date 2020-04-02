// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{Child, LayoutArgs, LayoutType};
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::Member;

pub(crate) fn data_type(children: &Vec<Child>, layout: &LayoutArgs) -> Result<TokenStream> {
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
    let mut col_spans: usize = 0;
    let mut row_spans: usize = 0;

    for child in children.iter() {
        let args = &child.args;

        match layout.layout {
            LayoutType::Single => (),
            LayoutType::Right | LayoutType::Left => {
                cols += 1;
                rows = 1;
            }
            LayoutType::Down | LayoutType::Up => {
                cols = 1;
                rows += 1;
            }
            LayoutType::Grid => {
                let pos = args.as_pos()?;
                let c1 = pos.0 + pos.2;
                let r1 = pos.1 + pos.3;
                cols = cols.max(c1 as usize);
                rows = rows.max(r1 as usize);
                if pos.2 > 1 {
                    col_spans += 1;
                }
                if pos.3 > 1 {
                    row_spans += 1;
                }
            }
        }
    }

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

    Ok(match layout.layout {
        LayoutType::Single => quote! {
            type Data = ();
            type Solver = kas::layout::SingleSolver;
            type Setter = kas::layout::SingleSetter;
        },
        l @ LayoutType::Right | l @ LayoutType::Left => quote! {
            type Data = kas::layout::FixedRowStorage::<
                [kas::layout::SizeRules; #cols + 1],
                [u32; #cols],
            >;
            type Solver = kas::layout::RowSolver::<
                Self::Data,
            >;
            type Setter = kas::layout::RowSetter::<
                #l,
                #col_temp,
                Self::Data,
            >;
        },
        l @ LayoutType::Down | l @ LayoutType::Up => quote! {
            type Data = kas::layout::FixedRowStorage::<
                [kas::layout::SizeRules; #rows + 1],
                [u32; #rows],
            >;
            type Solver = kas::layout::RowSolver::<
                Self::Data,
            >;
            type Setter = kas::layout::RowSetter::<
                #l,
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
                [(kas::layout::SizeRules, u32, u32); #col_spans],
                [(kas::layout::SizeRules, u32, u32); #row_spans],
                Self::Data,
            >;
            type Setter = kas::layout::GridSetter::<
                #col_temp,
                #row_temp,
                Self::Data,
            >;
        },
    })
}

pub(crate) fn derive(
    children: &Vec<Child>,
    layout: &LayoutArgs,
    data_field: &Option<Member>,
) -> Result<TokenStream> {
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

    let find_id_area = layout.area.as_ref().map(|area_widget| {
        quote! {
            if self.rect().contains(coord) {
                Some(self.#area_widget.id())
            }
        }
    });

    let mut cols: usize = 0;
    let mut rows: usize = 0;
    let mut size = TokenStream::new();
    let mut set_rect = TokenStream::new();
    let mut draw = TokenStream::new();
    let mut find_id_else = TokenStream::new();

    for child in children.iter() {
        let ident = &child.ident;
        let args = &child.args;

        let child_info = match layout.layout {
            LayoutType::Single => quote! { () },
            LayoutType::Right | LayoutType::Left => {
                let col = cols;
                cols += 1;
                rows = 1;

                quote! { #col }
            }
            LayoutType::Down | LayoutType::Up => {
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

                quote! {
                    kas::layout::GridChildInfo {
                        col: #c0,
                        col_end: #c1,
                        row: #r0,
                        row_end: #r1,
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
            self.#ident.set_rect(setter.child_rect(&mut #data, #child_info), align);
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

    let dim = match layout.layout {
        LayoutType::Single => quote! { () },
        LayoutType::Right => quote! { (kas::Right, #cols) },
        LayoutType::Left => quote! { (kas::Left, #cols) },
        LayoutType::Down => quote! { (kas::Down, #rows) },
        LayoutType::Up => quote! { (kas::Up, #rows) },
        LayoutType::Grid => quote! { (#cols, #rows) },
    };

    let find_id_body = find_id_area.unwrap_or_else(|| {
        quote! {
            #find_id_else if self.rect().contains(coord) {
                Some(self.id())
            }
        }
    });

    Ok(quote! {
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
            solver.finish(&mut #data)
        }

        fn set_rect(&mut self, rect: kas::geom::Rect, _: kas::AlignHints) {
            use kas::{WidgetCore, Widget};
            use kas::layout::{Margins, RulesSetter};
            self.core.rect = rect;

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
    })
}
