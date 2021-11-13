// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::{Span, TokenStream};
use proc_macro_error::emit_error;
use quote::{quote, quote_spanned, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Field, Fields, Ident, ItemStruct, LitInt, Member, Token, WhereClause};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(on);
    custom_keyword!(skip);
}

/// Traits targetting many fields
#[derive(Clone, Copy)]
enum TraitMany {
    Clone(Span),
    Debug(Span),
}
/// Traits targetting one field
#[derive(Clone, Copy)]
enum TraitOne {
    Deref(Span),
    DerefMut(Span),
    HasBool(Span),
    HasStr(Span),
    HasString(Span),
    SetAccel(Span),
}
#[derive(Clone, Copy)]
enum Class {
    Many(TraitMany),
    One(TraitOne),
}
fn class(ident: &Ident) -> Option<Class> {
    if ident == "Clone" {
        Some(Class::Many(TraitMany::Clone(ident.span())))
    } else if ident == "Debug" {
        Some(Class::Many(TraitMany::Debug(ident.span())))
    } else if ident == "Deref" {
        Some(Class::One(TraitOne::Deref(ident.span())))
    } else if ident == "DerefMut" {
        Some(Class::One(TraitOne::DerefMut(ident.span())))
    } else if ident == "HasBool" {
        Some(Class::One(TraitOne::HasBool(ident.span())))
    } else if ident == "HasStr" {
        Some(Class::One(TraitOne::HasStr(ident.span())))
    } else if ident == "HasString" {
        Some(Class::One(TraitOne::HasString(ident.span())))
    } else if ident == "SetAccel" {
        Some(Class::One(TraitOne::SetAccel(ident.span())))
    } else {
        None
    }
}

enum Body {
    Many {
        targets: Vec<TraitMany>,
        skip: Vec<Member>,
    },
    One {
        targets: Vec<TraitOne>,
        on: Member,
    },
}

pub struct AutoImpl {
    body: Body,
    clause: Option<WhereClause>,
}

impl Parse for AutoImpl {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut first = None;
        let mut targets_many = Vec::new();
        let mut targets_one = Vec::new();
        let mut clause = None;
        let mut on = None;
        let mut skip = Vec::new();
        let mut empty_or_trailing = true;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![where]) || lookahead.peek(kw::on) || lookahead.peek(kw::skip) {
                break;
            }

            if empty_or_trailing {
                if lookahead.peek(Ident) {
                    let target = input.parse()?;
                    match class(&target) {
                        Some(class) if first.is_none() => {
                            first = Some(class);
                            match class {
                                Class::Many(trait_) => targets_many.push(trait_),
                                Class::One(trait_) => targets_one.push(trait_),
                            }
                        }
                        Some(Class::Many(trait_)) if matches!(first, Some(Class::Many(_))) => {
                            targets_many.push(trait_);
                        }
                        Some(Class::One(trait_)) if matches!(first, Some(Class::One(_))) => {
                            targets_one.push(trait_);
                        }
                        Some(_) => {
                            return Err(Error::new(
                                target.span(),
                                "incompatible: traits targetting a single field and traits targetting multiple fields may not be derived simultaneously",
                            ));
                        }
                        None => {
                            return Err(Error::new(target.span(), "unsupported trait"));
                        }
                    }
                    empty_or_trailing = false;
                    continue;
                }
            } else if input.peek(Comma) {
                let _ = input.parse::<Comma>()?;
                empty_or_trailing = true;
                continue;
            }
            return Err(lookahead.error());
        }

        let mut lookahead = input.lookahead1();
        if lookahead.peek(Token![where]) {
            clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        if matches!(first, Some(Class::One(_))) {
            let _: kw::on = input.parse()?;
            on = Some(input.parse()?);
            lookahead = input.lookahead1();
        } else if lookahead.peek(kw::skip) {
            let _: kw::skip = input.parse()?;
            skip.push(input.parse()?);
            empty_or_trailing = false;
            while !input.is_empty() {
                let lookahead = input.lookahead1();
                if empty_or_trailing {
                    if lookahead.peek(Ident) || lookahead.peek(LitInt) {
                        skip.push(input.parse()?);
                        empty_or_trailing = false;
                        continue;
                    }
                } else if lookahead.peek(Comma) {
                    let _ = input.parse::<Comma>()?;
                    empty_or_trailing = true;
                    continue;
                }
                return Err(lookahead.error());
            }
        }

        if !input.is_empty() {
            return Err(lookahead.error());
        }

        let body = if matches!(first, Some(Class::One(_))) {
            Body::One {
                targets: targets_one,
                on: on.unwrap(),
            }
        } else {
            Body::Many {
                targets: targets_many,
                skip,
            }
        };

        Ok(AutoImpl { body, clause })
    }
}

