// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{Handler, Widget};
use crate::extend_generics;
use proc_macro2::TokenStream;
use proc_macro_error::{emit_call_site_warning, emit_error, emit_warning};
use quote::{quote, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{parse_quote, Result};

pub(crate) fn widget(mut args: Widget) -> Result<TokenStream> {
    let mut toks = quote! { #args };

    let name = &args.ident;
    for impl_ in &mut args.extra_impls {
        if impl_.self_ty == parse_quote! { Self } {
            let (_, ty_generics, _) = args.generics.split_for_impl();
            impl_.self_ty = parse_quote! { #name #ty_generics };
            extend_generics(&mut impl_.generics, &args.generics);
        }
    }

    let opt_derive = &args.attr_widget.derive;

    let mut impl_widget_children = true;
    let mut impl_widget_config = true;
    let mut has_find_id_impl = args.attr_widget.layout.is_some();
    let mut handler_impl = None;
    let mut send_event_impl = None;
    for (index, impl_) in args.extra_impls.iter().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::WidgetChildren }
                || *path == parse_quote! { kas::WidgetChildren }
                || *path == parse_quote! { WidgetChildren }
            {
                if opt_derive.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(derive=FIELD)]"
                    );
                }
                impl_widget_children = false;
            } else if *path == parse_quote! { ::kas::WidgetConfig }
                || *path == parse_quote! { kas::WidgetConfig }
                || *path == parse_quote! { WidgetConfig }
            {
                if opt_derive.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(derive=FIELD)]"
                    );
                }
                // TODO: if args.widget_attr.config.is_some() { warn unused }
                impl_widget_config = false;
            } else if *path == parse_quote! { ::kas::Layout }
                || *path == parse_quote! { kas::Layout }
                || *path == parse_quote! { Layout }
            {
                if args.attr_widget.layout.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(layout=...;)]"
                    );
                }
                for item in &impl_.items {
                    if let syn::ImplItem::Method(method) = item {
                        if method.sig.ident == "layout" || method.sig.ident == "find_id" {
                            has_find_id_impl = true;
                        }
                    }
                }
            } else if *path == parse_quote! { ::kas::event::Handler }
                || *path == parse_quote! { kas::event::Handler }
                || *path == parse_quote! { event::Handler }
                || *path == parse_quote! { Handler }
            {
                if opt_derive.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(derive=FIELD)]"
                    );
                }
                // TODO: warn about unused handler stuff if present
                handler_impl = Some(index);
            } else if *path == parse_quote! { ::kas::event::SendEvent }
                || *path == parse_quote! { kas::event::SendEvent }
                || *path == parse_quote! { event::SendEvent }
                || *path == parse_quote! { SendEvent }
            {
                if opt_derive.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(derive=FIELD)]"
                    );
                }
                send_event_impl = Some(index);
            }
        }
    }

    if !has_find_id_impl && (!impl_widget_children || !args.children.is_empty()) {
        emit_call_site_warning!("widget appears to have children yet does not implement Layout::layout or Layout::find_id; this may cause incorrect handling of mouse/touch events");
    }

    let (mut impl_generics, ty_generics, mut where_clause) = args.generics.split_for_impl();
    let widget_name = name.to_string();

    let (core_data, core_data_mut) = args
        .core_data
        .as_ref()
        .map(|cd| (quote! { &self.#cd }, quote! { &mut self.#cd }))
        .unwrap_or_else(|| {
            let inner = opt_derive.as_ref().unwrap();
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

    if let Some(inner) = opt_derive {
        toks.append_all(quote! {
            impl #impl_generics ::kas::WidgetChildren
                for #name #ty_generics #where_clause
            {
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
        let key_nav = args.attr_widget.key_nav.value;
        let hover_highlight = args.attr_widget.hover_highlight.value;
        let cursor_icon = args.attr_widget.cursor_icon.value;

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
    } else {
        if let Some(span) = args.attr_widget.key_nav.span {
            emit_warning!(span, "unused due to manual impl of `WidgetConfig`");
        }
        if let Some(span) = args.attr_widget.hover_highlight.span {
            emit_warning!(span, "unused due to manual impl of `WidgetConfig`");
        }
        if let Some(span) = args.attr_widget.cursor_icon.span {
            emit_warning!(span, "unused due to manual impl of `WidgetConfig`");
        }
    }

    if let Some(inner) = opt_derive {
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
                fn translation(&self) -> ::kas::geom::Offset {
                    self.#inner.translation()
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
                fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                    self.#inner.find_id(coord)
                }
                #[inline]
                fn draw(
                    &mut self,
                    draw: &mut dyn ::kas::draw::DrawHandle,
                    mgr: &::kas::event::ManagerState,
                    disabled: bool,
                ) {
                    self.#inner.draw(draw, mgr, disabled);
                }
            }
        });
    } else if let Some(layout) = args.attr_widget.layout.take() {
        let find_id = match args.attr_widget.find_id.value {
            None => quote! {},
            Some(find_id) => quote! {
                fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                    if !self.rect().contains(coord) {
                        return None;
                    }
                    #find_id
                }
            },
        };

        let core = args.core_data.as_ref().unwrap();
        let layout = layout.generate(args.children.iter().map(|c| &c.ident))?;

        toks.append_all(quote! {
            impl #impl_generics ::kas::Layout for #name #ty_generics #where_clause {
                fn layout<'a>(&'a mut self) -> ::kas::layout::Layout<'a> {
                    use ::kas::WidgetCore;
                    let mut _chain = &mut self.#core.layout;
                    #layout
                }

                #find_id
            }
        });
    } else if let Some(span) = args.attr_widget.find_id.span {
        emit_warning!(span, "unused without generated impl of `Layout`");
    }

    if let Some(index) = handler_impl {
        // Manual Handler impl may add additional bounds:
        let (a, _, c) = args.extra_impls[index].generics.split_for_impl();
        impl_generics = a;
        where_clause = c;
    } else {
        let handler = args.attr_handler.unwrap_or_default();
        let msg = handler.msg;
        let handle = if let Some(inner) = opt_derive {
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
        let send_impl = if let Some(inner) = opt_derive {
            quote! { self.#inner.send(mgr, id, event) }
        } else {
            let mut ev_to_num = TokenStream::new();
            for (i, child) in args.children.iter().enumerate() {
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
                    Some(#i) => {
                        let r = self.#ident.send(mgr, id, event);
                        #update
                        #handler
                    }
                });
            }

            quote! {
                use ::kas::{WidgetCore, event::Response};
                if self.is_disabled() {
                    return Response::Unhandled;
                }

                let self_id = self.id();
                match self_id.index_of_child(id) {
                    #ev_to_num
                    _ if id == self_id => ::kas::event::Manager::handle_generic(self, mgr, event),
                    _ => {
                        debug_assert!(false, "SendEvent::send: bad WidgetId");
                        Response::None
                    }
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

    for impl_ in &mut args.extra_impls {
        toks.append_all(quote! {
            #impl_
        });
    }

    Ok(toks)
}
