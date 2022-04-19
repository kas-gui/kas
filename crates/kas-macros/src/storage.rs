// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use impl_tools_lib::autoimpl::{ImplArgs, ImplTrait, Result};
use impl_tools_lib::SimplePath;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub struct ImplStorage;
impl ImplTrait for ImplStorage {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["", "kas", "layout", "Storage"])
    }

    fn support_ignore(&self) -> bool {
        false
    }

    fn support_using(&self) -> bool {
        false
    }

    fn struct_items(&self, _: &ItemStruct, _: &ImplArgs) -> Result<TokenStream> {
        Ok(quote! {
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
                self
            }
        })
    }
}
