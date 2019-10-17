// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

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
            fn size_pref(&mut self, _: &dyn kas::TkWidget, pref: kas::SizePref, _: kas::Axes) -> kas::Size {
                if pref == SizePref::Max {
                    Size::MAX
                } else {
                    Size::ZERO
                }
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
            fn size_pref(&mut self, tk: &dyn kas::TkWidget, pref: kas::SizePref, _: kas::Axes) -> kas::Size {
                tk.size_pref(self, pref)
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
            fn size_pref(&mut self, tk: &dyn kas::TkWidget, pref: kas::SizePref, axes: kas::Axes) -> kas::Size {
                self.#ident.size_pref(tk, pref, axes)
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
    size: TokenStream,
    set_rect: TokenStream,
}

impl ImplLayout {
    pub fn new(layout: &Ident) -> Result<ImplLayout> {
        if layout == "horizontal" {
            Ok(ImplLayout {
                layout: Layout::Horiz,
                cols: 0,
                rows: 1,
                size: quote! {
                    let mut size = Size::ZERO;
                },
                set_rect: quote! {
                    let mut crect = rect;
                },
            })
        } else if layout == "vertical" {
            Ok(ImplLayout {
                layout: Layout::Vert,
                cols: 1,
                rows: 0,
                size: quote! {
                    let mut size = Size::ZERO;
                },
                set_rect: quote! {
                    let mut crect = rect;
                },
            })
        } else if layout == "grid" {
            Ok(ImplLayout {
                layout: Layout::Grid,
                cols: 0,
                rows: 0,
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
                let n = (self.cols * 2) as usize;
                self.cols += 1;

                self.size.append_all(quote! {
                    let child_size = self.#ident.size_pref(tk, pref, axes);
                    if axes != Axes::Vert {
                        self.layout_widths[#n + which] = child_size.0;
                    }
                    size.0 += child_size.0;
                    size.1 = std::cmp::max(size.1, child_size.1);
                });

                self.set_rect.append_all(quote! {
                    crect.pos.0 = self.layout_widths[#n + 1] as i32;
                    crect.size.0 = self.layout_widths[#n];
                    self.#ident.set_rect(crect);
                });
            }
            Layout::Vert => {
                let n = (self.rows * 2) as usize;
                self.rows += 1;

                self.size.append_all(quote! {
                    let child_size = self.#ident.size_pref(tk, pref, axes);
                    if axes != Axes::Horiz {
                        self.layout_heights[#n + which] = child_size.1;
                    }
                    size.0 = std::cmp::max(size.0, child_size.0);
                    size.1 += child_size.1;
                });

                self.set_rect.append_all(quote! {
                    crect.pos.1 = self.layout_heights[#n + 1] as i32;
                    crect.size.1 = self.layout_heights[#n];
                    self.#ident.set_rect(crect);
                });
            }
            Layout::Grid => {
                let pos = args.as_pos()?;
                let (c0, c1) = (pos.0, pos.0 + pos.2);
                let (r0, r1) = (pos.1, pos.1 + pos.3);
                let (nc, nr) = ((2 * c0) as usize, (2 * r0) as usize);
                let (nc1, nr1) = ((2 * c1) as usize, (2 * r1) as usize);
                self.cols = self.cols.max(c1);
                self.rows = self.rows.max(r1);

                self.size.append_all(quote! {
                    let child_size = self.#ident.size_pref(tk, pref, axes);
                    // FIXME: this doesn't deal with column spans correctly!
                    if axes != Axes::Vert {
                        let i = #nc + which;
                        self.layout_widths[i] = self.layout_widths[i].max(child_size.0);
                    }
                    if axes != Axes::Horiz {
                        let i = #nr + which;
                        self.layout_heights[i] = self.layout_heights[i].max(child_size.1);
                    }
                });

                self.set_rect.append_all(quote! {
                    let pos = Coord(self.layout_widths[#nc + 1] as i32,
                            self.layout_heights[#nr + 1] as i32);
                    let mut size = Size::ZERO;
                    for c in (#nc..#nc1).step_by(2) {
                        size.0 += self.layout_widths[c];
                    }
                    for r in (#nr..#nr1).step_by(2) {
                        size.1 += self.layout_heights[r];
                    }
                    let crect = Rect { pos: pos + rect.pos, size };
                    self.#ident.set_rect(crect);
                });
            }
        }
        Ok(())
    }

    pub fn finish(self) -> (TokenStream, TokenStream, TokenStream) {
        let cols = self.cols as usize;
        let rows = self.rows as usize;
        let nc = cols * 2;
        let nr = rows * 2;
        let size = self.size;
        let set_rect = self.set_rect;

        let mut fields = quote! {
            layout_which: bool,
        };
        let mut field_ctors = quote! {
            layout_which: false,
        };

        if self.layout != Layout::Vert {
            fields.append_all(quote! {
                layout_widths: [u32; #nc + 2],
            });
            field_ctors.append_all(quote! {
                layout_widths: [0; #nc + 2],
            });
        }
        if self.layout != Layout::Horiz {
            fields.append_all(quote! {
                    layout_heights: [u32; #nr + 2],
            });
            field_ctors.append_all(quote! {
                    layout_heights: [0; #nr + 2],
            });
        }

        let size_pre = if self.layout != Layout::Grid {
            quote! {
                let mut size = Size::ZERO;
            }
        } else {
            quote! {
                if axes != Axes::Vert {
                    for i in (0..#nc).step_by(2) {
                        self.layout_widths[i + which] = 0;
                    }
                }
                if axes != Axes::Horiz {
                    for i in (0..#nr).step_by(2) {
                        self.layout_heights[i + which] = 0;
                    }
                }
            }
        };

        let size_post = match self.layout {
            Layout::Horiz => quote! {
                if axes != Axes::Vert {
                    self.layout_widths[#nc + which] = size.0;
                }
            },
            Layout::Vert => quote! {
                if axes != Axes::Horiz {
                    self.layout_heights[#nr + which] = size.1;
                }
            },
            Layout::Grid => quote! {
                let mut size = Size::ZERO;
                if axes != Axes::Vert {
                    for i in (0..#nc).step_by(2) {
                        size.0 += self.layout_widths[i + which];
                    }
                    self.layout_widths[#nc + which] = size.0;
                }
                if axes != Axes::Horiz {
                    for i in (0..#nr).step_by(2) {
                        size.1 += self.layout_heights[i + which];
                    }
                    self.layout_heights[#nr + which] = size.1;
                }
            },
        };

        let mut set_rect_pre = quote! {};
        if self.layout != Layout::Vert {
            set_rect_pre.append_all(quote! {
                let u0 = self.layout_widths[#nc + 0] as i64;
                let u1 = self.layout_widths[#nc + 1] as i64;
                let u = rect.size.0 as i64;
                let x = if u0 == u1 { 0.0 } else {
                    (u - u0) as f64 / (u1 - u0) as f64
                };
                // println!("Grid: u0={}, u1={}, u={}, x={}", u0, u1, u, x);
                assert!(0.0 <= x && x <= 1.0);
                let x1 = 1.0 - x;

                // Now calculate widths and cumulative widths
                let mut accum_w = 0;
                for i in (0..#nc).step_by(2) {
                    let u = (x1 * self.layout_widths[i] as f64
                        + x * self.layout_widths[i + 1] as f64) as u32;
                    self.layout_widths[i] = u;
                    self.layout_widths[i + 1] = accum_w;
                    accum_w += u;
                }

                // Assign excess from rounding errors to last rows/columns
                let excess = rect.size.0 - accum_w;
                let ex2 = 2 * excess as usize;
                assert!(excess >= 0 && ex2 <= #nc);
                accum_w = 0;
                for i in ((#nc - ex2)..#nc).step_by(2) {
                    self.layout_widths[i] += 1;
                    self.layout_widths[i + 1] += accum_w;
                    accum_w += 1;
                }
                assert!(rect.size.0 == self.layout_widths[#nc - 1] + self.layout_widths[#nc - 2]);
            });
        }
        if self.layout != Layout::Horiz {
            set_rect_pre.append_all(quote! {
                let u0 = self.layout_heights[#nr + 0] as i64;
                let u1 = self.layout_heights[#nr + 1] as i64;
                let u = rect.size.1 as i64;
                let y = if u0 == u1 { 0.0 } else {
                    (u - u0) as f64 / (u1 - u0) as f64
                };
                // println!("Grid: v0={}, v1={}, v={}, y={}", u0, u1, u, y);
                assert!(0.0 <= y && y <= 1.0);
                let y1 = 1.0 - y;

                // Now calculate widths and cumulative widths
                let mut accum_h = 0;
                for i in (0..#nr).step_by(2) {
                    let u = (y1 * self.layout_heights[i] as f64
                        + y * self.layout_heights[i + 1] as f64) as u32;
                    self.layout_heights[i] = u;
                    self.layout_heights[i + 1] = accum_h;
                    accum_h += u;
                }

                // Assign excess from rounding errors to last rows/columns
                let excess = rect.size.1 - accum_h;
                let ex2 = 2 * excess as usize;
                assert!(excess >= 0 && ex2 <= #nr);
                accum_h = 0;
                for i in ((#nr - ex2)..#nr).step_by(2) {
                    self.layout_heights[i] += 1;
                    self.layout_heights[i + 1] += accum_h;
                    accum_h += 1;
                }
                assert!(rect.size.1 == self.layout_heights[#nr - 1] + self.layout_heights[#nr - 2]);
            });
        }

        let fns = quote! {
            fn size_pref(&mut self, tk: &dyn kas::TkWidget, pref: kas::SizePref, axes: kas::Axes) -> kas::Size {
                use kas::{Axes, Core, Size, SizePref};

                let which = self.layout_which as usize;
                self.layout_which = !self.layout_which;

                #size_pre
                #size
                #size_post
                size
            }

            fn set_rect(&mut self, rect: kas::Rect) {
                use kas::{Core, Coord, Size, Rect};
                self.core_data_mut().rect = rect;

                #set_rect_pre
                #set_rect
            }
        };

        (fields, field_ctors, fns)
    }
}
