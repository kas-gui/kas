// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![recursion_limit="128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

mod args;

use std::fmt::Write;
use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{DeriveInput, FnArg, Ident, ImplItemMethod, Type, TypePath};
use syn::{parse_quote, parse_macro_input};
    use syn::spanned::Spanned;
use syn::token::Comma;
use syn::punctuated::Punctuated;

use self::args::{Class, ChildType};

#[cfg(not(feature = "cassowary"))] mod layout_extern;
#[cfg(not(feature = "cassowary"))] use self::layout_extern as layout;

#[cfg(feature = "cassowary")] mod layout_cw;
#[cfg(feature = "cassowary")] use self::layout_cw as layout;

/// Macro to derive widget traits
/// 
/// See the [`kas::macros`](../kas/macros/index.html) module documentation.
#[proc_macro_derive(Widget, attributes(core, widget, handler))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    
    let args = match args::read_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let core = args.core;
    let class = args.widget.class;
    let count = args.children.len();
    
    let layout_fns = match layout::fns(&args.children, args.widget.layout) {
        Ok(fns) => fns,
        Err(err) => return err.to_compile_error().into(),
    };
    
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
    
    let mut toks = quote! {
        impl #impl_generics kas::Core
            for #name #ty_generics #where_clause
        {
            fn number(&self) -> u32 {
                use kas::Core;
                self.#core.number()
            }
            fn set_number(&mut self, number: u32) {
                use kas::Core;
                self.#core.set_number(number);
            }
            
            fn tkd(&self) -> kas::TkData {
                use kas::Core;
                self.#core.tkd()
            }
            fn set_tkd(&mut self, tkd: kas::TkData) {
                use kas::Core;
                self.#core.set_tkd(tkd)
            }
            
            fn rect(&self) -> &kas::Rect {
                use kas::Core;
                self.#core.rect()
            }
            fn rect_mut(&mut self) -> &mut kas::Rect {
                use kas::Core;
                self.#core.rect_mut()
            }
        }
        
        impl #impl_generics kas::Layout
                for #name #ty_generics #where_clause
        {
            #layout_fns
        }
        
        impl #impl_generics kas::Widget
                for #name #ty_generics #where_clause
        {
            fn class(&self) -> kas::Class { #class }

            fn len(&self) -> usize {
                #count
            }
            fn get(&self, _index: usize) -> Option<&kas::Widget> {
                match _index {
                    #get_rules
                    _ => None
                }
            }
            fn get_mut(&mut self, _index: usize) -> Option<&mut kas::Widget> {
                match _index {
                    #get_mut_rules
                    _ => None
                }
            }
        }
    };
    
    if let Some(handler) = args.handler {
        let msg = handler.msg;
        let mut generics = ast.generics.clone();
        if !handler.generics.params.is_empty() {
            if !generics.params.empty_or_trailing() {
                generics.params.push_punct(Default::default());
            }
            generics.params.extend(handler.generics.params.into_pairs());
        }
        if let Some(h_clauses) = handler.generics.where_clause {
            if let Some(ref mut clauses) = generics.where_clause {
                if !clauses.predicates.empty_or_trailing() {
                    clauses.predicates.push_punct(Default::default());
                }
                clauses.predicates.extend(h_clauses.predicates.into_pairs());
            } else {
                generics.where_clause = Some(h_clauses);
            }
        }
        // Note: we may have extra generic types used in where clauses, but we
        // don't want these in ty_generics.
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        
        let mut handler_toks = TokenStream::new();
        for child in args.children.iter() {
            let ident = &child.ident;
            let handler = if let Some(ref h) = child.args.handler {
                quote!{ self.#h(_tk, msg) }
            } else {
                quote!{ msg.into() }
            };
            handler_toks.append_all(quote!{
                else if num <= self.#ident.number() {
                    let msg = self.#ident.handle(_tk, action, num);
                    #handler
                }
            });
        }
        
        toks.append_all(quote! {
            impl #impl_generics kas::event::Handler
                    for #name #ty_generics #where_clause
            {
                type Msg = #msg;
                
                fn handle(&mut self, _tk: &mut kas::TkWidget, action: kas::event::Action,
                        num: u32) -> Self::Msg
                {
                    use kas::{Core, event::{Handler, err_unhandled, err_num}};
                    
                    if num == self.number() {
                        // we may want to allow custom handlers on self here?
                        err_unhandled(action)
                    }
                    #handler_toks
                    else {
                        err_num()
                    }
                }
            }
        });
    };
    
    toks.into()
}

