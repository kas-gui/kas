// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{ChildType, ImplSingleton, StructStyle};
use impl_tools_lib::{
    fields::{Field, Fields, FieldsNamed, FieldsUnnamed},
    Scope, ScopeItem,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use std::collections::HashMap;
use std::fmt::Write;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{visit_mut, ConstParam, GenericParam, Lifetime, LifetimeDef, TypeParam};
use syn::{Ident, Member, Result, Type, TypePath, Visibility};

pub(crate) fn impl_singleton(mut args: ImplSingleton) -> Result<TokenStream> {
    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    let mut make_ident = move |args: std::fmt::Arguments, span| -> Ident {
        name_buf.clear();
        name_buf.write_fmt(args).unwrap();
        Ident::new(&name_buf, span)
    };

    let mut fields = Punctuated::<Field, Comma>::new();
    let mut field_val_toks = quote! {};

    for (index, pair) in args.fields.into_pairs().enumerate() {
        let (field, opt_comma) = pair.into_tuple();

        let mut ident = field.ident.clone();
        let ty = &field.ty;
        let field_span = match field.assignment {
            None => quote! { #ty }.span(),
            Some((ref eq, ref expr)) => quote! { #ty #eq #expr }.span(),
        };
        let mem = match args.style {
            StructStyle::Regular(_) => {
                let id =
                    ident.unwrap_or_else(|| make_ident(format_args!("_field{index}"), field_span));
                ident = Some(id.clone());
                Member::Named(id)
            }
            StructStyle::Tuple(_, _) => Member::Unnamed(syn::Index {
                index: index as u32,
                span: field_span,
            }),
            _ => unreachable!(),
        };
        let ty_name = match ident {
            None => format!("_Field{index}"),
            Some(ref id) => {
                let ident = id.to_string();
                let mut buf = "_Field".to_string();
                buf.reserve(ident.len());
                let mut next_upper = true;
                for c in ident.chars() {
                    if c == '_' {
                        next_upper = true;
                        continue;
                    }

                    if next_upper {
                        buf.extend(c.to_uppercase());
                        next_upper = false;
                    } else {
                        buf.push(c);
                    }
                }
                buf
            }
        };

        let is_widget = field
            .attrs
            .iter()
            .any(|attr| (attr.path == parse_quote! { widget }));

        let ty: Type = match field.ty {
            ChildType::Fixed(Type::Infer(infer_token)) => {
                // This is a special case: add ::kas::Widget bound

                let ty = Ident::new(&ty_name, infer_token.span());
                args.generics.params.push(if is_widget {
                    parse_quote! { #ty: ::kas::Widget }
                } else {
                    parse_quote! { #ty }
                });

                Type::Path(TypePath {
                    qself: None,
                    path: ty.into(),
                })
            }
            ChildType::Fixed(mut ty) => {
                struct ReplaceInfers<'a, F: FnMut(std::fmt::Arguments, Span) -> Ident> {
                    index: usize,
                    params: Vec<GenericParam>,
                    make_ident: &'a mut F,
                    ty_name: &'a str,
                }
                let mut replacer = ReplaceInfers {
                    index: 0,
                    params: vec![],
                    make_ident: &mut make_ident,
                    ty_name: &ty_name,
                };

                impl<'a, F: FnMut(std::fmt::Arguments, Span) -> Ident> visit_mut::VisitMut
                    for ReplaceInfers<'a, F>
                {
                    fn visit_type_mut(&mut self, node: &mut Type) {
                        let (span, bounds) = match node {
                            Type::ImplTrait(syn::TypeImplTrait { impl_token, bounds }) => {
                                (impl_token.span, std::mem::take(bounds))
                            }
                            Type::Infer(infer) => (infer.span(), Punctuated::new()),
                            _ => return,
                        };

                        let ident =
                            (self.make_ident)(format_args!("{}{}", self.ty_name, self.index), span);
                        self.index += 1;

                        self.params.push(GenericParam::Type(TypeParam {
                            attrs: vec![],
                            ident: ident.clone(),
                            colon_token: Some(Default::default()),
                            bounds,
                            eq_token: None,
                            default: None,
                        }));

                        *node = Type::Path(TypePath {
                            qself: None,
                            path: ident.into(),
                        });
                    }
                }
                visit_mut::visit_type_mut(&mut replacer, &mut ty);

                args.generics.params.extend(replacer.params);
                ty
            }
            ChildType::InternGeneric(mut ig) => {
                struct RenameUnique(HashMap<Ident, Ident>);
                let mut renames = RenameUnique(HashMap::new());

                for param in &mut ig.params {
                    let ident = match param {
                        GenericParam::Type(TypeParam { ident, .. }) => ident,
                        GenericParam::Lifetime(LifetimeDef {
                            lifetime: Lifetime { ident, .. },
                            ..
                        }) => ident,
                        GenericParam::Const(ConstParam { ident, .. }) => ident,
                    };
                    let from = ident.clone();
                    let to = make_ident(format_args!("{ty_name}{from}"), from.span());
                    *ident = to.clone();
                    renames.0.insert(from, to);
                }
                args.generics.params.extend(ig.params);

                impl visit_mut::VisitMut for RenameUnique {
                    fn visit_ident_mut(&mut self, ident: &mut Ident) {
                        if let Some(repl) = self.0.get(ident) {
                            *ident = repl.clone();
                        }
                    }
                }
                visit_mut::visit_type_mut(&mut renames, &mut ig.ty);
                ig.ty
            }
            ChildType::ImplTrait((impl_token, bound)) => {
                let span = quote! { #impl_token #bound }.span();
                let ty = Ident::new(&ty_name, span);

                args.generics.params.push(parse_quote! { #ty: #bound });

                Type::Path(TypePath {
                    qself: None,
                    path: ty.into(),
                })
            }
        };

        if let Some((_, ref value)) = field.assignment {
            field_val_toks.append_all(quote! { #mem: #value, });
        } else {
            field_val_toks.append_all(quote! { #mem: Default::default(), });
        }

        fields.push_value(Field {
            attrs: field.attrs,
            vis: field.vis,
            ident,
            colon_token: field.colon_token.or_else(|| Some(Default::default())),
            ty,
            assign: None,
        });
        if let Some(comma) = opt_comma {
            fields.push_punct(comma);
        }
    }

    let (fields, semi) = match args.style {
        StructStyle::Unit(semi) => (Fields::Unit, Some(semi)),
        StructStyle::Regular(brace_token) => (
            Fields::Named(FieldsNamed {
                brace_token,
                fields,
            }),
            None,
        ),
        StructStyle::Tuple(paren_token, semi) => (
            Fields::Unnamed(FieldsUnnamed {
                paren_token,
                fields,
            }),
            Some(semi),
        ),
    };

    let mut scope = Scope {
        attrs: args.attrs,
        vis: Visibility::Inherited,
        ident: parse_quote! { _Singleton },
        generics: args.generics,
        item: ScopeItem::Struct {
            token: args.token,
            fields,
        },
        semi,
        impls: args.impls,
        generated: vec![],
    };
    scope.apply_attrs(|path| {
        crate::IMPL_SCOPE_RULES
            .iter()
            .cloned()
            .find(|rule| rule.path().matches(path))
    });
    scope.expand_impl_self();

    let toks = quote! { {
        #scope

        _Singleton {
            #field_val_toks
        }
    } };

    Ok(toks)
}
