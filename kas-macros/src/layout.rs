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
            fn size_pref(
                &mut self,
                _tk: &mut dyn kas::TkWidget,
                pref: kas::SizePref,
                _axes: kas::Axes,
                _index: bool,
            ) -> kas::Size {
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
            fn size_pref(
                &mut self,
                tk: &mut dyn kas::TkWidget,
                pref: kas::SizePref,
                axes: kas::Axes,
                _index: bool,
            ) -> kas::Size {
                tk.size_pref(self, pref, axes)
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
            fn size_pref(
                &mut self,
                tk: &mut dyn kas::TkWidget,
                pref: kas::SizePref,
                axes: kas::Axes,
                index: bool,
            ) -> kas::Size {
                self.#ident.size_pref(tk, pref, axes, index)
            }

            fn set_rect(&mut self, rect: kas::Rect, axes: kas::Axes) {
                use kas::Core;
                self.core_data_mut().rect = rect;
                self.#ident.set_rect(rect, axes);
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
                    let child_size = self.#ident.size_pref(tk, pref, axes, index);
                    if axes.horiz() {
                        self.layout_widths[#n + which] = child_size.0;
                    }
                    size.0 += child_size.0;
                    size.1 = std::cmp::max(size.1, child_size.1);
                });

                self.set_rect.append_all(quote! {
                    crect.pos.0 = self.layout_widths[#n + 1] as i32;
                    crect.size.0 = self.layout_widths[#n];
                    self.#ident.set_rect(crect, axes);
                });
            }
            Layout::Vert => {
                let n = (self.rows * 2) as usize;
                self.rows += 1;

                self.size.append_all(quote! {
                    let child_size = self.#ident.size_pref(tk, pref, axes, index);
                    if axes.vert() {
                        self.layout_heights[#n + which] = child_size.1;
                    }
                    size.0 = std::cmp::max(size.0, child_size.0);
                    size.1 += child_size.1;
                });

                self.set_rect.append_all(quote! {
                    crect.pos.1 = self.layout_heights[#n + 1] as i32;
                    crect.size.1 = self.layout_heights[#n];
                    self.#ident.set_rect(crect, axes);
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

                self.size.append_all(quote! {
                    let child_size = self.#ident.size_pref(tk, pref, axes, index);
                    // FIXME: this doesn't deal with column spans correctly!
                    if axes.horiz() {
                        let i = #nc2 + which;
                        self.layout_widths[i] = self.layout_widths[i].max(child_size.0);
                    }
                    if axes.vert() {
                        let i = #nr2 + which;
                        self.layout_heights[i] = self.layout_heights[i].max(child_size.1);
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

    pub fn finish(self) -> (TokenStream, TokenStream, TokenStream) {
        let cols = self.cols;
        let rows = self.rows;
        let nc2 = cols as usize * 2;
        let nr2 = rows as usize * 2;
        let size = self.size;
        let set_rect = self.set_rect;

        let mut fields = quote! {};
        let mut field_ctors = quote! {};

        if self.layout != Layout::Vert {
            fields.append_all(quote! {
                layout_widths: [u32; #nc2 + 2],
            });
            field_ctors.append_all(quote! {
                layout_widths: [0; #nc2 + 2],
            });
        }
        if self.layout != Layout::Horiz {
            fields.append_all(quote! {
                    layout_heights: [u32; #nr2 + 2],
            });
            field_ctors.append_all(quote! {
                    layout_heights: [0; #nr2 + 2],
            });
        }

        let size_pre = if self.layout != Layout::Grid {
            quote! {
                let mut size = Size::ZERO;
            }
        } else {
            quote! {
                if axes.horiz() {
                    for i in (0..#nc2).step_by(2) {
                        self.layout_widths[i + which] = 0;
                    }
                }
                if axes.vert() {
                    for i in (0..#nr2).step_by(2) {
                        self.layout_heights[i + which] = 0;
                    }
                }
            }
        };

        let size_post = match self.layout {
            Layout::Horiz => quote! {
                if axes.horiz() {
                    self.layout_widths[#nc2 + which] = size.0;
                }
            },
            Layout::Vert => quote! {
                if axes.vert() {
                    self.layout_heights[#nr2 + which] = size.1;
                }
            },
            Layout::Grid => quote! {
                let mut size = Size::ZERO;
                if axes.horiz() {
                    for i in (0..#nc2).step_by(2) {
                        size.0 += self.layout_widths[i + which];
                    }
                    self.layout_widths[#nc2 + which] = size.0;
                }
                if axes.vert() {
                    for i in (0..#nr2).step_by(2) {
                        size.1 += self.layout_heights[i + which];
                    }
                    self.layout_heights[#nr2 + which] = size.1;
                }
            },
        };

        let mut set_rect_pre = quote! {};
        if self.layout != Layout::Vert {
            set_rect_pre.append_all(quote! {
                if axes.horiz() {
                    let target = rect.size.0;
                    let u0 = self.layout_widths[#nc2 + 0];
                    let u1 = self.layout_widths[#nc2 + 1];
                    if target != u0 && (target == u1 || u1 < u0) {
                        for i in (0..(#nc2 + 2)).step_by(2) {
                            self.layout_widths.swap(i, i + 1);
                        }
                    }

                    assert!(self.layout_widths[#nc2] <= target);
                    let mut excess = target - self.layout_widths[#nc2];
                    assert!(excess == 0 || target <= self.layout_widths[#nc2 + 1]);

                    let mut rounds = 0;
                    let mut remaining = #cols;
                    while excess > 0 {
                        assert!(rounds < #cols, "Layout::set_rect: too many rounds!");
                        rounds += 1;

                        let mut next_step = 0;
                        let mut num_over = 0;
                        for i in (0..#nc2).step_by(2) {
                            let step = self.layout_widths[i + 1] - self.layout_widths[i];
                            if step > 0 {
                                num_over += 1;
                                if next_step == 0
                                    || (remaining * next_step > excess && step < next_step)
                                    || (remaining * step <= excess && step > next_step)
                                {
                                    next_step = step;
                                }
                            }
                        }

                        assert!(num_over <= remaining);
                        remaining = num_over;

                        let mut extra = 0;
                        if num_over * next_step > excess {
                            next_step = excess / num_over;  // round down
                            extra = 2 * (excess - num_over * next_step) as usize;
                        }
                        let mut total = 0;
                        for i in (0..#nc2).step_by(2) {
                            let diff = self.layout_widths[i + 1] - self.layout_widths[i];
                            let extra1 = if i < extra { 1 } else { 0 };
                            let add = diff.min(next_step + extra1);
                            self.layout_widths[i] += add;
                            total += self.layout_widths[i];
                        }
                        assert!(target >= total);
                        excess = target - total;
                    }

                    let mut total = 0;
                    for i in (0..#nc2).step_by(2) {
                        self.layout_widths[i + 1] = total;
                        total += self.layout_widths[i];
                    }
                    assert!(total == target);
                }
            });
        }
        if self.layout != Layout::Horiz {
            set_rect_pre.append_all(quote! {
                if axes.vert() {
                    let target = rect.size.1;
                    let u0 = self.layout_heights[#nr2 + 0];
                    let u1 = self.layout_heights[#nr2 + 1];
                    if target != u0 && (target == u1 || u1 < u0) {
                        for i in (0..(#nr2 + 2)).step_by(2) {
                            self.layout_heights.swap(i, i + 1);
                        }
                    }

                    assert!(self.layout_heights[#nr2] <= target);
                    let mut excess = target - self.layout_heights[#nr2];
                    assert!(excess == 0 || target <= self.layout_heights[#nr2 + 1]);

                    let mut rounds = 0;
                    let mut remaining = #rows;
                    while excess > 0 {
                        assert!(rounds < #rows, "Layout::set_rect: too many rounds!");
                        rounds += 1;

                        let mut next_step = 0;
                        let mut num_over = 0;
                        for i in (0..#nr2).step_by(2) {
                            let step = self.layout_heights[i + 1] - self.layout_heights[i];
                            if step > 0 {
                                num_over += 1;
                                if next_step == 0
                                    || (remaining * next_step > excess && step < next_step)
                                    || (remaining * step <= excess && step > next_step)
                                {
                                    next_step = step;
                                }
                            }
                        }

                        let mut extra = 0;
                        if num_over * next_step > excess {
                            next_step = excess / num_over;  // round down
                            extra = 2 * (excess - num_over * next_step) as usize;
                        }
                        let mut total = 0;
                        for i in (0..#nr2).step_by(2) {
                            let diff = self.layout_heights[i + 1] - self.layout_heights[i];
                            let extra1 = if i < extra { 1 } else { 0 };
                            let add = diff.min(next_step + extra1);
                            self.layout_heights[i] += add;
                            total += self.layout_heights[i];
                        }
                        assert!(target >= total);
                        excess = target - total;
                    }

                    let mut total = 0;
                    for i in (0..#nr2).step_by(2) {
                        self.layout_heights[i + 1] = total;
                        total += self.layout_heights[i];
                    }
                    assert!(total == target);
                }
            });
        }

        let fns = quote! {
            fn size_pref(
                &mut self,
                tk: &mut dyn kas::TkWidget,
                pref: kas::SizePref,
                axes: kas::Axes,
                index: bool,
            ) -> kas::Size {
                use kas::{Axes, Core, Size, SizePref};
                let which = index as usize;

                #size_pre
                #size
                #size_post
                size
            }

            fn set_rect(&mut self, rect: kas::Rect, axes: kas::Axes) {
                use kas::{Core, Coord, Size, Rect};
                self.core_data_mut().rect = rect;

                #set_rect_pre
                #set_rect
            }
        };

        (fields, field_ctors, fns)
    }
}
