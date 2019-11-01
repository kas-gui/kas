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
            fn size_rules(&mut self, tk: &mut dyn kas::TkWidget, axis: kas::AxisInfo) -> kas::SizeRules {
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
            fn size_rules(&mut self, tk: &mut dyn kas::TkWidget, axis: kas::AxisInfo) -> kas::SizeRules {
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
            fn size_rules(&mut self, tk: &mut dyn kas::TkWidget, axis: kas::AxisInfo) -> kas::SizeRules {
                self.#ident.size_rules(tk, axis)
            }

            fn set_rect(&mut self, rect: kas::Rect) {
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
    spans: Vec<(u32, u32, bool, u32)>,
    size: TokenStream,
    set_rect: TokenStream,
}

impl ImplLayout {
    pub fn new(layout: &Ident) -> Result<ImplLayout> {
        if layout == "horizontal" {
            Ok(ImplLayout {
                layout: Layout::Horiz,
                cols: 0,
                rows: 0,
                spans: vec![],
                size: quote! {},
                set_rect: quote! {
                    let mut crect = rect;
                },
            })
        } else if layout == "vertical" {
            Ok(ImplLayout {
                layout: Layout::Vert,
                cols: 0,
                rows: 0,
                spans: vec![],
                size: quote! {},
                set_rect: quote! {
                    let mut crect = rect;
                },
            })
        } else if layout == "grid" {
            Ok(ImplLayout {
                layout: Layout::Grid,
                cols: 0,
                rows: 0,
                spans: vec![],
                size: quote! {},
                set_rect: quote! {},
            })
        } else {
            // Note: "single" case is already handled by caller
            Err(Error::new(
                layout.span(),
                "expected one of: single, horizontal, vertical, grid",
            ))
        }
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
                let (nc2, nr2) = ((2 * c0) as usize, (2 * r0) as usize);
                let (nc2_1, nr2_1) = ((2 * c1) as usize, (2 * r1) as usize);
                self.cols = self.cols.max(c1);
                self.rows = self.rows.max(r1);

                let width = if pos.2 == 1 {
                    quote! { self.layout_widths[#nc2 + which] }
                } else {
                    assert!(pos.2 > 1);
                    let span_index = self.get_span(false, c0, c1);
                    quote! { layout_spans[#span_index + which] }
                };
                let height = if pos.3 == 1 {
                    quote! { self.layout_heights[#nr2 + which] }
                } else {
                    assert!(pos.3 > 1);
                    let span_index = self.get_span(true, r0, r1);
                    quote! { layout_spans[#span_index + which] }
                };

                self.size.append_all(quote! {
                    let child_size = self.#ident.size_pref(tk, pref, axes, index);
                    if axes.horiz() {
                        #width = #width.max(child_size.0);
                    }
                    if axes.vert() {
                        #height = #height.max(child_size.1);
                    }
                });

                self.set_rect.append_all(quote! {
                    let pos = Coord(self.layout_widths[#nc2 + 1] as i32,
                            self.layout_heights[#nr2 + 1] as i32);
                    let mut size = Size::ZERO;
                    for c in (#nc2..#nc2_1).step_by(2) {
                        size.0 += self.layout_widths[c];
                    }
                    for r in (#nr2..#nr2_1).step_by(2) {
                        size.1 += self.layout_heights[r];
                    }
                    let crect = Rect { pos: pos + rect.pos, size };
                    self.#ident.set_rect(crect, axes);
                });
            }
        }
        Ok(())
    }
    // dir: horiz (false) or vert (true)
    fn get_span(&mut self, dir: bool, begin: u32, end: u32) -> usize {
        for s in &self.spans {
            if s.0 == begin && s.1 == end && s.2 == dir {
                return s.3 as usize;
            }
        }

        let i = self.spans.len();
        self.spans.push((begin, end, dir, i as u32));
        i
    }

    pub fn finish(self) -> (TokenStream, TokenStream, TokenStream) {
        let cols = self.cols as usize;
        let rows = self.rows as usize;
        let mut spans = self.spans;
        let ns2 = spans.len() * 2;
        let size = self.size;
        let set_rect = self.set_rect;

        // sort by end column, then by start column in reverse order
        spans.sort_by(|a, b| match a.1.cmp(&b.1) {
            Ordering::Equal => a.0.cmp(&b.0).reverse(),
            o @ _ => o,
        });

        let mut fields = quote! {};
        let mut field_ctors = quote! {};

        if self.layout != Layout::Vert {
            fields.append_all(quote! {
                col_rules: [kas::SizeRules; #cols + 1],
            });
            field_ctors.append_all(quote! {
                col_rules: Default::default(),
            });
        }
        if self.layout != Layout::Horiz {
            fields.append_all(quote! {
                row_rules: [kas::SizeRules; #rows + 1],
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
                for span in &spans {
                    let start2 = 2 * span.0 as usize;
                    let end2 = 2 * span.1 as usize;
                    let ind = span.3 as usize;
                    if !span.2 {
                        horiz.append_all(quote! {
                            let start = #start2 + which;
                            let mut sum = (start..#end2)
                                .step_by(2)
                                .map(|i| self.layout_widths[i])
                                .sum();
                            if layout_spans[#ind] > sum {
                                self.layout_widths[start] += layout_spans[#ind] - sum;
                            }
                        });
                    } else {
                        vert.append_all(quote! {
                            let start = #start2 + which;
                            let mut sum = (start..#end2)
                                .step_by(2)
                                .map(|i| self.layout_heights[i])
                                .sum();
                            if layout_spans[#ind] > sum {
                                self.layout_heights[start] += layout_spans[#ind] - sum;
                            }
                        });
                    }
                }
                quote! {
                    let mut size = Size::ZERO;
                    if axes.horiz() {
                        #horiz
                        for i in (0..#nc2).step_by(2) {
                            size.0 += self.layout_widths[i + which];
                        }
                        self.layout_widths[#nc2 + which] = size.0;
                    }
                    if axes.vert() {
                        #vert
                        for i in (0..#nr2).step_by(2) {
                            size.1 += self.layout_heights[i + which];
                        }
                        self.layout_heights[#nr2 + which] = size.1;
                    }
                }
            }
        };

        let mut set_rect_pre = quote! {
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
        };

        let fns = quote! {
            fn size_rules(&mut self, tk: &mut dyn kas::TkWidget, mut axis: kas::AxisInfo) -> kas::SizeRules {
                use kas::{AxisInfo, Core, Size, SizeRules};

                #size_pre
                #size
                #size_post
                rules
            }

            fn set_rect(&mut self, rect: kas::Rect) {
                use kas::{Core, Coord, Size, SizeRules, Rect};
                self.core_data_mut().rect = rect;

                #set_rect_pre
                #set_rect
            }
        };

        (fields, field_ctors, fns)
    }
}
