// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::autoimpl::{Error, ImplArgs, ImplTrait, Result};
use impl_tools_lib::{generics::clause_to_toks, SimplePath};
use proc_macro2::TokenStream as Toks;
use quote::{quote, TokenStreamExt};
use syn::ItemStruct;

pub const CLASS_IMPLS: &[&dyn ImplTrait] = &[&ImplHasBool, &ImplHasStr, &ImplHasString];

pub struct ImplClassTraits;
impl ImplTrait for ImplClassTraits {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["class_traits"])
    }

    fn support_ignore(&self) -> bool {
        false
    }

    fn support_using(&self) -> bool {
        true
    }

    fn struct_impl(&self, item: &ItemStruct, args: &ImplArgs) -> Result<Toks> {
        let type_ident = &item.ident;
        let (impl_generics, ty_generics, item_wc) = item.generics.split_for_impl();

        let mut toks = Toks::new();

        for trait_ in CLASS_IMPLS {
            let (path, items) = trait_.struct_items(item, args)?;
            let wc = clause_to_toks(&args.clause, item_wc, &path);

            toks.append_all(quote! {
                impl #impl_generics #path for #type_ident #ty_generics #wc {
                    #items
                }
            });
        }

        Ok(toks)
    }

    fn struct_items(&self, _: &ItemStruct, _: &ImplArgs) -> Result<(Toks, Toks)> {
        Err(Error::CallSite("unimplemented"))
    }
}

pub struct ImplHasBool;
impl ImplTrait for ImplHasBool {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "class", "HasBool"])
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
                fn get_bool(&self) -> bool {
                    self.#using.get_bool()
                }

                #[inline]
                fn set_bool(&mut self, state: bool) -> ::kas::Action {
                    self.#using.set_bool(state)
                }
            };
            Ok((quote! { ::kas::class::HasBool }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}

pub struct ImplHasStr;
impl ImplTrait for ImplHasStr {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "class", "HasStr"])
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
                fn get_str(&self) -> &str {
                    self.#using.get_str()
                }

                #[inline]
                fn get_string(&self) -> String {
                    self.#using.get_string()
                }
            };
            Ok((quote! { ::kas::class::HasStr }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}

pub struct ImplHasString;
impl ImplTrait for ImplHasString {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "class", "HasString"])
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
                fn set_str(&mut self, text: &str) -> ::kas::Action {
                    self.#using.set_str(text)
                }

                #[inline]
                fn set_string(&mut self, text: String) -> ::kas::Action {
                    self.#using.set_string(text)
                }
            };
            Ok((quote! { ::kas::class::HasString }, methods))
        } else {
            Err(Error::RequireUsing)
        }
    }
}

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
