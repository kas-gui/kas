// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::autoimpl::{Error, ImplArgs, ImplTrait, Result};
use impl_tools_lib::SimplePath;
use proc_macro2::TokenStream as Toks;
use quote::quote;
use syn::ItemStruct;

pub struct ImplScrollable;
impl ImplTrait for ImplScrollable {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "Scrollable"])
    }

    fn support_ignore(&self) -> bool {
        false
    }

    fn support_using(&self) -> bool {
        true
    }

    fn struct_items(&self, _: &ItemStruct, args: &ImplArgs) -> Result<(Toks, Toks)> {
        if let Some(using) = args.using_member() {
            let methods = quote! {
                #[inline]
                fn scroll_axes(&self, size: ::kas::geom::Size) -> (bool, bool) {
                    self.#using.scroll_axes(size)
                }
                #[inline]
                fn max_scroll_offset(&self) -> ::kas::geom::Offset {
                    self.#using.max_scroll_offset()
                }
                #[inline]
                fn scroll_offset(&self) -> ::kas::geom::Offset {
                    self.#using.scroll_offset()
                }
                #[inline]
                fn set_scroll_offset(
                    &mut self,
                    cx: &mut ::kas::event::EventCx,
                    offset: ::kas::geom::Offset,
                ) -> ::kas::geom::Offset {
                    self.#using.set_scroll_offset(cx, offset)
                }
            };
            Ok((quote! { ::kas::Scrollable }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}

pub struct ImplHasScrollBars;
impl ImplTrait for ImplHasScrollBars {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "HasScrollBars"])
    }

    fn support_ignore(&self) -> bool {
        false
    }

    fn support_using(&self) -> bool {
        true
    }

    fn struct_items(&self, _: &ItemStruct, args: &ImplArgs) -> Result<(Toks, Toks)> {
        if let Some(using) = args.using_member() {
            let methods = quote! {
                #[inline]
                fn get_mode(&self) -> ::kas::ScrollBarMode {
                    self.#using.get_mode()
                }
                #[inline]
                fn set_mode(&mut self, mode: ::kas::ScrollBarMode) -> ::kas::Action {
                    self.#using.set_mode(mode)
                }
                #[inline]
                fn get_visible_bars(&self) -> (bool, bool) {
                    self.#using.get_visible_bars()
                }
                #[inline]
                fn set_visible_bars(&mut self, bars: (bool, bool)) -> ::kas::Action {
                    self.#using.set_visible_bars(bars)
                }
            };
            Ok((quote! { ::kas::HasScrollBars }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}
