// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::SimplePath;
use impl_tools_lib::autoimpl::{Error, ImplArgs, ImplTrait, Result};
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
                fn content_size(&self) -> ::kas::geom::Size {
                    self.#using.content_size()
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
