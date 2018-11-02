use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::{parse_quote, Path};
use syn::parse::{Error, Result};
use super::{LayoutArgs, Child};

pub(crate) fn fns(c: &TokenStream, children: &Vec<Child>, layout: LayoutArgs)
    -> Result<TokenStream>
{
    let layout: Path = if let Some(l) = layout.layout {
        if l == "single" {
            parse_quote!{ #c::widget::ChildLayout::None }
        } else if l == "horizontal" {
            parse_quote!{ #c::widget::ChildLayout::Horizontal }
        } else if l == "vertical" {
            parse_quote!{ #c::widget::ChildLayout::Vertical }
        } else if l == "grid" {
            parse_quote!{ #c::widget::ChildLayout::Grid }
        } else {
            return Err(Error::new(l.span(),
                "expected one of: single, horizontal, vertical, grid"));
        }
    } else {
        parse_quote!{ #c::widget::ChildLayout::None }
    };
    
    let mut pos_rules = TokenStream::new();
    let mut sync_children = TokenStream::new();
    for (i, child) in children.iter().enumerate() {
        let ident = &child.ident;
        if let Some(pos) = &child.args.pos {
            pos_rules.append_all(quote!{ #i => Some(#pos), });
        }
        sync_children.append_all(quote!{ self.#ident.sync_size(tk); });
    }
    
    Ok(quote! {
        fn child_layout(&self) -> #c::widget::ChildLayout {
            #layout
        }
        
        fn grid_pos(&self, _index: usize) -> Option<#c::widget::GridPos> {
            match _index {
                #pos_rules
                _ => None
            }
        }

        fn sync_size(&mut self, tk: &#c::toolkit::TkWidget) {
            let new_rect = tk.get_rect(self.tkd());
            *self.rect_mut() = new_rect;
            
            #sync_children
        }
    })
}
