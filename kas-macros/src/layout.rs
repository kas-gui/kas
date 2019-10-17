// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::Ident;
use syn::parse::{Error, Result};
use syn::spanned::Spanned;
use crate::args::{Child, WidgetAttrArgs};

pub(crate) fn derive(children: &Vec<Child>, layout: &Ident)
    -> Result<TokenStream>
{
    if layout == "empty" {
        if !children.is_empty() {
            layout.span().unwrap()
                .warning("`layout = empty` is inappropriate ...")
                .emit();
            children[0].ident.span().unwrap()
                .warning("... when a child widget is present")
                .emit();
        }
        Ok(quote! {
            fn size_pref(&mut self, _: &dyn kas::TkWidget, pref: kas::SizePref) -> kas::Size {
                let size = if pref == SizePref::Max {
                    Size::MAX
                } else {
                    Size::ZERO
                };
                use kas::Core;
                println!("[{}] empty size({:?}): {:?}", self.number(), pref, size);
                size
            }
        })
    } else if layout == "derive" {
        if !children.is_empty() {
            layout.span().unwrap()
                .warning("`layout = derive` is inappropriate ...")
                .emit();
            children[0].ident.span().unwrap()
                .warning("... when a child widget is present")
                .emit();
        }
        Ok(quote! {
            fn size_pref(&mut self, tk: &dyn kas::TkWidget, pref: kas::SizePref) -> kas::Size {
                let size = tk.size_pref(self, pref);
                use kas::Core;
                println!("[{}] derive size({:?}): {:?}", self.number(), pref, size);
                size
            }
        })
    } else if layout == "single" {
        if !children.len() == 1 {
            return Err(Error::new(layout.span(),
                format_args!("expected 1 child when using `layout = single`; found {}", children.len())));
        }
        let ident = &children[0].ident;
        Ok(quote! {
            fn size_pref(&mut self, tk: &dyn kas::TkWidget, pref: kas::SizePref) -> kas::Size {
                let size = self.#ident.size_pref(tk, pref);
                use kas::Core;
                println!("[{}] single size({:?}): {:?}", self.number(), pref, size);
                size
            }

            fn set_rect(&mut self, rect: kas::Rect) {
                use kas::Core;
                self.core_data_mut().rect = rect;
                self.#ident.set_rect(rect);
            }
        })
    } else {
        return Err(Error::new(layout.span(),
            format_args!("expected one of empty, derive, single; found {}", layout)));
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
    cols: usize,
    rows: usize,
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
                size: quote!{
                    let mut size = Size::ZERO;
                },
                set_rect: quote!{
                    let u0 = self.layout_total[0] as i64;
                    let u1 = self.layout_total[1] as i64;
                    let u = rect.size.0 as i64;
                    let mut x = if u0 == u1 { 0.0 } else {
                        (u - u0) as f64 / (u1 - u0) as f64
                    };
                    println!("Horiz: u0={}, u1={}, u={}, x={}", u0, u1, u, x);
                    if !(0.0 <= x && x <= 1.0) {
                        println!("Unexpected ratio (horiz): u0={}, u1={}, u={}, x={}", u0, u1, u, x);
                        if x < 0.0 {
                            x = 0.0;
                        } else {
                            x = 1.0;
                        }
                    }
                    let x1 = 1.0 - x;

                    let mut crect = rect;
                },
            })
        } else if layout == "vertical" {
            Ok(ImplLayout {
                layout: Layout::Vert,
                cols: 1,
                rows: 0,
                size: quote!{
                    let mut size = Size::ZERO;
                },
                set_rect: quote!{
                    let u0 = self.layout_total[0] as i64;
                    let u1 = self.layout_total[1] as i64;
                    let u = rect.size.1 as i64;
                    let mut x = if u0 == u1 { 0.0 } else {
                        (u - u0) as f64 / (u1 - u0) as f64
                    };
                    println!("Vert: u0={}, u1={}, u={}, x={}", u0, u1, u, x);
                    if !(0.0 <= x && x <= 1.0) {
                        println!("Unexpected ratio (vert): u0={}, u1={}, u={}, x={}", u0, u1, u, x);
                        if x < 0.0 {
                            x = 0.0;
                        } else {
                            x = 1.0;
                        }
                    }
                    let x1 = 1.0 - x;

                    let mut crect = rect;
                },
            })
        } else if layout == "grid" {
            unimplemented!()
        } else {
            // Note: "single" case is already handled by caller
            Err(Error::new(layout.span(),
                "expected one of: single, horizontal, vertical, grid"))
        }
    }
    
    pub fn child(&mut self, ident: &Ident, _args: &WidgetAttrArgs) {
        match self.layout {
            Layout::Horiz => {
                let n = self.cols * 2;
                self.cols += 1;
                
                self.size.append_all(quote!{
                    let child_size = self.#ident.size_pref(tk, pref);
                    self.layout_sizes[#n + which] = child_size.0;
                    size.0 += child_size.0;
                    size.1 = std::cmp::max(size.1, child_size.1);
                });
                
                // This rounds down, which is fine except that a few pixels may go unused FIXME
                self.set_rect.append_all(quote!{
                    let u = (x1 * self.layout_sizes[#n] as f64
                        + x * self.layout_sizes[#n + 1] as f64) as u32;
                    println!("cache: {}, {}; u = {}", self.layout_sizes[#n], self.layout_sizes[#n + 1], u);
                    crect.size.0 = u;
                    self.#ident.set_rect(crect);
                    crect.pos.0 += u as i32;
                });
            }
            Layout::Vert => {
                let n = self.rows * 2;
                self.rows += 1;
                
                self.size.append_all(quote!{
                    let child_size = self.#ident.size_pref(tk, pref);
                    self.layout_sizes[#n + which] = child_size.1;
                    size.0 = std::cmp::max(size.0, child_size.0);
                    size.1 += child_size.1;
                });
                
                // This rounds down, which is fine except that a few pixels may go unused FIXME
                self.set_rect.append_all(quote!{
                    let u = (x1 * self.layout_sizes[#n] as f64
                        + x * self.layout_sizes[#n + 1] as f64) as u32;
                    crect.size.1 = u;
                    self.#ident.set_rect(crect);
                    crect.pos.1 += u as i32;
                });
            }
            Layout::Grid => {
                unimplemented!()
            }
        }
    }
    
    pub fn finish(self) -> (TokenStream, TokenStream, TokenStream) {
        let size = self.size;
        let set_rect = self.set_rect;
        
        let (fields, field_ctors);
        
        let axis = match self.layout {
            Layout::Horiz => quote!{ 0 },
            Layout::Vert => quote!{ 1 },
            Layout::Grid => unimplemented!()
        };
        
        match self.layout {
            Layout::Horiz | Layout::Vert => {
                let n = if self.layout == Layout::Horiz { 
                    self.cols * 2
                } else {
                    self.rows * 2
                };
                fields = quote! {
                    layout_sizes: [u32; #n],
                    layout_total: [u32; 2],
                    layout_which: bool,
                };
                field_ctors = quote! {
                    layout_sizes: [0; #n],
                    layout_total: [0; 2],
                    layout_which: false,
                };
            }
            _ => unimplemented!()
        }
        
        let fns = quote! {
            fn size_pref(&mut self, tk: &dyn kas::TkWidget, pref: kas::SizePref) -> kas::Size {
                use kas::{Core, Size, SizePref};
                
                let which = self.layout_which as usize;
                self.layout_which = !self.layout_which;
                
                #size
                self.layout_total[which] = size.#axis;
                
                println!("[{}] size({:?}): {:?}", self.number(), pref, size);
                size
            }

            fn set_rect(&mut self, rect: kas::Rect) {
                use kas::Core;
                self.core_data_mut().rect = rect;
                
                #set_rect
            }
        };
        
        (fields, field_ctors, fns)
    }
}
