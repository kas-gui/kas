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
use syn::Ident;

pub(crate) fn derive(children: &Vec<Child>, layout: &Ident) -> Result<TokenStream> {
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
        Ok(quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWindow, axis: kas::geom::AxisInfo) -> kas::geom::SizeRules {
                (0, 0)
            }
        })
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
        Ok(quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWindow, axis: kas::geom::AxisInfo) -> kas::geom::SizeRules {
                tk.size_rules(self, axis)
            }
        })
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
        Ok(quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWindow, axis: kas::geom::AxisInfo) -> kas::geom::SizeRules {
                self.#ident.size_rules(tk, axis)
            }

            fn set_rect(&mut self, rect: kas::geom::Rect) {
                use kas::Core;
                self.core_data_mut().rect = rect;
                self.#ident.set_rect(rect);
            }
        })
    } else {
        return Err(Error::new(
            layout.span(),
            format_args!("expected one of empty, derive, single; found {}", layout),
        ));
    }
}

#[derive(PartialEq)]
enum Layout {
    Horiz,
    Vert,
    Grid,
}

pub(crate) struct ImplLayout {
    layout: Layout,
    cols: u32,
    rows: u32,
    col_spans: Vec<(u32, u32, u32)>,
    row_spans: Vec<(u32, u32, u32)>,
    size: TokenStream,
    set_rect: TokenStream,
}

impl ImplLayout {
    pub fn new(layout: &Ident) -> Result<ImplLayout> {
        // Note: "single" case is already handled by caller
        let layout = if layout == "horizontal" {
            Layout::Horiz
        } else if layout == "vertical" {
            Layout::Vert
        } else if layout == "grid" {
            Layout::Grid
        } else {
            return Err(Error::new(
                layout.span(),
                "expected one of: single, horizontal, vertical, grid",
            ));
        };

        Ok(ImplLayout {
            layout,
            cols: 0,
            rows: 0,
            col_spans: vec![],
            row_spans: vec![],
            size: quote! {},
            set_rect: quote! {},
        })
    }

