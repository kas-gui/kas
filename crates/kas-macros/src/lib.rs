// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![recursion_limit = "128"]
#![allow(clippy::let_and_return)]

extern crate proc_macro;

use self::args::{ChildType, Handler, HandlerArgs};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use std::fmt::Write;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;
use syn::{parse_macro_input, parse_quote};
use syn::{GenericParam, Ident, Type, TypeParam, TypePath, WhereClause, WherePredicate};

mod args;
mod layout;

struct SubstTyGenerics<'a>(&'a syn::Generics, HashMap<Ident, Type>);

// impl copied from syn, with modifications
impl<'a> ToTokens for SubstTyGenerics<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.0.params.is_empty() {
            return;
        }

        <Token![<]>::default().to_tokens(tokens);

        // Print lifetimes before types and consts, regardless of their
        // order in self.params.
        //
        // TODO: ordering rules for const parameters vs type parameters have
        // not been settled yet. https://github.com/rust-lang/rust/issues/44580
        let mut trailing_or_empty = true;
        for param in self.0.params.pairs() {
            if let GenericParam::Lifetime(def) = *param.value() {
                // Leave off the lifetime bounds and attributes
                def.lifetime.to_tokens(tokens);
                param.punct().to_tokens(tokens);
                trailing_or_empty = param.punct().is_some();
            }
        }
        for param in self.0.params.pairs() {
            if let GenericParam::Lifetime(_) = **param.value() {
                continue;
            }
            if !trailing_or_empty {
                <Token![,]>::default().to_tokens(tokens);
                trailing_or_empty = true;
            }
            match *param.value() {
                GenericParam::Lifetime(_) => unreachable!(),
                GenericParam::Type(param) => {
                    if let Some(result) = self.1.get(&param.ident) {
                        result.to_tokens(tokens);
                    } else {
                        param.ident.to_tokens(tokens);
                    }
                }
                GenericParam::Const(param) => {
                    // Leave off the const parameter defaults
                    param.ident.to_tokens(tokens);
                }
            }
            param.punct().to_tokens(tokens);
        }

        <Token![>]>::default().to_tokens(tokens);
    }
}