/// Macro to create a widget with anonymous type
/// 
/// See the [`kas::macros`](../kas/macros/index.html) module documentation.
/// 
/// Currently usage of this macro requires `#![feature(proc_macro_hygiene)]`.
#[proc_macro]
pub fn make_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut find_handler_ty_buf: Vec<(Ident, Type)> = vec![];
    // find type of handler's message; return None on error
    let mut find_handler_ty = |handler: &Ident,
            impls: &Vec<(Option<TypePath>, Vec<ImplItemMethod>)>|
            -> Option<Type>
    {
        // check the buffer in case we did this already
        for (ident, ty) in &find_handler_ty_buf {
            if ident == handler {
                return Some(ty.clone());
            }
        }
        
        let mut x: Option<(Ident, Type)> = None;
        
        for impl_block in impls {
            for f in &impl_block.1 {
                if f.sig.ident == *handler {
                    if let Some(x) = x {
                        handler.span()
                            .unstable()
                            .error("multiple methods with this name")
                            .emit();
                        x.0.span()
                            .unstable()
                            .error("first method with this name")
                            .emit();
                        f.sig.ident.span()
                            .unstable()
                            .error("second method with this name")
                            .emit();
                        return None;
                    }
                    if f.sig.decl.inputs.len() != 3 {
                        f.sig.span()
                            .unstable()
                            .error("handler functions must have signature: fn handler(&mut self, tk: &mut TkWidget, msg: T)")
                            .emit();
                        return None;
                    }
                    let pair = f.sig.decl.inputs.last().unwrap();
                    let arg = pair.value();
                    let ty = match arg {
                        FnArg::Captured(arg) => arg.ty.clone(),
                        _ => panic!("expected captured argument"),  // nothing else is possible here?
                    };
                    x = Some((f.sig.ident.clone(), ty));
                }
            }
        }
        if let Some(x) = x {
            find_handler_ty_buf.push((handler.clone(), x.1.clone()));
            Some(x.1)
        } else {
            handler.span()
                .unstable()
                .error("no methods with this name found")
                .emit();
            None
        }
    };
    
    let mut args = parse_macro_input!(input as args::MakeWidget);
    
    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    
    // fields of anonymous struct:
    let mut field_toks = quote!{ #[core] core: kas::CoreData, };
    // initialisers for these fields:
    let mut field_val_toks = quote!{ core: Default::default(), };
    // debug impl
    let mut debug_fields = TokenStream::new();
    
    // generic types on struct, without constraints:
    let mut gen_tys = Punctuated::<_, Comma>::new();
    // generic types on struct, with constraints:
    let mut gen_ptrs = Punctuated::<_, Comma>::new();
    // extra generic types and where clause for handler impl
    let mut handler_extra = Punctuated::<_, Comma>::new();
    let mut handler_clauses = Punctuated::<_, Comma>::new();
    
    let msg = &args.msg;
    
    let widget_args = match args.class {
        Class::Container(layout) => quote!{
            class = kas::Class::Container, layout = #layout
        },
        Class::Frame => quote!{
            class = kas::Class::Frame
        },
    };
    
    for (index, field) in args.fields.drain(..).enumerate() {
        let attr = &field.widget_attr;
        
        let ident = match &field.ident {
            Some(ref ident) => ident.clone(),
            None => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("mw_anon_{}", index)).unwrap();
                Ident::new(&name_buf, Span::call_site())
            }
        };
        
        let ty: Type = match field.ty {
            ChildType::Fixed(ty) => ty.clone(),
            ChildType::Generic(gen_msg, gen_bound) => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("MWAnon{}", index)).unwrap();
                let ty = Ident::new(&name_buf, Span::call_site());
                
                gen_tys.push(ty.clone());
                if let Some(ref wattr) = attr {
                    if let Some(tyr) = gen_msg {
                        handler_clauses.push(quote!{ #ty: kas::event::Handler<Msg = #tyr> });
                    } else {
                        // No typing. If a handler is specified, then the child must implement
                        // Handler<Msg = X> where the handler takes type X; otherwise
                        // we use `msg.into()` and this conversion must be supported.
                        if let Some(ref handler) = wattr.args.handler {
                            if let Some(ty_bound) = find_handler_ty(handler, &args.impls) {
                                handler_clauses.push(quote!{ #ty: kas::event::Handler<Msg = #ty_bound> });
                            } else {
                                return quote!{}.into(); // exit after emitting error
                            }
                        } else {
                            name_buf.push_str("R");
                            let tyr = Ident::new(&name_buf, Span::call_site());
                            handler_extra.push(tyr.clone());
                            handler_clauses.push(quote!{ #ty: kas::event::Handler<Msg = #tyr> });
                            handler_clauses.push(quote!{ #tyr: From<kas::event::EmptyMsg> });
                            handler_clauses.push(quote!{ #msg: From<#tyr> });
                        }
                    }
                    
                    if let Some(mut bound) = gen_bound {
                        bound.bounds.push(parse_quote!{ kas::Widget });
                        gen_ptrs.push(quote!{ #ty: #bound });
                    } else {
                        gen_ptrs.push(quote!{ #ty: kas::Widget });
                    }
                } else {
                    gen_ptrs.push(quote!{ #ty });
                }
                
                Type::Path(TypePath { qself: None, path: ty.into() })
            }
        };
        
        let value = &field.value;
        
        field_toks.append_all(quote!{ #attr #ident: #ty, });
        field_val_toks.append_all(quote!{ #ident: #value, });
        debug_fields.append_all(quote!{ write!(f, ", {}: {:?}", stringify!(#ident), self.#ident)?; });
    }
    
    let handler_where = if handler_clauses.is_empty() {
        quote!{}
    } else {
        quote!{ where #handler_clauses }
    };
    
    let mut impls = TokenStream::new();
    for impl_block in args.impls {
        let mut contents = TokenStream::new();
        for method in impl_block.1 {
            contents.append_all(std::iter::once(method));
        }
        let target = if let Some(t) = impl_block.0 {
            quote!{ #t for }
        } else {
            quote!{}
        };
        impls.append_all(quote!{
            impl<#gen_ptrs> #target AnonWidget<#gen_tys> {
                #contents
            }
        });
    };
    
    // TODO: we should probably not rely on recursive macro expansion here!
    // (I.e. use direct code generation for Widget derivation, instead of derive.)
    let toks = (quote!{ {
        #[widget(#widget_args)]
        #[handler(msg = #msg, generics = < #handler_extra > #handler_where)]
        #[derive(Clone, Debug, kas::macros::Widget)]
        struct AnonWidget<#gen_ptrs> {
            #field_toks
        }
        
        #impls

        AnonWidget {
            #field_val_toks
        }
    } }).into();
    
    toks
}

/// Macro to derive `From<EmptyMsg>`
/// 
/// See the [`kas::macros`](../kas/macros/index.html) module documentation.
/// 
/// This macro assumes the type is an enum with a simple variant named `None`.
// TODO: add diagnostics to check against mis-use?
#[proc_macro_derive(EmptyMsg)]
pub fn derive_empty_msg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let toks = quote!{
        impl #impl_generics From<kas::event::EmptyMsg>
            for #name #ty_generics #where_clause
        {
            fn from(_: kas::event::EmptyMsg) -> Self {
                #name::None
            }
        }
    };
    toks.into()
}
