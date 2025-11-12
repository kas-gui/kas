// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::SimplePath;
use impl_tools_lib::autoimpl::{Error, ImplArgs, ImplTrait, Result};
use proc_macro2::TokenStream as Toks;
use quote::quote;
use syn::ItemStruct;

pub struct ImplViewport;
impl ImplTrait for ImplViewport {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "Viewport"])
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
                fn content_size(&self) -> ::kas::geom::Size {
                    self.#using.content_size()
                }
                #[inline]
                fn update_offset(
                    &mut self,
                    cx: &mut ::kas::event::EventState,
                    viewport: ::kas::geom::Rect,
                    offset: ::kas::geom::Offset,
                ) {
                    self.#using.update_offset(cx, viewport, offset)
                }
                #[inline]
                fn draw_with_offset(
                    &self,
                    draw: ::kas::theme::DrawCx,
                    viewport: ::kas::geom::Rect,
                    offset: ::kas::geom::Offset,
                ) {
                    self.#using.draw_with_offset(draw, viewport, offset);
                }
                #[inline]
                fn try_probe_with_offset(
                    &self,
                    coord: ::kas::geom::Coord,
                    offset: ::kas::geom::Offset,
                ) -> Option<::kas::Id> {
                    self.#using.try_probe_with_offset(coord, offset)
                }
            };
            Ok((quote! { ::kas::Viewport }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}
