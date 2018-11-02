#![recursion_limit="128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use std::env;
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::DeriveInput;
use syn::{parse_quote, parse_macro_input};


mod args;

#[cfg(not(feature = "cassowary"))] mod layout_extern;
#[cfg(not(feature = "cassowary"))] use self::layout_extern as layout;

#[cfg(feature = "cassowary")] mod layout_cw;
#[cfg(feature = "cassowary")] use self::layout_cw as layout;

/// Macro to derive widget traits
/// 
/// Unlike normal derive macros, this one implements multiple traits. [`Core`]
/// is always derived; other traits are optional.
/// 
/// One struct field must be marked with `#[core]` and implement the [`Core`]
/// trait. It is recommended to use the [`CoreData`] type.
/// 
/// If there is a `#[widget(...)]` attribute on the struct (in addition to the
/// `#[derive(Widget)]` attribute), then the [`Widget`] trait will be
/// implemented. All child widgets must be a field marked with `#[widget]`.
/// The `#[widget(...)]` attribute on the struct itself supports the following
/// parameters:
/// 
/// -   `class = ...` (required) — an expression yielding the widget's [`Class`]
/// -   `label = ...`(optional) — an expression yielding the widget's [`label`]
/// 
/// Example:
/// 
/// ```notest
/// #[widget(class = Class::Window)]
/// #[derive(Widget)]
/// pub struct SimpleWindow<W: Widget> {
///     #[core] core: CoreData,
///     min_size: Coord,
///     #[widget] w: W
/// }
/// ```
/// 
/// Note: usage of this macro currently requires `#![feature(unrestricted_attribute_tokens)]`.
/// 
/// [`label`]: ../mygui/widget/trait.Widget.html#tymethod.label
/// [`Class`]: ../mygui/widget/enum.Class.html
/// [`Core`]: ../mygui/widget/trait.Core.html
/// [`CoreData`]: ../mygui/widget/struct.CoreData.html
/// [`Widget`]: ../mygui/widget/trait.Widget.html
#[proc_macro_derive(Widget, attributes(core, layout, widget))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    
    // Our stand-in for $crate. Imperfect, but works (excepting other crates in
    // the same package, i.e. doc-tests, examples, integration tests, benches).
    let c = if env::var("CARGO_PKG_NAME") == Ok("mygui".to_string()) {
        quote!( crate )
    } else {
        quote!( mygui )
    };
    
    let args = match args::read_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let mut toks = TokenStream::default();
    
    if true {
        let core = args.core;
        toks.append_all(quote! {
            impl #impl_generics #c::widget::Core
                for #name #ty_generics #where_clause
            {
                fn number(&self) -> u32 {
                    self.#core.number()
                }
                fn set_number(&mut self, number: u32) {
                    self.#core.set_number(number);
                }
                
                fn tkd(&self) -> #c::toolkit::TkData {
                    self.#core.tkd()
                }
                fn set_tkd(&mut self, tkd: #c::toolkit::TkData) {
                    self.#core.set_tkd(tkd)
                }
                
                fn rect(&self) -> &#c::widget::Rect {
                    self.#core.rect()
                }
                fn rect_mut(&mut self) -> &mut #c::widget::Rect {
                    self.#core.rect_mut()
                }
            }
        });
    }
    
    if let Some(layout) = args.layout {
        let fns = match layout::fns(&c, &args.children, layout) {
            Ok(fns) => fns,
            Err(err) => return err.to_compile_error().into(),
        };
        
        toks.append_all(quote! {
            impl #impl_generics #c::widget::Layout
                    for #name #ty_generics #where_clause
            {
                #fns
            }
        });
    }
    
    if let Some(widget) = args.widget {
        let class = widget.class;
        let label = widget.label.unwrap_or_else(|| parse_quote!{ None });
        let count = args.children.len();
        
        fn make_match_rules(children: &Vec<args::Child>, mut_ref: TokenStream) -> TokenStream {
            let mut toks = TokenStream::new();
            for (i, child) in children.iter().enumerate() {
                let ident = &child.ident;
                toks.append_all(quote!{ #i => Some(&#mut_ref self.#ident), });
            }
            toks
        };
        let get_rules = make_match_rules(&args.children, quote!{});
        let get_mut_rules = make_match_rules(&args.children, quote!{mut});
        
        toks.append_all(quote! {
            impl #impl_generics #c::widget::Widget
                    for #name #ty_generics #where_clause
            {
                fn class(&self) -> #c::widget::Class { #class }
                fn label(&self) -> Option<&str> { #label }

                fn len(&self) -> usize {
                    #count
                }
                fn get(&self, _index: usize) -> Option<&#c::widget::Widget> {
                    match _index {
                        #get_rules
                        _ => None
                    }
                }
                fn get_mut(&mut self, _index: usize) -> Option<&mut #c::widget::Widget> {
                    match _index {
                        #get_mut_rules
                        _ => None
                    }
                }
            }
        });
    }
    
    toks.into()
}
