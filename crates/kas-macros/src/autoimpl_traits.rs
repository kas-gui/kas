// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::SimplePath;
use impl_tools_lib::autoimpl::{Error, ImplArgs, ImplTrait, Result};
use proc_macro2::TokenStream as Toks;
use quote::quote;
use syn::ItemStruct;

pub struct ImplLayout;
impl ImplTrait for ImplLayout {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "Layout"])
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
                fn rect(&self) -> ::kas::geom::Rect {
                    self.#using.rect()
                }
                #[inline]
                fn size_rules(&mut self, cx: &mut ::kas::theme::SizeCx, axis: ::kas::layout::AxisInfo) -> ::kas::layout::SizeRules {
                    self.#using.size_rules(cx, axis)
                }
                #[inline]
                fn set_rect(&mut self, cx: &mut ::kas::theme::SizeCx, rect: ::kas::geom::Rect, hints: ::kas::layout::AlignHints) {
                    self.#using.set_rect(cx, rect, hints);
                }
                #[inline]
                fn draw(&self, draw: ::kas::theme::DrawCx) {
                    self.#using.draw(draw);
                }
            };
            Ok((quote! { ::kas::Layout }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}

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
                fn set_offset(
                    &mut self,
                    cx: &mut ::kas::theme::SizeCx,
                    viewport: ::kas::geom::Rect,
                    offset: ::kas::geom::Offset,
                ) {
                    self.#using.set_offset(cx, viewport, offset)
                }
                #[inline]
                fn update_offset(
                    &mut self,
                    cx: &mut ::kas::event::ConfigCx,
                    data: &Self::Data,
                    viewport: ::kas::geom::Rect,
                    offset: ::kas::geom::Offset,
                ) {
                    self.#using.update_offset(cx, data, viewport, offset)
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

/// List of all Kas trait implementations
pub const KAS_IMPLS: &[&dyn ImplTrait] = &[&ImplLayout, &ImplViewport];