/// Macro to derive widget traits
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-derivewidget-macro).
#[proc_macro_derive(
    Widget,
    attributes(handler, layout, layout_data, widget, widget_core, widget_derive)
)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as syn::DeriveInput);

    let mut args = match args::read_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };

    let derive_inner = args.core_data.is_none();
    let opt_inner = args.inner.as_ref().map(|(inner, _)| inner.clone());

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    let widget_name = name.to_string();

    let (core_data, core_data_mut) = args
        .core_data
        .map(|cd| (quote! { &self.#cd }, quote! { &mut self.#cd }))
        .unwrap_or_else(|| {
            let inner = opt_inner.as_ref().unwrap();
            (
                quote! { self.#inner.core_data() },
                quote! { self.#inner.core_data_mut() },
            )
        });
    let mut toks = quote! {
        impl #impl_generics ::kas::WidgetCore
            for #name #ty_generics #where_clause
        {
            fn as_any(&self) -> &dyn std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

            fn core_data(&self) -> &::kas::CoreData {
                #core_data
            }

            fn core_data_mut(&mut self) -> &mut ::kas::CoreData {
                #core_data_mut
            }

            fn widget_name(&self) -> &'static str {
                #widget_name
            }

            fn as_widget(&self) -> &dyn ::kas::WidgetConfig { self }
            fn as_widget_mut(&mut self) -> &mut dyn ::kas::WidgetConfig { self }
        }
    };

    if derive_inner {
        let inner = opt_inner.as_ref().unwrap();
        toks.append_all(quote! {
            impl #impl_generics ::kas::WidgetChildren
                for #name #ty_generics #where_clause
            {
                fn first_id(&self) -> ::kas::WidgetId {
                    self.#inner.first_id()
                }
                fn record_first_id(&mut self, id: WidgetId) {
                    self.#inner.record_first_id(id);
                }
                fn num_children(&self) -> usize {
                    self.#inner.num_children()
                }
                fn get_child(&self, index: usize) -> Option<&dyn ::kas::WidgetConfig> {
                    self.#inner.get_child(index)
                }
                fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn ::kas::WidgetConfig> {
                    self.#inner.get_child_mut(index)
                }
            }
        });
    } else if args.widget.children {
        let first_id = if args.children.is_empty() {
            quote! { self.id() }
        } else {
            let ident = &args.children[0].ident;
            quote! { self.#ident.first_id() }
        };

        let count = args.children.len();

        let mut get_rules = quote! {};
        let mut get_mut_rules = quote! {};
        for (i, child) in args.children.iter().enumerate() {
            let ident = &child.ident;
            get_rules.append_all(quote! { #i => Some(&self.#ident), });
            get_mut_rules.append_all(quote! { #i => Some(&mut self.#ident), });
        }

        toks.append_all(quote! {
            impl #impl_generics ::kas::WidgetChildren
                for #name #ty_generics #where_clause
            {
                fn first_id(&self) -> ::kas::WidgetId {
                    #first_id
                }
                fn num_children(&self) -> usize {
                    #count
                }
                fn get_child(&self, _index: usize) -> Option<&dyn ::kas::WidgetConfig> {
                    match _index {
                        #get_rules
                        _ => None
                    }
                }
                fn get_child_mut(&mut self, _index: usize) -> Option<&mut dyn ::kas::WidgetConfig> {
                    match _index {
                        #get_mut_rules
                        _ => None
                    }
                }
            }
        });
    }

    if let Some(config) = args.widget.config {
        let key_nav = config.key_nav;
        let hover_highlight = config.hover_highlight;
        let cursor_icon = config.cursor_icon;

        toks.append_all(quote! {
            impl #impl_generics ::kas::WidgetConfig
                    for #name #ty_generics #where_clause
            {
                fn key_nav(&self) -> bool {
                    #key_nav
                }
                fn hover_highlight(&self) -> bool {
                    #hover_highlight
                }
                fn cursor_icon(&self) -> ::kas::event::CursorIcon {
                    #cursor_icon
                }
            }
        });
    }

    if derive_inner {
        let inner = opt_inner.as_ref().unwrap();
        toks.append_all(quote! {
            impl #impl_generics ::kas::Layout
                    for #name #ty_generics #where_clause
            {
                #[inline]
                fn size_rules(
                    &mut self,
                    size_handle: &mut dyn ::kas::draw::SizeHandle,
                    axis: ::kas::layout::AxisInfo,
                ) -> ::kas::layout::SizeRules {
                    self.#inner.size_rules(size_handle, axis)
                }
                #[inline]
                fn set_rect(
                    &mut self,
                    mgr: &mut ::kas::event::Manager,
                    rect: ::kas::geom::Rect,
                    align: ::kas::layout::AlignHints,
                ) {
                    self.#inner.set_rect(mgr, rect, align);
                }
                #[inline]
                fn translation(&self, child_index: usize) -> ::kas::geom::Offset {
                    self.#inner.translation(child_index)
                }
                #[inline]
                fn spatial_nav(
                    &mut self,
                    mgr: &mut ::kas::event::Manager,
                    reverse: bool,
                    from: Option<usize>,
                ) -> Option<usize> {
                    self.#inner.spatial_nav(mgr, reverse, from)
                }
                #[inline]
                fn find_id(&self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                    self.#inner.find_id(coord)
                }
                #[inline]
                fn draw(
                    &self,
                    draw_handle: &mut dyn ::kas::draw::DrawHandle,
                    mgr: &::kas::event::ManagerState,
                    disabled: bool,
                ) {
                    self.#inner.draw(draw_handle, mgr, disabled);
                }
            }
        });
    } else if let Some(ref layout) = args.layout {
        match layout::data_type(&args.children, layout) {
            Ok(dt) => toks.append_all(quote! {
                impl #impl_generics ::kas::LayoutData
                        for #name #ty_generics #where_clause
                {
                    #dt
                }
            }),
            Err(err) => return err.to_compile_error().into(),
        }

        match layout::derive(&args.children, layout, &args.layout_data) {
            Ok(fns) => toks.append_all(quote! {
                impl #impl_generics ::kas::Layout
                        for #name #ty_generics #where_clause
                {
                    #fns
                }
            }),
            Err(err) => return err.to_compile_error().into(),
        }
    }

    // The following traits are all parametrised over the Handler::Msg type.
    // Usually we only have one instance of this, but we support multiple; in
    // case no `#[handler]` attribute is present, we use a default value.
    if args.handler.is_empty() {
        args.handler.push(Default::default());
    }
    for handler in args.handler.drain(..) {
        let subs = handler.substitutions;
        let mut generics = ast.generics.clone();
        generics.params = generics
            .params
            .into_pairs()
            .filter(|pair| match pair.value() {
                GenericParam::Type(TypeParam { ref ident, .. }) => !subs.contains_key(ident),
                _ => true,
            })
            .collect();
        /* Problem: bounded_ty is too generic with no way to extract the Ident
        if let Some(clause) = &mut generics.where_clause {
            clause.predicates = clause.predicates
                .into_pairs()
                .filter(|pair| match pair.value() {
                    &WherePredicate::Type(PredicateType { ref bounded_ty, .. }) =>
                        subs.iter().all(|pair| &pair.0 != ident),
                    _ => true,
                })
                .collect();
        }
        */
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
        let (impl_generics, _ty, where_clause) = generics.split_for_impl();
        let ty_generics = SubstTyGenerics(&ast.generics, subs);

        if handler.handle {
            let msg = handler.msg;
            let handle = if derive_inner {
                let inner = opt_inner.as_ref().unwrap();
                quote! {
                    #[inline]
                    fn activation_via_press(&self) -> bool {
                        self.#inner.activation_via_press()
                    }
                    #[inline]
                    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
                        self.#inner.handle(mgr, event)
                    }
                }
            } else {
                quote! {}
            };
            toks.append_all(quote! {
                impl #impl_generics ::kas::event::Handler
                        for #name #ty_generics #where_clause
                {
                    type Msg = #msg;
                    #handle
                }
            });
        }

        if handler.send {
            let send_impl = if derive_inner {
                let inner = opt_inner.as_ref().unwrap();
                quote! { self.#inner.send(mgr, id, event) }
            } else {
                let mut ev_to_num = TokenStream::new();
                for child in args.children.iter() {
                    #[cfg(feature = "log")]
                    let log_msg = quote! {
                        log::trace!(
                            "Received by {} from {}: {:?}",
                            self.id(),
                            id,
                            ::kas::util::TryFormat(&msg)
                        );
                    };
                    #[cfg(not(feature = "log"))]
                    let log_msg = quote! {};

                    let ident = &child.ident;
                    let update = if let Some(f) = child.args.update.as_ref() {
                        quote! {
                            self.#f(mgr);
                        }
                    } else {
                        quote! {}
                    };
                    let handler = match &child.args.handler {
                        Handler::Use(f) => quote! {
                            r.try_into().unwrap_or_else(|msg| {
                                #log_msg
                                let _: () = self.#f(mgr, msg);
                                Response::None
                            })
                        },
                        Handler::Map(f) => quote! {
                            r.try_into().unwrap_or_else(|msg| {
                                #log_msg
                                Response::Msg(self.#f(mgr, msg))
                            })
                        },
                        Handler::FlatMap(f) => quote! {
                            r.try_into().unwrap_or_else(|msg| {
                                #log_msg
                                self.#f(mgr, msg)
                            })
                        },
                        Handler::Discard => quote! {
                            r.try_into().unwrap_or_else(|msg| {
                                #log_msg
                                let _ = msg;
                                Response::None
                            })
                        },
                        Handler::None => quote! { r.into() },
                    };

                    ev_to_num.append_all(quote! {
                        if id <= self.#ident.id() {
                            let r = self.#ident.send(mgr, id, event);
                            #update
                            #handler
                        } else
                    });
                }

                quote! {
                    use ::kas::{WidgetCore, event::Response};
                    if self.is_disabled() {
                        return Response::Unhandled;
                    }

                    #ev_to_num {
                        debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
                        ::kas::event::Manager::handle_generic(self, mgr, event)
                    }
                }
            };

            toks.append_all(quote! {
                impl #impl_generics ::kas::event::SendEvent
                        for #name #ty_generics #where_clause
                {
                    fn send(
                        &mut self,
                        mgr: &mut ::kas::event::Manager,
                        id: ::kas::WidgetId,
                        event: ::kas::event::Event
                    ) -> ::kas::event::Response<Self::Msg>
                    {
                        #send_impl
                    }
                }
            });
        }

        toks.append_all(quote! {
            impl #impl_generics ::kas::Widget for #name #ty_generics #where_clause {}
        });
    }

    if let Some((member, ty)) = args.inner {
        if args.derive.deref {
            toks.append_all(quote! {
                impl #impl_generics std::ops::Deref for #name #ty_generics #where_clause {
                    type Target = #ty;
                    #[inline]
                    fn deref(&self) -> &Self::Target {
                        &self.#member
                    }
                }
            });
        }

        if args.derive.deref_mut {
            toks.append_all(quote! {
                impl #impl_generics std::ops::DerefMut for #name #ty_generics #where_clause {
                    #[inline]
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#member
                    }
                }
            });
        }

        let extended_where_clause = move |pred: WherePredicate| {
            if let Some(clause) = where_clause {
                let mut clauses: WhereClause = (*clause).clone();
                clauses.predicates.push_punct(Default::default());
                clauses.predicates.push_value(pred);
                clauses
            } else {
                let mut predicates = Punctuated::new();
                predicates.push_value(pred);
                WhereClause {
                    where_token: Default::default(),
                    predicates,
                }
            }
        };

        if args.derive.has_bool {
            let wc = extended_where_clause(parse_quote! { #ty: ::kas::class::HasBool });
            toks.append_all(quote! {
                impl #impl_generics ::kas::class::HasBool for #name #ty_generics #wc {
                    #[inline]
                    fn get_bool(&self) -> bool {
                        self.#member.get_bool()
                    }

                    #[inline]
                    fn set_bool(&mut self, state: bool) -> ::kas::TkAction {
                        self.#member.set_bool(state)
                    }
                }
            });
        }

        if args.derive.has_str {
            let wc = extended_where_clause(parse_quote! { #ty: ::kas::class::HasStr });
            toks.append_all(quote! {
                impl #impl_generics ::kas::class::HasStr for #name #ty_generics #wc {
                    #[inline]
                    fn get_str(&self) -> &str {
                        self.#member.get_str()
                    }

                    #[inline]
                    fn get_string(&self) -> String {
                        self.#member.get_string()
                    }
                }
            });
        }

        if args.derive.has_string {
            let wc = extended_where_clause(parse_quote! { #ty: ::kas::class::HasString });
            toks.append_all(quote! {
                impl #impl_generics ::kas::class::HasString for #name #ty_generics #wc {
                    #[inline]
                    fn set_str(&mut self, text: &str) -> ::kas::TkAction {
                        self.#member.set_str(text)
                    }

                    #[inline]
                    fn set_string(&mut self, text: String) -> ::kas::TkAction {
                        self.#member.set_string(text)
                    }
                }
            });
        }

        if args.derive.set_accel {
            let wc = extended_where_clause(parse_quote! { #ty: ::kas::class::SetAccel });
            toks.append_all(quote! {
                impl #impl_generics ::kas::class::SetAccel for #name #ty_generics #wc {
                    #[inline]
                    fn set_accel_string(&mut self, accel: AccelString) -> ::kas::TkAction {
                        self.#member.set_accel_string(accel)
                    }
                }
            });
        }
    }

    toks.into()
}

/// Macro to create a widget with anonymous type
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-make_widget-macro).
#[proc_macro]
pub fn make_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut find_handler_ty_buf: Vec<(Ident, Type)> = vec![];
    // find type of handler's message; return None on error
    let mut find_handler_ty = |handler: &Ident,
                               impls: &Vec<(Option<TypePath>, Vec<syn::ImplItem>)>|
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

    let mut args = parse_macro_input!(input as args::MakeWidget);

    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);

    // fields of anonymous struct:
    let mut field_toks = quote! {
        #[widget_core] core: ::kas::CoreData,
        #[layout_data] layout_data: <Self as ::kas::LayoutData>::Data,
    };
    // initialisers for these fields:
    let mut field_val_toks = quote! {
        core: Default::default(),
        layout_data: Default::default(),
    };
    // debug impl
    let mut debug_fields = TokenStream::new();

    let mut handler = if let Some(h) = args.handler {
        h
    } else {
        // A little magic: try to deduce parameters, applying defaults otherwise
        let mut handle = true;
        let mut send = true;
        let mut msg = None;
        let msg_ident: Ident = parse_quote! { Msg };
        for (name, body) in &args.impls {
            if name == &Some(parse_quote! { Handler })
                || name == &Some(parse_quote! { ::kas::Handler })
            {
                handle = false;

                for item in body {
                    match item {
                        syn::ImplItem::Type(syn::ImplItemType {
                            ref ident, ref ty, ..
                        }) if *ident == msg_ident => {
                            msg = Some(ty.clone());
                            continue;
                        }
                        _ => (),
                    }
                }
            } else if name == &Some(parse_quote! { SendEvent })
                || name == &Some(parse_quote! { ::kas::SendEvent })
            {
                send = false;
            }
        }

        if let Some(msg) = msg {
            HandlerArgs::new(msg, handle, send)
        } else {
            // We could default to msg=VoidMsg here. If error messages weren't
            // so terrible this might even be a good idea!
            abort!(
                args.struct_span,
                "make_widget: cannot discover msg type from #[handler] attr or Handler impl"
            );
        }
    };
    let msg = &handler.msg;
    let mut handler_clauses = if let Some(ref clause) = handler.generics.where_clause {
        // Ideally we'd use take() or swap() here, but functionality is limited
        clause.predicates.clone()
    } else {
        Default::default()
    };

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
                    } else {
                        if let Some(ref handler) = wattr.args.handler.any_ref() {
                            // Message passed to a method; exact type required
                            if let Some(ty_bound) = find_handler_ty(handler, &args.impls) {
                                handler_clauses
                                    .push(parse_quote! { #ty: ::kas::Widget<Msg = #ty_bound> });
                            } else {
                                return quote! {}.into(); // exit after emitting error
                            }
                        } else if wattr.args.handler == Handler::Discard {
                            // No type bound on discarded message
                        } else {
                            // Message converted via Into
                            name_buf.push('R');
                            let tyr = Ident::new(&name_buf, Span::call_site());
                            handler
                                .generics
                                .params
                                .push(syn::GenericParam::Type(tyr.clone().into()));
                            handler_clauses.push(parse_quote! { #ty: ::kas::Widget<Msg = #tyr> });
                            handler_clauses.push(parse_quote! { #msg: From<#tyr> });
                        }
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

    if !handler_clauses.is_empty() {
        handler.generics.where_clause = Some(syn::WhereClause {
            where_token: Default::default(),
            predicates: handler_clauses,
        });
    }

    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

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
            impl #impl_generics #target AnonWidget #ty_generics #where_clause {
                #contents
            }
        });
    }

    // TODO: we should probably not rely on recursive macro expansion here!
    // (I.e. use direct code generation for Widget derivation, instead of derive.)
    let toks = (quote! { {
        #[derive(Debug, ::kas::macros::Widget)]
        #handler
        #extra_attrs
        struct AnonWidget #impl_generics #where_clause {
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
/// See documentation [ in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-derivevoidmsg-macro).
#[proc_macro_derive(VoidMsg)]
pub fn derive_empty_msg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;

    let toks = quote! {
        impl #impl_generics From<::kas::event::VoidMsg>
            for #name #ty_generics #where_clause
        {
            fn from(_: ::kas::event::VoidMsg) -> Self {
                unreachable!()
            }
        }
    };
    toks.into()
}
