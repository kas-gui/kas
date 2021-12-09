// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{ChildType, Handler, MakeWidget};
use crate::extend_generics;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, TokenStreamExt};
use std::fmt::Write;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::{Generics, Ident, ItemImpl, Type, TypePath, WhereClause};

pub(crate) fn make_widget(mut args: MakeWidget) -> TokenStream {
    let mut find_handler_ty_buf: Vec<(Ident, Type)> = vec![];
    // find type of handler's message; return None on error
    let mut find_handler_ty = |handler: &Ident, impls: &Vec<ItemImpl>| -> Option<Type> {
        // check the buffer in case we did this already
        for (ident, ty) in &find_handler_ty_buf {
            if ident == handler {
                return Some(ty.clone());
            }
        }

        let mut x: Option<(Ident, Type)> = None;

        for impl_ in impls {
            if impl_.trait_.is_some() {
                continue;
            }
            for f in &impl_.items {
                match f {
                    syn::ImplItem::Method(syn::ImplItemMethod { sig, .. })
                        if sig.ident == *handler =>
                    {
                        if let Some(_x) = x {
                            abort!(
                                handler.span(), "multiple methods with this name";
                                help = _x.0.span() => "first method with this name";
                                help = sig.ident.span() => "second method with this name";
                            );
                        }
                        if sig.inputs.len() != 3 {
                            abort!(
                                sig.span(),
                                "handler functions must have signature: fn handler(&mut self, mgr: &mut Manager, msg: T)"
                            );
                        }
                        let arg = sig.inputs.last().unwrap();
                        let ty = match arg {
                            syn::FnArg::Typed(arg) => (*arg.ty).clone(),
                            _ => panic!("expected typed argument"), // nothing else is possible here?
                        };
                        x = Some((sig.ident.clone(), ty));
                    }
                    _ => (),
                }
            }
        }
        if let Some(x) = x {
            find_handler_ty_buf.push((handler.clone(), x.1.clone()));
            Some(x.1)
        } else {
            abort!(handler.span(), "no methods with this name found");
        }
    };

    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);

    // fields of anonymous struct:
    let mut field_toks = quote! {
        #[widget_core] core: ::kas::CoreData,
    };
    // initialisers for these fields:
    let mut field_val_toks = quote! {
        core: Default::default(),
    };
    // debug impl
    let mut debug_fields = TokenStream::new();

    let msg;
    let mut handler_generics = Generics::default();
    if let Some(h) = args.handler {
        msg = h.msg;
    } else {
        // A little magic: try to deduce parameters, applying defaults otherwise
        let mut opt_msg = None;
        let msg_ident: Ident = parse_quote! { Msg };
        for impl_ in &args.impls {
            if let Some((_, ref name, _)) = impl_.trait_ {
                if *name == parse_quote! { Handler } || *name == parse_quote! { ::kas::Handler } {
                    for item in &impl_.items {
                        match item {
                            syn::ImplItem::Type(syn::ImplItemType {
                                ref ident, ref ty, ..
                            }) if *ident == msg_ident => {
                                opt_msg = Some(ty.clone());
                                continue;
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        if let Some(m) = opt_msg {
            msg = m;
        } else {
            // We could default to msg=VoidMsg here. If error messages weren't
            // so terrible this might even be a good idea!
            abort!(
                args.struct_span,
                "make_widget: cannot discover msg type from #[handler] attr or Handler impl"
            );
        }
    };

    if handler_generics.where_clause.is_none() {
        handler_generics.where_clause = Some(WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
    }
    let handler_clauses = &mut handler_generics.where_clause.as_mut().unwrap().predicates;

    let extra_attrs = args.extra_attrs;

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
            ChildType::InternGeneric(gen_args, ty) => {
                args.generics.params.extend(gen_args);
                ty.clone()
            }
            ChildType::Generic(gen_msg, gen_bound) => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("MWAnon{}", index)).unwrap();
                let ty = Ident::new(&name_buf, Span::call_site());

                if let Some(ref wattr) = attr {
                    if let Some(tyr) = gen_msg {
                        handler_clauses.push(parse_quote! { #ty: ::kas::Widget<Msg = #tyr> });
                    } else if let Some(handler) = wattr.args.handler.any_ref() {
                        // Message passed to a method; exact type required
                        if let Some(ty_bound) = find_handler_ty(handler, &args.impls) {
                            handler_clauses
                                .push(parse_quote! { #ty: ::kas::Widget<Msg = #ty_bound> });
                        } else {
                            return quote! {}; // exit after emitting error
                        }
                    } else if wattr.args.handler == Handler::Discard {
                        // No type bound on discarded message
                    } else {
                        // Message converted via Into
                        handler_clauses
                            .push(parse_quote! { <#ty as ::kas::event::Handler>::Msg: Into<#msg> });
                    }

                    if let Some(mut bound) = gen_bound {
                        bound.bounds.push(parse_quote! { ::kas::Widget });
                        args.generics.params.push(parse_quote! { #ty: #bound });
                    } else {
                        args.generics
                            .params
                            .push(parse_quote! { #ty: ::kas::Widget });
                    }
                } else {
                    args.generics.params.push(parse_quote! { #ty });
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

    if handler_clauses.is_empty() {
        handler_generics.where_clause = None;
    }

    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

    let mut impl_handler = true;
    let mut impls = quote! {};
    for mut impl_ in args.impls {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::event::Handler }
                || *path == parse_quote! { kas::event::Handler }
                || *path == parse_quote! { event::Handler }
                || *path == parse_quote! { Handler }
            {
                impl_handler = false;
                extend_generics(&mut impl_.generics, &handler_generics);
            }
        }

        impls.append_all(quote! {
            #impl_
        });
    }

    let handler = if impl_handler {
        extend_generics(&mut handler_generics, &args.generics);
        let (handler_generics, _, handler_where_clause) = handler_generics.split_for_impl();

        quote! {
            impl #handler_generics ::kas::event::Handler
            for AnonWidget #ty_generics
            #handler_where_clause
            {
                type Msg = #msg;
            }
        }
    } else {
        quote! {}
    };

    // TODO: we should probably not rely on recursive macro expansion here!
    // (I.e. use direct code generation for Widget derivation, instead of derive.)
    let toks = quote! { {
        ::kas::macros::widget! {
            #[derive(Debug)]
            #extra_attrs
            struct AnonWidget #impl_generics #where_clause {
                #field_toks
            }

            #handler

            #impls
        }

        AnonWidget {
            #field_val_toks
        }
    } };

    toks
}
