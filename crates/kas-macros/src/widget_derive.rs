// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::widget::{collect_idents, widget_as_node};
use crate::widget_args::{member, Layout, WidgetArgs};
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::scope::{Scope, ScopeItem};
use proc_macro2::Span;
use proc_macro_error2::{emit_error, emit_warning};
use quote::{quote, ToTokens};
use syn::parse::{Error, Result};
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::ImplItem::Verbatim;
use syn::Type;

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

    if let Some(ref toks) = args.data_ty {
        emit_error!(
            toks, "not supported by #[widget(derive=FIELD)]";
            note = derive_span  => "usage of derive mode";
        )
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
                emit_error!(
                    path, "Widget impl is supported by #[widget(derive=FIELD)]";
                    note = derive_span  => "usage of derive mode";
                );
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

    let required_layout_methods = quote! {
        #[inline]
        fn as_layout(&self) -> &dyn ::kas::Layout {
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
        fn get_child(&self, index: usize) -> Option<&dyn ::kas::Layout> {
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
    let fn_nav_next = quote! {
        #[inline]
        fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
            self.#inner.nav_next(reverse, from)
        }
    };
    let fn_translation = quote! {
        #[inline]
        fn translation(&self) -> ::kas::geom::Offset {
            self.#inner.translation()
        }
    };
    let fn_try_probe_forward = quote! {
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

    let data_ty: Type = 'outer: {
        for (i, field) in fields.iter_mut().enumerate() {
            if *inner == member(i, field.ident.clone()) {
                let ty = &field.ty;
                break 'outer parse_quote! { <#ty as ::kas::Widget>::Data };
            }
        }
        return Err(Error::new(inner.span(), "field not found"));
    };

    // Widget methods are derived. Cost: cannot override any Events methods or translation().
    let fns_as_node = widget_as_node();

    if widget_impl.is_none() {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Widget for #impl_target {
                type Data = #data_ty;
                #fns_as_node

                #[inline]
                fn for_child_node(
                    &mut self,
                    data: &Self::Data,
                    index: usize,
                    closure: Box<dyn FnOnce(::kas::Node<'_>) + '_>,
                ) {
                    self.#inner.for_child_node(data, index, closure)
                }

                fn _configure(
                    &mut self,
                    cx: &mut ::kas::event::ConfigCx,
                    data: &Self::Data,
                    id: ::kas::Id,
                ) {
                    self.#inner._configure(cx, data, id);
                }

                fn _update(
                    &mut self,
                    cx: &mut ::kas::event::ConfigCx,
                    data: &Self::Data,
                ) {
                    self.#inner._update(cx, data);
                }

                fn _send(
                    &mut self,
                    cx: &mut ::kas::event::EventCx,
                    data: &Self::Data,
                    id: ::kas::Id,
                    event: ::kas::event::Event,
                ) -> ::kas::event::IsUsed {
                    self.#inner._send(cx, data, id, event)
                }

                fn _replay(
                    &mut self,
                    cx: &mut ::kas::event::EventCx,
                    data: &Self::Data,
                    id: ::kas::Id,
                ) {
                    self.#inner._replay(cx, data, id);
                }

                fn _nav_next(
                    &mut self,
                    cx: &mut ::kas::event::ConfigCx,
                    data: &Self::Data,
                    focus: Option<&::kas::Id>,
                    advance: ::kas::NavAdvance,
                ) -> Option<::kas::Id> {
                    self.#inner._nav_next(cx, data, focus, advance)
                }
            }
        });
    }

    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let item_idents = collect_idents(layout_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        layout_impl.items.push(Verbatim(required_layout_methods));

        if !has_item("size_rules") {
            layout_impl.items.push(Verbatim(fn_size_rules));
        }

        if !has_item("set_rect") {
            layout_impl.items.push(Verbatim(fn_set_rect));
        }

        if !has_item("nav_next") {
            layout_impl.items.push(Verbatim(fn_nav_next));
        }

        if let Some(ident) = item_idents
            .iter()
            .find_map(|(_, ident)| (*ident == "translation").then_some(ident))
        {
            emit_error!(ident, "method not supported in derive mode");
        } else {
            layout_impl.items.push(Verbatim(fn_translation));
        }

        if has_item("probe") {
            // Use default Layout::try_probe impl
        } else {
            // Use default Layout::probe (unimplemented)
            layout_impl.items.push(Verbatim(fn_try_probe_forward));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "try_probe") {
            if let syn::ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                emit_warning!(
                    f,
                    "Implementations are expected to impl `fn probe`, not `try_probe`"
                );
            }
        }

        if !has_item("draw") {
            layout_impl.items.push(Verbatim(fn_draw));
        }
    } else {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Layout for #impl_target {
                #required_layout_methods
                #fn_size_rules
                #fn_set_rect
                #fn_nav_next
                #fn_translation
                #fn_try_probe_forward
                #fn_draw
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
