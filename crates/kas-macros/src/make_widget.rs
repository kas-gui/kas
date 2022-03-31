// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{ChildType, Handler, MakeWidget, WidgetAttrArgs};
use impl_tools_lib::{
    fields::{Field, Fields, FieldsNamed},
    Scope, ScopeItem,
};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, TokenStreamExt};
use std::fmt::Write;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{AttrStyle, Attribute, Ident, ItemImpl, Result, Type, TypePath, Visibility, WhereClause};

pub(crate) fn make_widget(mut args: MakeWidget) -> Result<TokenStream> {
    let mut find_handler_ty_buf: Vec<(Ident, Type)> = vec![];
    // find type of handler's message; return None on error
    let mut find_handler_ty = |handler: &Ident, impls: &Vec<ItemImpl>| -> Type {
        // check the buffer in case we did this already
        for (ident, ty) in &find_handler_ty_buf {
            if ident == handler {
                return ty.clone();
            }
        }

        for impl_ in impls {
            if impl_.trait_.is_some() {
                continue;
            }
            for f in &impl_.items {
                match f {
                    syn::ImplItem::Method(syn::ImplItemMethod { sig, .. })
                        if sig.ident == *handler =>
                    {
                        if sig.inputs.len() != 3 {
                            abort!(
                                sig.span(),
                                "handler functions must have signature: fn handler(&mut self, mgr: &mut EventMgr, msg: T)"
                            );
                        }
                        let arg = sig.inputs.last().unwrap();
                        let ty = match arg {
                            syn::FnArg::Typed(arg) => (*arg.ty).clone(),
                            _ => panic!("expected typed argument"), // nothing else is possible here?
                        };

                        find_handler_ty_buf.push((handler.clone(), ty.clone()));
                        return ty;
                    }
                    _ => (),
                }
            }
        }

        abort!(handler.span(), "no methods with this name found");
    };

    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    let mut make_ident = move |args: std::fmt::Arguments| -> Ident {
        name_buf.clear();
        name_buf.write_fmt(args).unwrap();
        Ident::new(&name_buf, Span::call_site())
    };

    let core_ident: Ident = parse_quote! { core };

    // fields of anonymous struct:
    let mut fields = Punctuated::<Field, Comma>::new();
    fields.push_value(Field {
        attrs: vec![Attribute {
            pound_token: Default::default(),
            style: AttrStyle::Outer,
            bracket_token: Default::default(),
            path: parse_quote! { widget_core },
            tokens: Default::default(),
        }],
        vis: Visibility::Inherited,
        ident: Some(core_ident),
        colon_token: Default::default(),
        ty: parse_quote! { ::kas::CoreData },
        assign: None,
    });
    fields.push_punct(Default::default());

    // initialisers for these fields:
    let mut field_val_toks = quote! {
        core: Default::default(),
    };

    let mut impl_handler = false;
    let mut opt_msg = None;
    let msg = if let Some(msg) = args.attr_widget.msg.as_ref() {
        impl_handler = true;
        msg
    } else {
        // A little magic: try to deduce parameters, applying defaults otherwise
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
                                break;
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        if let Some(msg) = opt_msg.as_ref() {
            msg
        } else {
            // We could default to msg=VoidMsg here. If error messages weren't
            // so terrible this might even be a good idea!
            abort!(
                args.token.span,
                "make_widget: cannot discover msg type from #[handler] attr or Handler impl"
            );
        }
    };

    if args.generics.where_clause.is_none() {
        args.generics.where_clause = Some(WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
    }
    let clauses = &mut args.generics.where_clause.as_mut().unwrap().predicates;

    for (index, pair) in args.fields.into_pairs().enumerate() {
        let (field, opt_comma) = pair.into_tuple();

        let ident = match field.ident {
            Some(ident) => ident,
            None => make_ident(format_args!("mw_anon_{}", index)),
        };

        let widget_attr = if let Some(attr) = field
            .attrs
            .iter()
            .find(|attr| (attr.path == parse_quote! { widget }))
        {
            Some(syn::parse2::<WidgetAttrArgs>(attr.tokens.clone())?)
        } else {
            None
        };

        let ty: Type = match field.ty {
            ChildType::Fixed(ty) => ty.clone(),
            ChildType::InternGeneric(gen_args, ty) => {
                args.generics.params.extend(gen_args);
                ty.clone()
            }
            ChildType::Generic(gen_msg, gen_bound) => {
                let ty = make_ident(format_args!("MWAnon{}", index));

                if let Some(ref wattr) = widget_attr {
                    if let Some(tyr) = gen_msg {
                        clauses.push(parse_quote! { #ty: ::kas::Widget<Msg = #tyr> });
                    } else if let Some(handler) = wattr.handler.any_ref() {
                        // Message passed to a method; exact type required
                        let ty_bound = find_handler_ty(handler, &args.impls);
                        clauses.push(parse_quote! { #ty: ::kas::Widget<Msg = #ty_bound> });
                    } else if wattr.handler == Handler::Discard {
                        // No type bound on discarded message
                    } else {
                        // Message converted via Into
                        clauses
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
        field_val_toks.append_all(quote! { #ident: #value, });

        fields.push_value(Field {
            attrs: field.attrs,
            vis: field.vis,
            ident: Some(ident),
            colon_token: field.colon_token.or_else(|| Some(Default::default())),
            ty,
            assign: None,
        });
        if let Some(comma) = opt_comma {
            fields.push_punct(comma);
        }
    }

    if clauses.is_empty() {
        args.generics.where_clause = None;
    }
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

    if impl_handler {
        // This cannot go in Scope::generated since ImplWidget checks for it
        args.impls.push(parse_quote! {
            impl #impl_generics ::kas::event::Handler
            for AnonWidget #ty_generics
            #where_clause
            {
                type Msg = #msg;
            }
        });
    }

    args.attrs.insert(0, parse_quote! { #[derive(Debug)] });

    let mut scope = Scope {
        attrs: args.attrs,
        vis: Visibility::Inherited,
        ident: parse_quote! { AnonWidget },
        generics: args.generics,
        item: ScopeItem::Struct {
            token: args.token,
            fields: Fields::Named(FieldsNamed {
                brace_token: args.brace_token,
                fields,
            }),
        },
        semi: None,
        impls: args.impls,
        generated: vec![],
    };
    crate::widget::widget(args.attr_widget, &mut scope)?;
    scope.expand_impl_self();

    let toks = quote! { {
        #scope

        AnonWidget {
            #field_val_toks
        }
    } };

    Ok(toks)
}
