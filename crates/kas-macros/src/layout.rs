// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{Child, LayoutArgs, LayoutType};
use proc_macro2::TokenStream as Toks;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::Member;

pub(crate) fn data_type(children: &[Child], layout: &LayoutArgs) -> Result<(Toks, Toks, Toks)> {
    if layout.layout == LayoutType::Single && children.len() != 1 {
        return Err(Error::new(
            layout.span,
            format_args!(
                "expected 1 child marked #[widget] when using layout 'single'; found {}",
                children.len()
            ),
        ));
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
        quote! { Vec<i32> }
    } else {
        quote! { [i32; #cols] }
    };
    let row_temp = if rows > 16 {
        quote! { Vec<i32> }
    } else {
        quote! { [i32; #rows] }
    };

    Ok(match layout.layout {
        LayoutType::Single => (
            quote! { () },
            quote! { ::kas::layout::SingleSolver },
            quote! { ::kas::layout::SingleSetter },
        ),
        l @ LayoutType::Right | l @ LayoutType::Left => {
            let dt = quote! { ::kas::layout::FixedRowStorage::<#cols> };
            let solver = quote! { ::kas::layout::RowSolver::<#dt> };
            let setter = quote! { ::kas::layout::RowSetter::<#l, #col_temp, #dt> };
            (dt, solver, setter)
        }
        l @ LayoutType::Down | l @ LayoutType::Up => {
            let dt = quote! { ::kas::layout::FixedRowStorage::<#rows> };
            let solver = quote! { ::kas::layout::RowSolver::<#dt> };
            let setter = quote! { ::kas::layout::RowSetter::<#l, #row_temp, #dt> };
            (dt, solver, setter)
        }
        LayoutType::Grid => {
            let dt = quote! { ::kas::layout::FixedGridStorage::<#cols, #rows> };
            let solver = quote! {
                ::kas::layout::GridSolver::<
                    [(::kas::layout::SizeRules, u32, u32); #col_spans],
                    [(::kas::layout::SizeRules, u32, u32); #row_spans],
                    #dt,
                >
            };
            let setter = quote! { ::kas::layout::GridSetter::<#col_temp, #row_temp, #dt> };
            (dt, solver, setter)
        }
    })
}

pub(crate) fn derive(core: &Member, children: &[Child], layout: &LayoutArgs) -> Result<Toks> {
    let (storage_type, solver_type, setter_type) = data_type(children, layout)?;

    let mut cols = 0u32;
    let mut rows = 0u32;
    let mut col_spans = 0u32;
    let mut row_spans = 0u32;
    let mut size = Toks::new();
    let mut set_rect = Toks::new();
    let mut draw = quote! {
        use ::kas::{geom::Coord, WidgetCore};
        let rect = draw.get_clip_rect();
        let pos1 = rect.pos;
        let pos2 = rect.pos2();
        let disabled = disabled || self.is_disabled();
    };

    for child in children.iter() {
        let ident = &child.ident;
        let args = &child.args;

        let child_info = match layout.layout {
            LayoutType::Single => quote! { () },
            LayoutType::Right | LayoutType::Left => {
                let col = cols as usize;
                cols += 1;
                rows = 1;

                quote! { #col }
            }
            LayoutType::Down | LayoutType::Up => {
                let row = rows as usize;
                cols = 1;
                rows += 1;

                quote! { #row }
            }
            LayoutType::Grid => {
                let pos = args.as_pos()?;
                let (c0, c1) = (pos.0, pos.0 + pos.2);
                let (r0, r1) = (pos.1, pos.1 + pos.3);
                cols = cols.max(c1);
                rows = rows.max(r1);
                if pos.2 > 1 {
                    col_spans += 1;
                }
                if pos.3 > 1 {
                    row_spans += 1;
                }

                quote! {
                    ::kas::layout::GridChildInfo {
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
                data,
                #child_info,
                |axis| child.size_rules(sh, axis)
            );
        });

        set_rect.append_all(quote! { let mut align2 = align; });
        if let Some(toks) = args.halign_toks()? {
            set_rect.append_all(quote! { align2.horiz = Some(#toks); });
        }
        if let Some(toks) = args.valign_toks()? {
            set_rect.append_all(quote! { align2.vert = Some(#toks); });
        }
        set_rect.append_all(quote! {
            self.#ident.set_rect(_mgr, setter.child_rect(data, #child_info), align2);
        });

        draw.append_all(quote! {
            let c1 = self.#ident.rect().pos;
            let c2 = self.#ident.rect().pos2();
            if c1.0 <= pos2.0 && c2.0 >= pos1.0 && c1.1 <= pos2.1 && c2.1 >= pos1.1 {
                self.#ident.draw(draw, mgr, disabled);
            }
        });
    }

    let (ucols, urows) = (cols as usize, rows as usize);
    let dim = match layout.layout {
        LayoutType::Single => quote! { () },
        LayoutType::Right => quote! { (::kas::dir::Right, #ucols) },
        LayoutType::Left => quote! { (::kas::dir::Left, #ucols) },
        LayoutType::Down => quote! { (::kas::dir::Down, #urows) },
        LayoutType::Up => quote! { (::kas::dir::Up, #urows) },
        LayoutType::Grid => quote! { ::kas::layout::GridDimensions {
            cols: #cols,
            rows: #rows,
            col_spans: #col_spans,
            row_spans: #row_spans
        } },
    };

    if let Some(ref method) = layout.draw {
        draw = quote! {
            self.#method(draw, mgr, disabled);
        }
    };

    Ok(quote! {
        fn size_rules(&mut self, sh: &mut dyn ::kas::draw::SizeHandle, axis: ::kas::layout::AxisInfo)
            -> ::kas::layout::SizeRules
        {
            use ::kas::WidgetCore;
            use ::kas::layout::RulesSolver;

            let (data, _) = self.#core.layout.storage::<#storage_type>();
            let mut solver = #solver_type::new(
                axis,
                #dim,
                data,
            );
            #size
            solver.finish(data)
        }

        fn set_rect(
            &mut self,
            _mgr: &mut ::kas::event::Manager,
            rect: ::kas::geom::Rect,
            align: ::kas::layout::AlignHints
        ) {
            use ::kas::{WidgetCore, Widget};
            use ::kas::layout::{RulesSetter};
            self.core.rect = rect;

            let (data, _) = self.#core.layout.storage::<#storage_type>();
            let mut setter = #setter_type::new(
                rect,
                #dim,
                align,
                data,
            );
            #set_rect
        }

        fn draw(
            &mut self,
            draw: &mut dyn ::kas::draw::DrawHandle,
            mgr: &::kas::event::ManagerState,
            disabled: bool,
        ) {
            #draw
        }
    })
}
