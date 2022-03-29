// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::autoimpl::{Error, ImplArgs, ImplTrait, Result};
use impl_tools_lib::{generics::clause_to_toks, SimplePath};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::ItemStruct;

pub const CLASS_IMPLS: &[&dyn ImplTrait] =
    &[&ImplHasBool, &ImplHasStr, &ImplHasString, &ImplSetAccel];

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

    fn struct_impl(&self, item: &ItemStruct, args: &ImplArgs) -> Result<TokenStream> {
        let type_ident = &item.ident;
        let (impl_generics, ty_generics, item_wc) = item.generics.split_for_impl();

        let mut toks = TokenStream::new();

        for trait_ in CLASS_IMPLS {
            let path = trait_.path().to_token_stream();
            let wc = clause_to_toks(&args.clause, item_wc, &path);

            let items = trait_.struct_items(item, args)?;

            toks.append_all(quote! {
                impl #impl_generics #path for #type_ident #ty_generics #wc {
                    #items
                }
            });
        }

        Ok(toks)
    }

    fn struct_items(&self, _: &ItemStruct, _: &ImplArgs) -> Result<TokenStream> {
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

    fn struct_items(&self, _: &ItemStruct, args: &ImplArgs) -> Result<TokenStream> {
        if let Some(using) = args.using_member() {
            Ok(quote! {
                #[inline]
                fn get_bool(&self) -> bool {
                    self.#using.get_bool()
                }

                #[inline]
                fn set_bool(&mut self, state: bool) -> ::kas::TkAction {
                    self.#using.set_bool(state)
                }
            })
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

    fn struct_items(&self, _: &ItemStruct, args: &ImplArgs) -> Result<TokenStream> {
        if let Some(using) = args.using_member() {
            Ok(quote! {
                #[inline]
                fn get_str(&self) -> &str {
                    self.#using.get_str()
                }

                #[inline]
                fn get_string(&self) -> String {
                    self.#using.get_string()
                }
            })
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

    fn struct_items(&self, _: &ItemStruct, args: &ImplArgs) -> Result<TokenStream> {
        if let Some(using) = args.using_member() {
            Ok(quote! {
                #[inline]
                fn set_str(&mut self, text: &str) -> ::kas::TkAction {
                    self.#using.set_str(text)
                }

                #[inline]
                fn set_string(&mut self, text: String) -> ::kas::TkAction {
                    self.#using.set_string(text)
                }
            })
        } else {
            Err(Error::RequireUsing)
        }
    }
}

pub struct ImplSetAccel;
impl ImplTrait for ImplSetAccel {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "class", "SetAccel"])
    }

    fn support_ignore(&self) -> bool {
        false
    }

    fn support_using(&self) -> bool {
        true
    }

    fn struct_items(&self, _: &ItemStruct, args: &ImplArgs) -> Result<TokenStream> {
        if let Some(using) = args.using_member() {
            Ok(quote! {
                #[inline]
                fn set_accel_string(&mut self, accel: AccelString) -> ::kas::TkAction {
                    self.#using.set_accel_string(accel)
                }
            })
        } else {
            Err(Error::RequireUsing)
        }
    }
}
