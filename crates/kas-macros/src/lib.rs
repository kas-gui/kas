// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![recursion_limit = "128"]
#![allow(clippy::let_and_return)]

extern crate proc_macro;

use self::args::{ChildType, Handler};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::proc_macro_error;
use proc_macro_error::{abort, emit_error};
use quote::{quote, TokenStreamExt};
use std::fmt::Write;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote};
use syn::{GenericParam, Generics, Ident, ItemImpl, Type, TypePath, WhereClause, WherePredicate};

mod args;
mod layout;

// Support impls on Self by replacing name and summing generics
fn extend_generics(generics: &mut Generics, in_generics: &Generics) {
    if generics.lt_token.is_none() {
        debug_assert!(generics.params.is_empty());
        debug_assert!(generics.gt_token.is_none());
        generics.lt_token = in_generics.lt_token.clone();
        generics.params = in_generics.params.clone();
        generics.gt_token = in_generics.gt_token.clone();
    } else if in_generics.lt_token.is_none() {
        debug_assert!(in_generics.params.is_empty());
        debug_assert!(in_generics.gt_token.is_none());
    } else {
        if !generics.params.empty_or_trailing() {
            generics.params.push_punct(Default::default());
        }
        generics
            .params
            .extend(in_generics.params.clone().into_pairs());
    }

    // Strip defaults which are legal on the struct but not on impls
    for param in &mut generics.params {
        match param {
            GenericParam::Type(p) => {
                p.eq_token = None;
                p.default = None;
            }
            GenericParam::Lifetime(_) => (),
            GenericParam::Const(p) => {
                p.eq_token = None;
                p.default = None;
            }
        }
    }

    if let Some(ref mut clause1) = generics.where_clause {
        if let Some(ref clause2) = in_generics.where_clause {
            if !clause1.predicates.empty_or_trailing() {
                clause1.predicates.push_punct(Default::default());
            }
            clause1
                .predicates
                .extend(clause2.predicates.clone().into_pairs());
        }
    } else {
        generics.where_clause = in_generics.where_clause.clone();
    }
}

/// Macro to derive widget traits
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-widget-macro).
#[proc_macro_error]
#[proc_macro]
pub fn widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut args = parse_macro_input!(input as args::Widget);
    let mut toks = quote! { #args };

    let name = &args.ident;
    for impl_ in &mut args.extra_impls {
        if impl_.self_ty == parse_quote! { Self } {
            let (_, ty_generics, _) = args.generics.split_for_impl();
            impl_.self_ty = parse_quote! { #name #ty_generics };
            extend_generics(&mut impl_.generics, &args.generics);
        }
    }

    let derive_inner = args.core_data.is_none();
    let opt_inner = args.inner.as_ref().map(|(inner, _)| inner.clone());

    let mut impl_widget_children = true;
    let mut impl_widget_config = true;
    let mut handler_impl = None;
    let mut send_event_impl = None;
    for (index, impl_) in args.extra_impls.iter().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::WidgetChildren }
                || *path == parse_quote! { kas::WidgetChildren }
                || *path == parse_quote! { WidgetChildren }
            {
                if derive_inner {
                    emit_error!(impl_.span(), "impl conflicts with use of widget_derive");
                }
                impl_widget_children = false;
            } else if *path == parse_quote! { ::kas::WidgetConfig }
                || *path == parse_quote! { kas::WidgetConfig }
                || *path == parse_quote! { WidgetConfig }
            {
                if derive_inner {
                    emit_error!(impl_.span(), "impl conflicts with use of widget_derive");
                }
                // TODO: if args.widget_attr.config.is_some() { warn unused }
                impl_widget_config = false;
            } else if *path == parse_quote! { ::kas::event::Handler }
                || *path == parse_quote! { kas::event::Handler }
                || *path == parse_quote! { event::Handler }
                || *path == parse_quote! { Handler }
            {
                if derive_inner {
                    emit_error!(impl_.span(), "impl conflicts with use of widget_derive");
                }
                // TODO: warn about unused handler stuff if present
                handler_impl = Some(index);
            } else if *path == parse_quote! { ::kas::event::SendEvent }
                || *path == parse_quote! { kas::event::SendEvent }
                || *path == parse_quote! { event::SendEvent }
                || *path == parse_quote! { SendEvent }
            {
                if derive_inner {
                    emit_error!(impl_.span(), "impl conflicts with use of widget_derive");
                }
                send_event_impl = Some(index);
            }
        }
    }

    let (mut impl_generics, ty_generics, mut where_clause) = args.generics.split_for_impl();
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
    toks.append_all(quote! {
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
    });

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
    } else if impl_widget_children {
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

    if impl_widget_config {
        let config = args.attr_widget.config.unwrap_or_default();
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
    } else if let Some(ref layout) = args.attr_layout {
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

    if let Some(index) = handler_impl {
        // Manual Handler impl may add additional bounds:
        let (a, _, c) = args.extra_impls[index].generics.split_for_impl();
        impl_generics = a;
        where_clause = c;
    } else {
        let handler = args.attr_handler.unwrap_or_default();
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

    if let Some(index) = send_event_impl {
        // Manual SendEvent impl may add additional bounds:
        let (a, _, c) = args.extra_impls[index].generics.split_for_impl();
        impl_generics = a;
        where_clause = c;
    } else {
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
                        if matches!(r, Response::Update) {
                            self.#f(mgr);
                        }
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

    if let Some((member, ty)) = args.inner {
        if args.attr_derive.deref {
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

        if args.attr_derive.deref_mut {
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

        if args.attr_derive.has_bool {
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

        if args.attr_derive.has_str {
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

        if args.attr_derive.has_string {
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

        if args.attr_derive.set_accel {
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

    for impl_ in &mut args.extra_impls {
        toks.append_all(quote! {
            #impl_
        });
    }

    toks.into()
}

/// Macro to create a widget with anonymous type
///
/// See documentation [in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-make_widget-macro).
#[proc_macro_error]
#[proc_macro]
pub fn make_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
                            return quote! {}.into(); // exit after emitting error
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
    let toks = (quote! { {
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
    } })
    .into();

    toks
}

/// Macro to derive `From<VoidMsg>`
///
/// See documentation [ in the `kas::macros` module](https://docs.rs/kas/latest/kas/macros#the-derivevoidmsg-macro).
#[proc_macro_error]
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