pub fn autoimpl(attr: AutoImpl, mut item: ItemStruct) -> TokenStream {
    if let Some(x) = attr.clause {
        if let Some(ref mut y) = item.generics.where_clause {
            if !y.predicates.empty_or_trailing() {
                y.predicates.push_punct(Default::default());
            }
            y.predicates.extend(x.predicates.into_pairs());
        } else {
            item.generics.where_clause = Some(x);
        }
    }

    fn check_is_field(mem: &Member, fields: &Fields) {
        match (fields, mem) {
            (Fields::Named(fields), Member::Named(ref ident)) => {
                if fields
                    .named
                    .iter()
                    .any(|field| field.ident.as_ref() == Some(ident))
                {
                    return;
                }
            }
            (Fields::Unnamed(fields), Member::Unnamed(index)) => {
                if (index.index as usize) < fields.unnamed.len() {
                    return;
                }
            }
            _ => (),
        }
        emit_error!(mem.span(), "not a struct field");
    }
    match &attr.body {
        Body::Many { skip, .. } => {
            for mem in skip {
                check_is_field(mem, &item.fields);
            }
        }
        Body::One { on, .. } => check_is_field(on, &item.fields),
    }

    let mut toks = TokenStream::new();
    match attr.body {
        Body::Many { targets, skip } => autoimpl_many(targets, skip, item, &mut toks),
        Body::One { targets, on } => autoimpl_one(targets, on, item, &mut toks),
    }
    toks
}

