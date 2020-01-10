// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![recursion_limit = "128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

mod args;

use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use std::fmt::Write;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_macro_input, parse_quote};
use syn::{DeriveInput, FnArg, Ident, ImplItemMethod, Type, TypePath};

use self::args::ChildType;

mod layout;

/// Macro to derive widget traits
///
/// See the [`kas::macros`](../kas/macros/index.html) module documentation.
#[proc_macro_derive(Widget, attributes(core, widget, handler, layout_data))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);

    let args = match args::read_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    let widget_name = name.to_string();

    let core = args.core;
    let count = args.children.len();

    let mut get_rules = quote! {};
    let mut get_mut_rules = quote! {};
    let mut walk_rules = quote! {};
    let mut walk_mut_rules = quote! {};
    for (i, child) in args.children.iter().enumerate() {
        let ident = &child.ident;
        get_rules.append_all(quote! { #i => Some(&self.#ident), });
        get_mut_rules.append_all(quote! { #i => Some(&mut self.#ident), });
        walk_rules.append_all(quote! { self.#ident.walk(f); });
        walk_mut_rules.append_all(quote! { self.#ident.walk_mut(f); });
    }

    let mut toks = quote! {
        impl #impl_generics kas::WidgetCore
            for #name #ty_generics #where_clause
        {
            fn core_data(&self) -> &kas::CoreData {
                &self.#core
            }

            fn core_data_mut(&mut self) -> &mut kas::CoreData {
                &mut self.#core
            }

            fn widget_name(&self) -> &'static str {
                #widget_name
            }

            fn as_widget(&self) -> &dyn kas::Widget { self }
            fn as_widget_mut(&mut self) -> &mut dyn kas::Widget { self }

            fn len(&self) -> usize {
                #count
            }
            fn get(&self, _index: usize) -> Option<&dyn kas::Widget> {
                match _index {
                    #get_rules
                    _ => None
                }
            }
            fn get_mut(&mut self, _index: usize) -> Option<&mut dyn kas::Widget> {
                match _index {
                    #get_mut_rules
                    _ => None
                }
            }
            fn walk(&self, f: &mut dyn FnMut(&dyn kas::Widget)) {
                #walk_rules
                f(self);
            }
            fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn kas::Widget)) {
                #walk_mut_rules
                f(self);
            }
        }
    };

    if let Some(ref layout) = args.widget.layout {
        let (fns, dt) = match layout::derive(&args.children, layout, &args.layout_data) {
            Ok(res) => res,
            Err(err) => return err.to_compile_error().into(),
        };
        toks.append_all(quote! {
            impl #impl_generics kas::Widget
                    for #name #ty_generics #where_clause
            {
                #fns
            }
            impl #impl_generics kas::LayoutData
                    for #name #ty_generics #where_clause
            {
                #dt
            }
        });
    }

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

        let mut ev_to_num = TokenStream::new();
        let mut ev_to_coord = TokenStream::new();
        for child in args.children.iter() {
            let ident = &child.ident;
            let handler = if let Some(ref h) = child.args.handler {
                quote! { r.try_into().unwrap_or_else(|msg| self.#h(_tk, msg)) }
            } else {
                quote! { r.into() }
            };
            // TODO(opt): it is possible to code more efficient search strategies
            ev_to_num.append_all(quote! {
                if id <= self.#ident.id() {
                    let r = self.#ident.handle(_tk, addr, event);
                    #handler
                } else
            });
            ev_to_coord.append_all(quote! {
                if self.#ident.rect().contains(coord) {
                    let r = self.#ident.handle(_tk, addr, event);
                    #handler
                } else
            });
        }

        let handler = if args.children.is_empty() {
            // rely on the default implementation
            quote! {}
        } else {
            quote! {
                fn handle(&mut self, _tk: &mut dyn kas::TkWindow, addr: kas::event::Address, event: kas::event::Event)
                -> kas::event::Response<Self::Msg>
                {
                    use kas::{WidgetCore, event::{Event, Response}};
                    match addr {
                        kas::event::Address::Id(id) => {
                            #ev_to_num {
                                debug_assert!(id == self.id(), "Handler::handle: bad WidgetId");
                                Response::Unhandled(event)
                            }
                        }
                        kas::event::Address::Coord(coord) => {
                            #ev_to_coord {
                                kas::event::Manager::handle_generic(self, _tk, event)
                            }
                        }
                    }
                }
            }
        };

        toks.append_all(quote! {
            impl #impl_generics kas::event::Handler
                    for #name #ty_generics #where_clause
            {
                type Msg = #msg;
                #handler
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
     -> Option<Type> {
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
                        handler
                            .span()
                            .unwrap()
                            .error("multiple methods with this name")
                            .emit();
                        x.0.span()
                            .unwrap()
                            .error("first method with this name")
                            .emit();
                        f.sig
                            .ident
                            .span()
                            .unwrap()
                            .error("second method with this name")
                            .emit();
                        return None;
                    }
                    if f.sig.inputs.len() != 3 {
                        f.sig.span()
                            .unwrap()
                            .error("handler functions must have signature: fn handler(&mut self, tk: &mut dyn TkWindow, msg: T)")
                            .emit();
                        return None;
                    }
                    let arg = f.sig.inputs.last().unwrap();
                    let ty = match arg {
                        FnArg::Typed(arg) => (*arg.ty).clone(),
                        _ => panic!("expected typed argument"), // nothing else is possible here?
                    };
                    x = Some((f.sig.ident.clone(), ty));
                }
            }
        }
        if let Some(x) = x {
            find_handler_ty_buf.push((handler.clone(), x.1.clone()));
            Some(x.1)
        } else {
            handler
                .span()
                .unwrap()
                .error("no methods with this name found")
                .emit();
            None
        }
    };

    let mut args = parse_macro_input!(input as args::MakeWidget);

    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);

    // fields of anonymous struct:
    let mut field_toks = quote! {
        #[core] core: kas::CoreData,
        #[layout_data] layout_data: <Self as kas::LayoutData>::Data,
    };
    // initialisers for these fields:
    let mut field_val_toks = quote! {
        core: Default::default(),
        layout_data: Default::default(),
    };
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

    let layout = args.layout;
    let widget_args = quote! { layout = #layout };

    for (index, field) in args.fields.drain(..).enumerate() {
        let attr = field.widget_attr;

        let ident = match &field.ident {
            Some(ref ident) => ident.clone(),
            None => {
                name_buf.clear();
                name_buf
                    .write_fmt(format_args!("mw_anon_{}", index))
                    .unwrap();
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
                        handler_clauses.push(quote! { #ty: kas::event::Handler<Msg = #tyr> });
                    } else {
                        // No typing. If a handler is specified, then the child must implement
                        // Handler<Msg = X> where the handler takes type X; otherwise
                        // we use `msg.into()` and this conversion must be supported.
                        if let Some(ref handler) = wattr.args.handler {
                            if let Some(ty_bound) = find_handler_ty(handler, &args.impls) {
                                handler_clauses
                                    .push(quote! { #ty: kas::event::Handler<Msg = #ty_bound> });
                            } else {
                                return quote! {}.into(); // exit after emitting error
                            }
                        } else {
                            name_buf.push_str("R");
                            let tyr = Ident::new(&name_buf, Span::call_site());
                            handler_extra.push(tyr.clone());
                            handler_clauses.push(quote! { #ty: kas::event::Handler<Msg = #tyr> });
                            handler_clauses.push(quote! { #msg: From<#tyr> });
                        }
                    }

                    if let Some(mut bound) = gen_bound {
                        bound.bounds.push(parse_quote! { kas::Widget });
                        gen_ptrs.push(quote! { #ty: #bound });
                    } else {
                        gen_ptrs.push(quote! { #ty: kas::Widget });
                    }
                } else {
                    gen_ptrs.push(quote! { #ty });
                }

                Type::Path(TypePath {
                    qself: None,
                    path: ty.into(),
                })
            }
        };

        let value = &field.value;

        field_toks.append_all(quote! { #attr #ident: #ty, });
        field_val_toks.append_all(quote! { #ident: #value, });
        debug_fields
            .append_all(quote! { write!(f, ", {}: {:?}", stringify!(#ident), self.#ident)?; });
    }

    let handler_where = if handler_clauses.is_empty() {
        quote! {}
    } else {
        quote! { where #handler_clauses }
    };

    let mut impls = quote! {};

    for impl_block in args.impls {
        let mut contents = TokenStream::new();
        for method in impl_block.1 {
            contents.append_all(std::iter::once(method));
        }
        let target = if let Some(t) = impl_block.0 {
            quote! { #t for }
        } else {
            quote! {}
        };
        impls.append_all(quote! {
            impl<#gen_ptrs> #target AnonWidget<#gen_tys> {
                #contents
            }
        });
    }

    // TODO: we should probably not rely on recursive macro expansion here!
    // (I.e. use direct code generation for Widget derivation, instead of derive.)
    let toks = (quote! { {
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
    } })
    .into();

    toks
}

/// Macro to derive `From<VoidMsg>`
///
/// See the [`kas::macros`](../kas/macros/index.html) module documentation.
#[proc_macro_derive(VoidMsg)]
pub fn derive_empty_msg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;

    let toks = quote! {
        impl #impl_generics From<kas::event::VoidMsg>
            for #name #ty_generics #where_clause
        {
            fn from(_: kas::event::VoidMsg) -> Self {
                unreachable!()
            }
        }
    };
    toks.into()
}
