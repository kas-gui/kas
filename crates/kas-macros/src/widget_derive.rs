// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::widget::{collect_idents, widget_as_node};
use crate::widget_args::{member, DataExpr, Layout, WidgetArgs};
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::scope::{Scope, ScopeItem};
use proc_macro2::Span;
use proc_macro_error2::{emit_error, emit_warning};
use quote::{quote, ToTokens};
use syn::parse::{Error, Result};
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::ImplItem::Verbatim;

/// Custom widget definition
///
/// This macro may inject impls and inject items into existing impls.
/// It may also inject code into existing methods such that the only observable
/// behaviour is a panic.
pub fn widget(_attr_span: Span, args: WidgetArgs, scope: &mut Scope) -> Result<()> {
    let derive = args.derive.unwrap();
    let derive_span = proc_macro_error2::SpanRange {
        first: derive.kw.span(),
        last: derive.field.span(),
    }
    .collapse();
    let inner = &derive.field;

    let mut data_ty = if let Some(item) = args.data_ty {
        if args.data_expr.is_none() {
            emit_error!(
                &item, "usage requires `data_expr` definition in derive mode";
                note = derive_span  => "usage of derive mode";
            );
        }
        Some(item.ty)
    } else {
        None
    };
    if let Some(ref item) = args.data_expr {
        if data_ty.is_none() {
            emit_error!(&item, "usage requires `Data` type definition");
        }
    }

    if let Some(ref toks) = args.navigable {
        emit_error!(
            toks, "not supported by #[widget(derive=FIELD)]";
            note = derive_span  => "usage of derive mode";
        )
    }
    if let Some(ref toks) = args.hover_highlight {
        emit_error!(
            toks.span(), "not supported by #[widget(derive=FIELD)]";
            note = derive_span  => "usage of derive mode";
        )
    }
    if let Some(ref toks) = args.cursor_icon {
        emit_error!(
            toks, "not supported by #[widget(derive=FIELD)]";
            note = derive_span  => "usage of derive mode";
        )
    }
    if let Some(Layout { ref kw, .. }) = args.layout {
        emit_error!(
            kw, "not supported by #[widget(derive=FIELD)]";
            note = derive_span  => "usage of derive mode";
        )
    }

    scope.expand_impl_self();
    let name = &scope.ident;

    let mut layout_impl = None;
    let mut tile_impl = None;
    let mut widget_impl = None;

    for (index, impl_) in scope.impls.iter().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::Layout }
                || *path == parse_quote! { kas::Layout }
                || *path == parse_quote! { Layout }
            {
                if layout_impl.is_none() {
                    layout_impl = Some(index);
                }
            } else if *path == parse_quote! { ::kas::Tile }
                || *path == parse_quote! { kas::Tile }
                || *path == parse_quote! { Tile }
            {
                if tile_impl.is_none() {
                    tile_impl = Some(index);
                }
            } else if *path == parse_quote! { ::kas::Events }
                || *path == parse_quote! { kas::Events }
                || *path == parse_quote! { Events }
            {
                emit_warning!(
                    path, "Events impl is not used by #[widget(derive=FIELD)]";
                    note = derive_span  => "usage of derive mode";
                );
            } else if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                if widget_impl.is_none() {
                    widget_impl = Some(index);
                }
            }
        }
    }

    let fields = match &mut scope.item {
        ScopeItem::Struct { token, fields } => match fields {
            Fields::Named(FieldsNamed { fields, .. }) => fields,
            Fields::Unnamed(FieldsUnnamed { fields, .. }) => fields,
            Fields::Unit => {
                let span = scope
                    .semi
                    .map(|semi| semi.span())
                    .and_then(|span| token.span().join(span))
                    .unwrap_or_else(Span::call_site);
                return Err(Error::new(span, "expected struct, not unit struct"));
            }
        },
        item => {
            return Err(syn::Error::new(item.token_span(), "expected struct"));
        }
    };

    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let impl_generics = impl_generics.to_token_stream();
    let impl_target = quote! { #name #ty_generics #where_clause };
    let widget_name = name.to_string();

    let required_tile_methods = quote! {
        #[inline]
        fn as_tile(&self) -> &dyn ::kas::Tile {
            self
        }
        #[inline]
        fn id_ref(&self) -> &::kas::Id {
            self.#inner.id_ref()
        }
        #[inline]
        fn rect(&self) -> ::kas::geom::Rect {
            self.#inner.rect()
        }

        #[inline]
        fn widget_name(&self) -> &'static str {
            #widget_name
        }

        #[inline]
        fn num_children(&self) -> usize {
            self.#inner.num_children()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn ::kas::Tile> {
            self.#inner.get_child(index)
        }
        #[inline]
        fn find_child_index(&self, id: &::kas::Id) -> Option<usize> {
            self.#inner.find_child_index(id)
        }
    };

    let fn_size_rules = quote! {
        #[inline]
        fn size_rules(&mut self,
            sizer: ::kas::theme::SizeCx,
            axis: ::kas::layout::AxisInfo,
        ) -> ::kas::layout::SizeRules {
            self.#inner.size_rules(sizer, axis)
        }
    };
    let fn_set_rect = quote! {
        #[inline]
        fn set_rect(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            rect: ::kas::geom::Rect,
            hints: ::kas::layout::AlignHints,
        ) {
            self.#inner.set_rect(cx, rect, hints);
        }
    };
    let fn_try_probe = quote! {
        #[inline]
        fn try_probe(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
            self.#inner.try_probe(coord)
        }
    };
    let fn_draw = quote! {
        #[inline]
        fn draw(&mut self, draw: ::kas::theme::DrawCx) {
            self.#inner.draw(draw);
        }
    };

    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let item_idents = collect_idents(layout_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        if !has_item("size_rules") {
            layout_impl.items.push(Verbatim(fn_size_rules));
        }

        if !has_item("set_rect") {
            layout_impl.items.push(Verbatim(fn_set_rect));
        }

        if !has_item("try_probe") {
            layout_impl.items.push(Verbatim(fn_try_probe));
        }

        if !has_item("draw") {
            layout_impl.items.push(Verbatim(fn_draw));
        }
    } else {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Layout for #impl_target {
                #fn_size_rules
                #fn_set_rect
                #fn_try_probe
                #fn_draw
            }
        });
    }

    if data_ty.is_none() {
        if let Some(index) = widget_impl {
            let widget_impl = &mut scope.impls[index];
            for item in &widget_impl.items {
                if let syn::ImplItem::Type(ref ty_item) = item {
                    if ty_item.ident == "Data" {
                        data_ty = Some(ty_item.ty.clone());
                        break;
                    }
                }
            }
        }
    }
    let data_ty = if let Some(ty) = data_ty {
        ty
    } else {
        'outer: {
            for (i, field) in fields.iter_mut().enumerate() {
                if *inner == member(i, field.ident.clone()) {
                    let ty = &field.ty;
                    break 'outer parse_quote! { <#ty as ::kas::Widget>::Data };
                }
            }
            return Err(Error::new(inner.span(), "field not found"));
        }
    };
    let map_data = if let Some(DataExpr { ref expr, .. }) = args.data_expr {
        quote! { let data = #expr; }
    } else {
        quote! {}
    };

    // Widget methods are derived. Cost: cannot override any Events methods or translation().
    let fn_as_node = widget_as_node();
    let fn_for_child_node = quote! {
        #[inline]
        fn for_child_node(
            &mut self,
            data: &Self::Data,
            index: usize,
            closure: Box<dyn FnOnce(::kas::Node<'_>) + '_>,
        ) {
            #map_data
            self.#inner.for_child_node(data, index, closure)
        }
    };
    let fn_configure = quote! {
        fn _configure(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
            id: ::kas::Id,
        ) {
            #map_data
            self.#inner._configure(cx, data, id);
        }
    };
    let fn_update = quote! {
        fn _update(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
        ) {
            #map_data
            self.#inner._update(cx, data);
        }
    };
    let fn_send = quote! {
        fn _send(
            &mut self,
            cx: &mut ::kas::event::EventCx,
            data: &Self::Data,
            id: ::kas::Id,
            event: ::kas::event::Event,
        ) -> ::kas::event::IsUsed {
            #map_data
            self.#inner._send(cx, data, id, event)
        }
    };
    let fn_replay = quote! {
        fn _replay(
            &mut self,
            cx: &mut ::kas::event::EventCx,
            data: &Self::Data,
            id: ::kas::Id,
        ) {
            #map_data
            self.#inner._replay(cx, data, id);
        }
    };
    let fn_nav_next = quote! {
        fn _nav_next(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
            focus: Option<&::kas::Id>,
            advance: ::kas::NavAdvance,
        ) -> Option<::kas::Id> {
            #map_data
            self.#inner._nav_next(cx, data, focus, advance)
        }
    };

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        let item_idents = collect_idents(widget_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        if !has_item("Data") {
            widget_impl
                .items
                .push(Verbatim(quote! { type Data = #data_ty; }));
        }

        if !has_item("as_node") {
            widget_impl.items.push(Verbatim(fn_as_node));
        }

        if !has_item("for_child_node") {
            widget_impl.items.push(Verbatim(fn_for_child_node));
        }

        if !has_item("_configure") {
            widget_impl.items.push(Verbatim(fn_configure));
        }

        if !has_item("_update") {
            widget_impl.items.push(Verbatim(fn_update));
        }

        if !has_item("_send") {
            widget_impl.items.push(Verbatim(fn_send));
        }

        if !has_item("_replay") {
            widget_impl.items.push(Verbatim(fn_replay));
        }

        if !has_item("_nav_next") {
            widget_impl.items.push(Verbatim(fn_nav_next));
        }
    } else {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Widget for #impl_target {
                type Data = #data_ty;
                #fn_as_node
                #fn_for_child_node
                #fn_configure
                #fn_update
                #fn_send
                #fn_replay
                #fn_nav_next
            }
        });
    }

    let tile_methods = quote! {
        #[inline]
        fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
            self.#inner.nav_next(reverse, from)
        }
        #[inline]
        fn translation(&self) -> ::kas::geom::Offset {
            self.#inner.translation()
        }
    };

    if let Some(index) = tile_impl {
        let tile_impl = &mut scope.impls[index];
        tile_impl.items.push(Verbatim(required_tile_methods));
        tile_impl.items.push(Verbatim(tile_methods));
    } else {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Tile for #impl_target {
                #required_tile_methods
                #tile_methods
            }
        });
    }

    if let Ok(val) = std::env::var("KAS_DEBUG_WIDGET") {
        if name == val.as_str() {
            println!("{}", scope.to_token_stream());
        }
    }
    Ok(())
}