fn autoimpl_many(
    mut targets: Vec<TraitMany>,
    skip: Vec<Member>,
    item: ItemStruct,
    toks: &mut TokenStream,
) {
    let no_skips = skip.is_empty();
    let skip = |item: &Member| -> bool { skip.iter().any(|mem| *mem == *item) };
    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    for target in targets.drain(..) {
        match target {
            TraitMany::Clone(span) => {
                let mut inner = quote! {};
                for (i, field) in item.fields.iter().enumerate() {
                    let mem = if let Some(ref id) = field.ident {
                        inner.append_all(quote! { #id: });
                        Member::from(id.clone())
                    } else {
                        Member::from(i)
                    };

                    if skip(&mem) {
                        inner.append_all(quote! { Default::default(), });
                    } else {
                        inner.append_all(quote! { self.#mem.clone(), });
                    }
                }
                let inner = match &item.fields {
                    Fields::Named(_) => quote! { Self { #inner } },
                    Fields::Unnamed(_) => quote! { Self( #inner ) },
                    Fields::Unit => quote! { Self },
                };
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics std::clone::Clone for #ident #ty_generics #where_clause {
                        fn clone(&self) -> Self {
                            #inner
                        }
                    }
                });
            }
            TraitMany::Debug(span) => {
                let name = ident.to_string();
                let mut inner;
                match item.fields {
                    Fields::Named(ref fields) => {
                        inner = quote! { f.debug_struct(#name) };
                        for field in fields.named.iter() {
                            let ident = field.ident.as_ref().unwrap();
                            if !skip(&ident.clone().into()) {
                                let name = ident.to_string();
                                inner.append_all(quote! {
                                    .field(#name, &self.#ident)
                                });
                            }
                        }
                        if no_skips {
                            inner.append_all(quote! { .finish() });
                        } else {
                            inner.append_all(quote! { .finish_non_exhaustive() });
                        };
                    }
                    Fields::Unnamed(ref fields) => {
                        inner = quote! { f.debug_tuple(#name) };
                        for i in 0..fields.unnamed.len() {
                            if !skip(&i.into()) {
                                inner.append_all(quote! {
                                    .field(&self.#i)
                                });
                            }
                        }
                        inner.append_all(quote! { .finish() });
                    }
                    Fields::Unit => {
                        inner = quote! { #name };
                    }
                }
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics std::fmt::Debug for #ident #ty_generics #where_clause {
                        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                            #inner
                        }
                    }
                });
            }
        }
    }
}

fn autoimpl_one(mut targets: Vec<TraitOne>, on: Member, item: ItemStruct, toks: &mut TokenStream) {
    fn for_field<'a, T, F: Fn(&Field) -> T>(fields: &'a Fields, mem: &Member, f: F) -> Option<T> {
        match (fields, mem) {
            (Fields::Named(ref fields), Member::Named(ref ident)) => {
                for field in fields.named.iter() {
                    if field.ident.as_ref() == Some(ident) {
                        return Some(f(field));
                    }
                }
            }
            (Fields::Unnamed(ref fields), Member::Unnamed(index)) => {
                if let Some(field) = fields.unnamed.iter().nth(index.index as usize) {
                    return Some(f(field));
                }
            }
            _ => (),
        }
        None
    }

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    for target in targets.drain(..) {
        match target {
            TraitOne::Deref(span) => {
                let ty = for_field(&item.fields, &on, |field| field.ty.clone()).unwrap();
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics std::ops::Deref for #ident #ty_generics #where_clause {
                        type Target = #ty;
                        fn deref(&self) -> &Self::Target {
                            &self.#on
                        }
                    }
                });
            }
            TraitOne::DerefMut(span) => {
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics std::ops::DerefMut for #ident #ty_generics #where_clause {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.#on
                        }
                    }
                });
            }
            TraitOne::HasBool(span) => {
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics ::kas::class::HasBool for #ident #ty_generics
                        #where_clause
                    {
                        #[inline]
                        fn get_bool(&self) -> bool {
                            self.#on.get_bool()
                        }

                        #[inline]
                        fn set_bool(&mut self, state: bool) -> ::kas::TkAction {
                            self.#on.set_bool(state)
                        }
                    }
                });
            }
            TraitOne::HasStr(span) => {
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics ::kas::class::HasStr for #ident #ty_generics
                        #where_clause
                    {
                        #[inline]
                        fn get_str(&self) -> &str {
                            self.#on.get_str()
                        }

                        #[inline]
                        fn get_string(&self) -> String {
                            self.#on.get_string()
                        }
                    }
                });
            }
            TraitOne::HasString(span) => {
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics ::kas::class::HasString for #ident #ty_generics
                        #where_clause
                    {
                        #[inline]
                        fn set_str(&mut self, text: &str) -> ::kas::TkAction {
                            self.#on.set_str(text)
                        }

                        #[inline]
                        fn set_string(&mut self, text: String) -> ::kas::TkAction {
                            self.#on.set_string(text)
                        }
                    }
                });
            }
            TraitOne::SetAccel(span) => {
                toks.append_all(quote_spanned! {span=>
                    impl #impl_generics ::kas::class::SetAccel for #ident #ty_generics
                        #where_clause
                    {
                        #[inline]
                        fn set_accel_string(&mut self, accel: AccelString) -> ::kas::TkAction {
                            self.#on.set_accel_string(accel)
                        }
                    }
                });
            }
        }
    }
}
