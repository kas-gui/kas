// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{ChildType, MakeWidget};
use impl_tools_lib::{
    fields::{Field, Fields, FieldsNamed},
    Scope, ScopeItem,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use std::fmt::Write;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Ident, Result, Type, TypePath, Visibility, WhereClause};

pub(crate) fn make_widget(mut args: MakeWidget) -> Result<TokenStream> {
    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    let mut make_ident = move |args: std::fmt::Arguments| -> Ident {
        name_buf.clear();
        name_buf.write_fmt(args).unwrap();
        Ident::new(&name_buf, Span::call_site())
    };

    let mut fields = Punctuated::<Field, Comma>::new();
    let mut field_val_toks = quote! {};

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

        let is_widget = field
            .attrs
            .iter()
            .any(|attr| (attr.path == parse_quote! { widget }));

        let ty: Type = match field.ty {
            ChildType::Fixed(ty) => ty.clone(),
            ChildType::InternGeneric(gen_args, ty) => {
                args.generics.params.extend(gen_args);
                ty.clone()
            }
            ChildType::Generic(gen_bound) => {
                let ty = make_ident(format_args!("MWAnon{}", index));

                if is_widget {
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

        if let Some(ref value) = field.value {
            field_val_toks.append_all(quote! { #ident: #value, });
        } else {
            field_val_toks.append_all(quote! { #ident: Default::default(), });
        }

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
