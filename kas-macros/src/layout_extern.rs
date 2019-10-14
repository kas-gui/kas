// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::Child;
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::{parse_quote, Ident, Path};

pub(crate) fn fns(children: &Vec<Child>, layout: Option<Ident>) -> Result<TokenStream> {
    let layout: Path = if let Some(l) = layout {
        if l == "single" {
            parse_quote! { kas::ChildLayout::None }
        } else if l == "horizontal" {
            parse_quote! { kas::ChildLayout::Horizontal }
        } else if l == "vertical" {
            parse_quote! { kas::ChildLayout::Vertical }
        } else if l == "grid" {
            parse_quote! { kas::ChildLayout::Grid }
        } else {
            return Err(Error::new(
                l.span(),
                "expected one of: single, horizontal, vertical, grid",
            ));
        }
    } else {
        parse_quote! { kas::ChildLayout::None }
    };

    let mut pos_rules = TokenStream::new();
    let mut sync_children = TokenStream::new();
    for (i, child) in children.iter().enumerate() {
        let ident = &child.ident;
        if let Some(pos) = child.args.as_pos() {
            pos_rules.append_all(quote! { #i => Some(#pos), });
        }
        sync_children.append_all(quote! { self.#ident.sync_size(tk); });
    }

    Ok(quote! {
        fn child_layout(&self) -> kas::ChildLayout {
            #layout
        }

        fn grid_pos(&self, _index: usize) -> Option<kas::GridPos> {
            match _index {
                #pos_rules
                _ => None
            }
        }

        fn sync_size(&mut self, tk: &kas::TkWidget) {
            use kas::Core;
            let new_rect = tk.get_rect(self.tkd());
            *self.rect_mut() = new_rect;

            #sync_children
        }
    })
}
