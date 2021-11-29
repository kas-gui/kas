// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom version of where clause supporting 'X: trait' bound
//!
//! This is almost a direct copy from the syn crate.

use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{parenthesized, token};
use syn::{BoundLifetimes, Lifetime, Token, Type};
use syn::{PredicateLifetime, TraitBound};

/// A trait or lifetime used as a bound on a type parameter.
pub enum TypeParamBound {
    Trait(TraitBound),
    TraitSubst(Token![trait]),
    Lifetime(Lifetime),
}

/// A `where` clause in a definition: `where T: Deserialize<'de>, D: 'static`.
pub struct WhereClause {
    pub where_token: Token![where],
    pub predicates: Punctuated<WherePredicate, Token![,]>,
}

/// A single predicate in a `where` clause: `T: Deserialize<'de>`.
#[allow(clippy::large_enum_variant)]
pub enum WherePredicate {
    /// A type predicate in a `where` clause: `for<'c> Foo<'c>: Trait<'c>`.
    Type(PredicateType),

    /// A lifetime predicate in a `where` clause: `'a: 'b + 'c`.
    Lifetime(PredicateLifetime),
}

/// A type predicate in a `where` clause: `for<'c> Foo<'c>: Trait<'c>`.
pub struct PredicateType {
    /// Any lifetimes from a `for` binding
    pub lifetimes: Option<BoundLifetimes>,
    /// The type being bounded
    pub bounded_ty: Type,
    pub colon_token: Token![:],
    /// Trait and lifetime bounds (`Clone+Send+'static`)
    pub bounds: Punctuated<TypeParamBound, Token![+]>,
}

mod parsing {
    use super::*;

    impl Parse for TypeParamBound {
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Lifetime) {
                return input.parse().map(TypeParamBound::Lifetime);
            }

            if input.peek(Token![trait]) {
                return input.parse().map(TypeParamBound::TraitSubst);
            }

            if input.peek(token::Paren) {
                let content;
                let paren_token = parenthesized!(content in input);
                let mut bound: TraitBound = content.parse()?;
                bound.paren_token = Some(paren_token);
                return Ok(TypeParamBound::Trait(bound));
            }

            input.parse().map(TypeParamBound::Trait)
        }
    }

    impl Parse for WhereClause {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(WhereClause {
                where_token: input.parse()?,
                predicates: {
                    let mut predicates = Punctuated::new();
                    loop {
                        if input.is_empty()
                            || input.peek(token::Brace)
                            || input.peek(Token![,])
                            || input.peek(Token![;])
                            || input.peek(Token![:]) && !input.peek(Token![::])
                            || input.peek(Token![=])
                        {
                            break;
                        }
                        let value = input.parse()?;
                        predicates.push_value(value);
                        if !input.peek(Token![,]) {
                            break;
                        }
                        let punct = input.parse()?;
                        predicates.push_punct(punct);
                    }
                    predicates
                },
            })
        }
    }

    impl Parse for WherePredicate {
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Lifetime) && input.peek2(Token![:]) {
                Ok(WherePredicate::Lifetime(PredicateLifetime {
                    lifetime: input.parse()?,
                    colon_token: input.parse()?,
                    bounds: {
                        let mut bounds = Punctuated::new();
                        loop {
                            if input.is_empty()
                                || input.peek(token::Brace)
                                || input.peek(Token![,])
                                || input.peek(Token![;])
                                || input.peek(Token![:])
                                || input.peek(Token![=])
                            {
                                break;
                            }
                            let value = input.parse()?;
                            bounds.push_value(value);
                            if !input.peek(Token![+]) {
                                break;
                            }
                            let punct = input.parse()?;
                            bounds.push_punct(punct);
                        }
                        bounds
                    },
                }))
            } else {
                Ok(WherePredicate::Type(PredicateType {
                    lifetimes: input.parse()?,
                    bounded_ty: input.parse()?,
                    colon_token: input.parse()?,
                    bounds: {
                        let mut bounds = Punctuated::new();
                        loop {
                            if input.is_empty()
                                || input.peek(token::Brace)
                                || input.peek(Token![,])
                                || input.peek(Token![;])
                                || input.peek(Token![:]) && !input.peek(Token![::])
                                || input.peek(Token![=])
                            {
                                break;
                            }
                            let value = input.parse()?;
                            bounds.push_value(value);
                            if !input.peek(Token![+]) {
                                break;
                            }
                            let punct = input.parse()?;
                            bounds.push_punct(punct);
                        }
                        bounds
                    },
                }))
            }
        }
    }
}

mod printing_subst {
    use super::*;

    pub trait ToTokensSubst {
        fn to_tokens_subst(&self, tokens: &mut TokenStream, subst: &TokenStream);
    }

    impl ToTokensSubst for WhereClause {
        fn to_tokens_subst(&self, tokens: &mut TokenStream, subst: &TokenStream) {
            if !self.predicates.is_empty() {
                self.where_token.to_tokens(tokens);
                self.predicates.to_tokens_subst(tokens, subst);
            }
        }
    }

    impl<T, P> ToTokensSubst for Punctuated<T, P>
    where
        T: ToTokensSubst,
        P: ToTokens,
    {
        fn to_tokens_subst(&self, tokens: &mut TokenStream, subst: &TokenStream) {
            for pair in self.pairs() {
                pair.value().to_tokens_subst(tokens, subst);
                if let Some(punct) = pair.punct() {
                    punct.to_tokens(tokens);
                }
            }
        }
    }

    impl ToTokensSubst for WherePredicate {
        fn to_tokens_subst(&self, tokens: &mut TokenStream, subst: &TokenStream) {
            match self {
                WherePredicate::Type(ty) => ty.to_tokens_subst(tokens, subst),
                WherePredicate::Lifetime(lt) => lt.to_tokens(tokens),
            }
        }
    }

    impl ToTokensSubst for PredicateType {
        fn to_tokens_subst(&self, tokens: &mut TokenStream, subst: &TokenStream) {
            self.lifetimes.to_tokens(tokens);
            self.bounded_ty.to_tokens(tokens);
            self.colon_token.to_tokens(tokens);
            self.bounds.to_tokens_subst(tokens, subst);
        }
    }

    impl ToTokensSubst for TypeParamBound {
        fn to_tokens_subst(&self, tokens: &mut TokenStream, subst: &TokenStream) {
            match self {
                TypeParamBound::Trait(t) => t.to_tokens(tokens),
                TypeParamBound::TraitSubst(_) => tokens.append_all(quote! { #subst }),
                TypeParamBound::Lifetime(lt) => lt.to_tokens(tokens),
            }
        }
    }
}

pub fn clause_to_toks(
    wc: &Option<WhereClause>,
    item_wc: Option<&syn::WhereClause>,
    subst: TokenStream,
) -> TokenStream {
    use printing_subst::ToTokensSubst;

    match (wc, item_wc) {
        (None, None) => quote! {},
        (Some(wc), None) => {
            let mut toks = quote! {};
            wc.to_tokens_subst(&mut toks, &subst);
            toks
        }
        (None, Some(wc)) => quote! { #wc },
        (Some(wc), Some(item_wc)) => {
            let mut toks = quote! { item_wc };
            if !item_wc.predicates.empty_or_trailing() {
                toks.append_all(quote! { , });
            }
            wc.predicates.to_tokens_subst(&mut toks, &subst);
            toks
        }
    }
}
