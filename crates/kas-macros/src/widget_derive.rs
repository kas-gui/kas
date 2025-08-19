// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::widget::{collect_idents, widget_as_node};
use crate::widget_args::member;
use impl_tools_lib::SimplePath;
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::scope::{Scope, ScopeAttr, ScopeItem};
use proc_macro_error2::{emit_error, emit_warning};
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::ImplItem::Verbatim;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{MacroDelimiter, Meta, Token, parse_quote, parse2};

#[allow(non_camel_case_types)]
mod kw {
    syn::custom_keyword!(Data);
}

#[derive(Debug, Default)]
struct DeriveArgs {
    data_ty: Option<syn::Type>,
}

impl Parse for DeriveArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let data_ty = if !content.is_empty() {
            let _: Token![type] = content.parse()?;
            let _ = content.parse::<kw::Data>()?;
            let _: Token![=] = content.parse()?;
            Some(content.parse()?)
        } else {
            None
        };

        Ok(DeriveArgs { data_ty })
    }
}

pub struct AttrDeriveWidget;
impl ScopeAttr for AttrDeriveWidget {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["derive_widget"])
    }

    fn apply(&self, attr: syn::Attribute, scope: &mut Scope) -> Result<()> {
        let span = attr.span();
        let args = match &attr.meta {
            Meta::Path(_) => DeriveArgs::default(),
            _ => attr.parse_args()?,
        };
        derive_widget(span, args, scope)
    }
}

/// Custom widget definition
///
/// This macro may inject impls and inject items into existing impls.
/// It may also inject code into existing methods such that the only observable
/// behaviour is a panic.
fn derive_widget(attr_span: Span, args: DeriveArgs, scope: &mut Scope) -> Result<()> {
    let mut data_ty = args.data_ty;
    let mut data_binding: Option<syn::Expr> = None;
    let mut inner = None;

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
                emit_warning!(path, "Events impl is not used by #[derive_widget]");
            } else if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                if widget_impl.is_none() {
                    widget_impl = Some(index);
                }

                if data_ty.is_none() {
                    for item in &impl_.items {
                        if let syn::ImplItem::Type(ty_item) = item {
                            if ty_item.ident == "Data" {
                                data_ty = Some(ty_item.ty.clone());
                                break;
                            }
                        }
                    }
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

    for (i, field) in fields.iter_mut().enumerate() {
        let mut other_attrs = Vec::with_capacity(field.attrs.len());
        for attr in field.attrs.drain(..) {
            if *attr.path() == parse_quote! { widget } {
                if inner.is_some() {
                    emit_error!(
                        attr,
                        "`#[derive_widget]` expects `#[widget]` on only one field"
                    );
                    continue;
                }
                inner = Some(member(i, field.ident.clone()));

                match attr.meta {
                    Meta::Path(_) => {
                        if data_ty.is_none() {
                            let ty = &field.ty;
                            data_ty = Some(parse_quote! { <#ty as ::kas::Widget>::Data });
                        }
                    }
                    Meta::List(list) if matches!(&list.delimiter, MacroDelimiter::Paren(_)) => {
                        if data_ty.is_none() {
                            emit_error!(list, "usage requires definition of `type Data`");
                        }
                        data_binding = Some(parse2(list.tokens)?);
                    }
                    Meta::List(list) => {
                        let span = list.delimiter.span().join();
                        return Err(Error::new(span, "expected `#[widget]` or `#[widget(..)]`"));
                    }
                    Meta::NameValue(nv) => {
                        let span = nv.eq_token.span();
                        return Err(Error::new(span, "unexpected"));
                    }
                };
            } else {
                other_attrs.push(attr);
            }
        }
        field.attrs = other_attrs;
    }

    let inner = if let Some(ident) = inner {
        ident
    } else {
        return Err(Error::new(attr_span, "expected `#[widget]` on inner field"));
    };
    let data_ty = data_ty.unwrap();

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
        fn id(&self) -> ::kas::Id {
            self.#inner.id()
        }

        #[inline]
        fn identify(&self) -> ::kas::util::IdentifyWidget<'_> {
            ::kas::util::IdentifyWidget::wrapping(#widget_name, self.#inner.as_tile())
        }

        #[inline]
        fn child_indices(&self) -> ::kas::ChildIndices {
            self.#inner.child_indices()
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

    let fn_rect = quote! {
        #[inline]
        fn rect(&self) -> ::kas::geom::Rect {
            self.#inner.rect()
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
        fn try_probe(&self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
            self.#inner.try_probe(coord)
        }
    };
    let fn_draw = quote! {
        #[inline]
        fn draw(&self, draw: ::kas::theme::DrawCx) {
            self.#inner.draw(draw);
        }
    };

    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let item_idents = collect_idents(layout_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        if !has_item("rect") {
            layout_impl.items.push(Verbatim(fn_rect));
        }

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
                #fn_rect
                #fn_size_rules
                #fn_set_rect
                #fn_try_probe
                #fn_draw
            }
        });
    }

    let map_data = if let Some(ref expr) = data_binding {
        quote! { let data = #expr; }
    } else {
        quote! {}
    };

    // Widget methods are derived. Cost: cannot override any Events methods or translation().
    let fn_as_node = widget_as_node();
    let fn_child_node = quote! {
        #[inline]
        fn child_node<'__n>(
            &'__n mut self,
            data: &'__n Self::Data,
            index: usize,
        ) -> Option<::kas::Node<'__n>> {
            #map_data
            self.#inner.child_node(data, index)
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
            advance: ::kas::event::NavAdvance,
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

        if !has_item("child_node") {
            widget_impl.items.push(Verbatim(fn_child_node));
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
                #fn_child_node
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
        fn translation(&self, index: usize) -> ::kas::geom::Offset {
            self.#inner.translation(index)
        }
        #[inline]
        fn probe(&self, coord: ::kas::geom::Coord) -> ::kas::Id
        where
            Self: Sized,
        {
            self.#inner.probe(coord)
        }
    };

    let fn_role = quote! {
        #[inline]
        fn role(&self, cx: &mut dyn ::kas::RoleCx) -> ::kas::Role<'_> {
            self.#inner.role(cx)
        }
    };

    if let Some(index) = tile_impl {
        let tile_impl = &mut scope.impls[index];
        let item_idents = collect_idents(tile_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        tile_impl.items.push(Verbatim(required_tile_methods));
        tile_impl.items.push(Verbatim(tile_methods));
        if !has_item("role") {
            tile_impl.items.push(Verbatim(fn_role));
        }
    } else {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Tile for #impl_target {
                #required_tile_methods
                #tile_methods
                #fn_role
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