    pub fn child(&mut self, ident: &Ident, args: &WidgetAttrArgs) -> Result<()> {
        match self.layout {
            Layout::Horiz => {
                let col = self.cols as usize;
                self.cols += 1;

                self.size.append_all(quote! {
                    if axis.horiz() {
                        axis.set_size(widths[#col]);
                    }
                    let child_rules = self.#ident.size_rules(tk, axis);
                    if axis.horiz() {
                        self.col_rules[#col] = child_rules;
                        rules += child_rules;
                    } else {
                        rules = rules.max(child_rules);
                    }
                });

                self.set_rect.append_all(quote! {
                    crect.size.0 = widths[#col];
                    self.#ident.set_rect(crect);
                    crect.pos.0 += crect.size.0 as i32;
                });
            }
            Layout::Vert => {
                let row = self.rows as usize;
                self.rows += 1;

                self.size.append_all(quote! {
                    if axis.vert() {
                        axis.set_size(heights[#row]);
                    }
                    let child_rules = self.#ident.size_rules(tk, axis);
                    if axis.vert() {
                        self.row_rules[#row] = child_rules;
                        rules += child_rules;
                    } else {
                        rules = rules.max(child_rules);
                    }
                });

                self.set_rect.append_all(quote! {
                    crect.size.1 = heights[#row];
                    self.#ident.set_rect(crect);
                    crect.pos.1 += crect.size.1 as i32;
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
                let col_end = c1 as usize;
                let row_end = r1 as usize;

                let width = if pos.2 <= 1 {
                    quote! { self.col_rules[#col] }
                } else {
                    let ind = self.get_span(false, c0, c1);
                    quote! { col_spans[#ind] }
                };
                let height = if pos.3 <= 1 {
                    quote! { self.row_rules[#row] }
                } else {
                    let ind = self.get_span(true, r0, r1);
                    quote! { row_spans[#ind] }
                };

                self.size.append_all(quote! {
                    let child_rules = self.#ident.size_rules(tk, axis);
                    if axis.horiz() {
                        #width = #width.max(child_rules);
                    } else {
                        #height = #height.max(child_rules);
                    }
                });

                self.set_rect.append_all(quote! {
                    crect.pos = rect.pos + Coord(col_pos[#col], row_pos[#row]);
                    crect.size = Size::ZERO;
                    for n in #col..#col_end {
                        crect.size.0 += widths[n];
                    }
                    for n in #row..#row_end {
                        crect.size.1 += heights[n];
                    }
                    self.#ident.set_rect(crect);
                });
            }
        }
        Ok(())
    }
    // dir: horiz (false) or vert (true)
    fn get_span(&mut self, dir: bool, begin: u32, end: u32) -> usize {
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

    pub fn finish(self) -> (TokenStream, TokenStream, TokenStream) {
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

        let mut fields = quote! {};
        let mut field_ctors = quote! {};

        if self.layout != Layout::Vert {
            fields.append_all(quote! {
                col_rules: [kas::geom::SizeRules; #cols + 1],
            });
            field_ctors.append_all(quote! {
                col_rules: Default::default(),
            });
        }
        if self.layout != Layout::Horiz {
            fields.append_all(quote! {
                row_rules: [kas::geom::SizeRules; #rows + 1],
            });
            field_ctors.append_all(quote! {
                row_rules: Default::default(),
            });
        }

        let size_pre = match self.layout {
            Layout::Horiz => quote! {
                let mut rules = SizeRules::EMPTY;
                let mut widths = [0; #cols];
                if let Some(size) = axis.fixed(false) {
                    // TODO: cache this for use by set_rect?
                    SizeRules::solve_seq(&mut widths, &self.col_rules, size);
                }
            },
            Layout::Vert => quote! {
                let mut rules = SizeRules::EMPTY;
                let mut heights = [0; #rows];
                if let Some(size) = axis.fixed(true) {
                    SizeRules::solve_seq(&mut heights, &self.row_rules, size);
                }
            },
            Layout::Grid => quote! {
                if axis.horiz() {
                    for n in 0..#cols {
                        self.col_rules[n] = SizeRules::EMPTY;
                    }
                }
                if axis.vert() {
                    for n in 0..#rows {
                        self.row_rules[n] = SizeRules::EMPTY;
                    }
                }
                let mut col_spans = [SizeRules::EMPTY; #num_col_spans];
                let mut row_spans = [SizeRules::EMPTY; #num_row_spans];
            },
        };

        let size_post = match self.layout {
            Layout::Horiz => quote! {
                if axis.horiz() {
                    self.col_rules[#cols] = rules;
                }
            },
            Layout::Vert => quote! {
                if axis.vert() {
                    self.row_rules[#rows] = rules;
                }
            },
            Layout::Grid => {
                let mut horiz = quote! {};
                let mut vert = quote! {};
                for span in &col_spans {
                    let start = span.0 as usize;
                    let end = span.1 as usize;
                    let ind = span.2 as usize;
                    horiz.append_all(quote! {
                        let mut sum = (#start..#end)
                            .map(|n| self.col_rules[n])
                            .fold(SizeRules::EMPTY, |x, y| x + y);
                        self.col_rules[#start].set_at_least_op_sub(col_spans[#ind], sum);
                    });
                }
                for span in &row_spans {
                    let start = span.0 as usize;
                    let end = span.1 as usize;
                    let ind = span.2 as usize;
                    vert.append_all(quote! {
                        let mut sum = (#start..#end)
                            .map(|n| self.row_rules[n])
                            .fold(SizeRules::EMPTY, |x, y| x + y);
                        self.row_rules[#start].set_at_least_op_sub(row_spans[#ind], sum);
                    });
                }

                quote! {
                    let rules;
                    if axis.horiz() {
                        #horiz

                        rules = self.col_rules[0..#cols].iter().copied()
                            .fold(SizeRules::EMPTY, |rules, item| rules + item);
                        self.col_rules[#cols] = rules;
                    } else {
                        #vert

                        rules = self.row_rules[0..#rows].iter().copied()
                            .fold(SizeRules::EMPTY, |rules, item| rules + item);
                        self.row_rules[#rows] = rules;
                    }
                }
            }
        };

        let mut set_rect_pre = quote! {
            let mut crect = rect;
        };
        if self.layout != Layout::Vert {
            set_rect_pre.append_all(quote! {
                let mut widths = [0; #cols];
                SizeRules::solve_seq(&mut widths, &self.col_rules, rect.size.0);
            });
        }
        if self.layout != Layout::Horiz {
            set_rect_pre.append_all(quote! {
                let mut heights = [0; #rows];
                SizeRules::solve_seq(&mut heights, &self.row_rules, rect.size.1);
            });
        }
        if self.layout == Layout::Grid {
            set_rect_pre.append_all(quote! {
                let mut col_pos = [0; #cols];
                let mut row_pos = [0; #rows];
                let mut pos = 0;
                for n in 0..#cols {
                    col_pos[n] = pos;
                    pos += widths[n] as i32;
                }
                pos = 0;
                for n in 0..#rows {
                    row_pos[n] = pos;
                    pos += heights[n] as i32;
                }
            });
        }

        let fns = quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWindow, mut axis: kas::geom::AxisInfo) -> kas::geom::SizeRules {
                use kas::Core;
                use kas::geom::{AxisInfo, Size, SizeRules};

                #size_pre
                #size
                #size_post
                rules
            }

            fn set_rect(&mut self, rect: kas::geom::Rect) {
                use kas::Core;
                use kas::geom::{Coord, Size, SizeRules, Rect};
                self.core_data_mut().rect = rect;

                #set_rect_pre
                #set_rect
            }
        };

        (fields, field_ctors, fns)
    }
}
