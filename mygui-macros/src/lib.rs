#![recursion_limit="128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

mod args;

use std::env;
use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, TokenStreamExt};
use syn::{DeriveInput, Ident, Path};
use syn::{parse_quote, parse_macro_input};

use self::args::ChildType;

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
    let c = c();
    
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

/// Macro to create a widget with anonymous type
/// 
/// This exists purely to save you some typing. You could instead make your own
/// struct, derive `Widget` (with attributes to enable Core, Layout and Widget
/// implementation), manually implement `event::Handler`, and instantiate an
/// object.
/// 
/// Currently usage of this macro requires `#![feature(proc_macro_hygiene)]`.
#[proc_macro]
pub fn make_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as args::MakeWidget);
    
    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    name_buf.push_str("MWAnon");
    let len = name_buf.len();
    
    let c = c();
    let comma = TokenTree::from(Punct::new(',', Spacing::Alone));
    
    // fields of anonymous struct:
    let mut field_toks = quote!{ #[core] core: CoreData, };
    // initialisers for these fields:
    let mut field_val_toks = quote!{ core: Default::default(), };
    
    // generic types on struct, without constraints:
    let mut gen_tys = TokenStream::new();
    // generic types on struct, with constraints:
    let mut gen_ptrs = TokenStream::new();
    // generic types on handler impl, with constraints:
    let mut gen_response_ptrs = TokenStream::new();
    // where clause on handler impl:
    let mut handler_where = quote!{ where };
    // Has the above been appended to? (Difficult to test directly.)
    let mut have_where = false;
    // per-child-widget result handlering:
    let mut handler_toks = TokenStream::new();
    
    let layout = &args.layout;
    let response = &args.response;
    
    for child in &args.widgets {
        let ident = &child.ident;
        let ty: Path = match &child.ty {
            ChildType::Path(p) => p.clone(),
            cty @ ChildType::Generic |
            cty @ ChildType::Response(_) => {
                name_buf.truncate(len);
                name_buf.push_str(&ident.to_string());
                let ty = Ident::new(&name_buf, Span::call_site());
                
                if !gen_tys.is_empty() {
                    gen_tys.append(comma.clone());
                    gen_ptrs.append(comma.clone());
                    gen_response_ptrs.append(comma.clone());
                }
                
                gen_tys.append_all(quote!{ #ty });
                gen_ptrs.append_all(quote!{ #ty: Widget + 'static });
                
                match cty {
                    ChildType::Generic => {
                        name_buf.push_str("R");
                        let tyr = Ident::new(&name_buf, Span::call_site());
                        gen_response_ptrs.append_all(quote!{
                            #tyr, #ty: Widget + Handler<Response = #tyr>
                        });
                        if have_where {
                            handler_where.append(comma.clone());
                        }
                        handler_where.append_all(quote!{
                            #tyr: From<NoResponse>, #response: From<#tyr>
                        });
                        have_where = true;
                    }
                    ChildType::Response(tyr) => {
                        gen_response_ptrs.append_all(quote!{
                            #ty: Widget + Handler<Response = #tyr>
                        });
                    }
                    _ => unreachable!()
                }
                
                ty.into()
            }
        };
        let value = &child.value;
        
        // TODO: pos
        field_toks.append_all(quote!{ #[widget] #ident: #ty, });
        field_val_toks.append_all(quote!{ #ident: #value, });
        
        let handler = if let Some(ref h) = child.handler {
            quote!{ #h }
        } else {
            quote!{ msg.into() }
        };
        handler_toks.append_all(quote!{
            if num <= self.#ident.number() {
                let msg = self.#ident.handle_action(tk, action, num);
                return #handler;
            }
        });
    }
    
    if !have_where {
        handler_where = TokenStream::new();
    }
    
    for data in &args.fields {
        let ident = &data.0;
        let ty = &data.1;
        let expr = &data.2;
        field_toks.append_all(quote!{ #ident: #ty });
        field_val_toks.append_all(quote!{ #ident: #expr });
    }
    
    let toks = (quote!{ {
        use #c::event::{Action, Handler, ignore};
        use #c::macros::Widget;
        use #c::toolkit::Toolkit;
        use #c::widget::{Class, Core, CoreData, Widget};

        #[layout(#layout)]
        #[widget(class = Class::Container)]
        #[derive(Clone, Debug, Widget)]
        struct L<#gen_ptrs> {
            #field_toks
        }

        impl<#gen_response_ptrs> Handler for L<#gen_tys> #handler_where {
            type Response = #response;
            
            fn handle_action(&mut self, tk: &Toolkit, action: Action, num: u32) -> #response {
                #handler_toks
                
                if num != self.number() {
                    println!("Warning: incorrect widget number");
                }
                ignore(action)  // no actions handled by this widget
            }
        }

        L {
            #field_val_toks
        }
    } }).into();
    toks
}

// Our stand-in for $crate. Imperfect, but works (excepting other crates in
// the same package, i.e. doc-tests, examples, integration tests, benches).
fn c() -> TokenStream {
    if env::var("CARGO_PKG_NAME") == Ok("mygui".to_string()) {
        parse_quote!( crate )
    } else {
        parse_quote!( mygui )
    }
}
