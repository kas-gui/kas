// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{Child, WidgetArgs};
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::{Scope, ScopeAttr, ScopeItem, SimplePath};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{emit_error, emit_warning};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{parse_quote, Error, Ident, Index, Member, Result};

fn member(index: usize, ident: Option<Ident>) -> Member {
    match ident {
        None => Member::Unnamed(Index {
            index: index as u32,
            span: Span::call_site(),
        }),
        Some(ident) => Member::Named(ident),
    }
}

pub struct AttrImplWidget;
impl ScopeAttr for AttrImplWidget {
    fn path(&self) -> SimplePath {
        SimplePath::new(&["widget"])
    }

    fn apply(&self, args: TokenStream, _: Span, scope: &mut Scope) -> Result<()> {
        let attr = syn::parse2(args)?;
        widget(attr, scope)
    }
}

pub fn widget(mut attr: WidgetArgs, scope: &mut Scope) -> Result<()> {
    scope.expand_impl_self();
    let name = &scope.ident;
    let opt_derive = &attr.derive;

    let mut impl_widget_children = true;
    let mut impl_layout = true;
    let mut widget_impl = None;

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

    let mut core_data = None;
    let mut children = Vec::with_capacity(fields.len());
    for (i, field) in fields.iter_mut().enumerate() {
        let mut other_attrs = Vec::with_capacity(field.attrs.len());
        for attr in field.attrs.drain(..) {
            if attr.path == parse_quote! { widget_core } {
                if !attr.tokens.is_empty() {
                    return Err(Error::new(attr.tokens.span(), "unexpected token"));
                }
                if core_data.is_none() {
                    core_data = Some(member(i, field.ident.clone()));
                } else {
                    emit_error!(attr.span(), "multiple fields marked with #[widget_core]");
                }
            } else if attr.path == parse_quote! { widget } {
                if !attr.tokens.is_empty() {
                    return Err(Error::new(attr.tokens.span(), "unexpected token"));
                }
                let ident = member(i, field.ident.clone());
                children.push(Child { ident });
            } else {
                other_attrs.push(attr);
            }
        }
        field.attrs = other_attrs;
    }

    crate::widget_index::visit_impls(&children, &mut scope.impls);

    for (index, impl_) in scope.impls.iter().enumerate() {
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
                if !children.is_empty() {
                    emit_warning!(impl_.span(), "use of `#![widget]` on children with custom `WidgetChildren` implementation");
                }
                impl_widget_children = false;
            } else if *path == parse_quote! { ::kas::Layout }
                || *path == parse_quote! { kas::Layout }
                || *path == parse_quote! { Layout }
            {
                if attr.layout.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(layout=...;)]"
                    );
                }
                impl_layout = false;
            } else if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                if opt_derive.is_some() {
                    emit_error!(
                        impl_.span(),
                        "impl conflicts with use of #[widget(derive=FIELD)]"
                    );
                }
                widget_impl = Some(index);
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let widget_name = name.to_string();

    let (access_core_data, access_core_data_mut);
    if let Some(ref cd) = core_data {
        access_core_data = quote! { &self.#cd };
        access_core_data_mut = quote! { &mut self.#cd };
    } else if let Some(ref inner) = opt_derive {
        access_core_data = quote! { self.#inner.core_data() };
        access_core_data_mut = quote! { self.#inner.core_data_mut() };
    } else {
        return Err(Error::new(
            fields.span(),
            "no field marked with #[widget_core]",
        ));
    }

    scope.generated.push(quote! {
        impl #impl_generics ::kas::WidgetCore
            for #name #ty_generics #where_clause
        {
            fn as_any(&self) -> &dyn std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

            fn core_data(&self) -> &::kas::CoreData {
                #access_core_data
            }

            fn core_data_mut(&mut self) -> &mut ::kas::CoreData {
                #access_core_data_mut
            }

            fn widget_name(&self) -> &'static str {
                #widget_name
            }

            fn as_widget(&self) -> &dyn ::kas::Widget { self }
            fn as_widget_mut(&mut self) -> &mut dyn ::kas::Widget { self }
        }
    });

    if impl_widget_children {
        if let Some(inner) = opt_derive {
            scope.generated.push(quote! {
                impl #impl_generics ::kas::WidgetChildren
                    for #name #ty_generics #where_clause
                {
                    #[inline]
                    fn num_children(&self) -> usize {
                        self.#inner.num_children()
                    }
                    #[inline]
                    fn get_child(&self, index: usize) -> Option<&dyn ::kas::Widget> {
                        self.#inner.get_child(index)
                    }
                    #[inline]
                    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn ::kas::Widget> {
                        self.#inner.get_child_mut(index)
                    }
                    #[inline]
                    fn find_child_index(&self, id: &::kas::WidgetId) -> Option<usize> {
                        self.#inner.find_child_index(id)
                    }
                }
            });
        } else {
            let count = children.len();

            let mut get_rules = quote! {};
            let mut get_mut_rules = quote! {};
            for (i, child) in children.iter().enumerate() {
                let ident = &child.ident;
                get_rules.append_all(quote! { #i => Some(&self.#ident), });
                get_mut_rules.append_all(quote! { #i => Some(&mut self.#ident), });
            }

            scope.generated.push(quote! {
                impl #impl_generics ::kas::WidgetChildren
                    for #name #ty_generics #where_clause
                {
                    fn num_children(&self) -> usize {
                        #count
                    }
                    fn get_child(&self, _index: usize) -> Option<&dyn ::kas::Widget> {
                        match _index {
                            #get_rules
                            _ => None
                        }
                    }
                    fn get_child_mut(&mut self, _index: usize) -> Option<&mut dyn ::kas::Widget> {
                        match _index {
                            #get_mut_rules
                            _ => None
                        }
                    }
                }
            });
        }
    }

    if impl_layout {
        if let Some(inner) = opt_derive {
            scope.generated.push(quote! {
                impl #impl_generics ::kas::Layout
                        for #name #ty_generics #where_clause
                {
                    #[inline]
                    fn layout(&mut self) -> ::kas::layout::Layout<'_> {
                        self.#inner.layout()
                    }
                    #[inline]
                    fn size_rules(&mut self,
                        size_mgr: ::kas::theme::SizeMgr,
                        axis: ::kas::layout::AxisInfo,
                    ) -> ::kas::layout::SizeRules {
                        self.#inner.size_rules(size_mgr, axis)
                    }
                    #[inline]
                    fn set_rect(
                        &mut self,
                        mgr: &mut ::kas::layout::SetRectMgr,
                        rect: ::kas::geom::Rect,
                        align: ::kas::layout::AlignHints,
                    ) {
                        self.#inner.set_rect(mgr, rect, align);
                    }
                    #[inline]
                    fn draw(
                        &mut self,
                        draw: ::kas::theme::DrawMgr,
                    ) {
                        self.#inner.draw(draw);
                    }
                }
            });
        } else if let Some(layout) = attr.layout.take() {
            let core = if let Some(ref cd) = core_data {
                cd
            } else {
                return Err(Error::new(
                    fields.span(),
                    "no field marked with #[widget_core]",
                ));
            };
            let layout = layout.generate(children.iter().map(|c| &c.ident))?;

            scope.generated.push(quote! {
                impl #impl_generics ::kas::Layout for #name #ty_generics #where_clause {
                    fn layout<'a>(&'a mut self) -> ::kas::layout::Layout<'a> {
                        use ::kas::{WidgetCore, layout};
                        let mut _chain = &mut self.#core.layout;
                        #layout
                    }
                }
            });
        }
    }

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        if let Some(item) = attr.key_nav {
            widget_impl.items.push(item);
        }
        if let Some(item) = attr.hover_highlight {
            widget_impl.items.push(item);
        }
        if let Some(item) = attr.cursor_icon {
            widget_impl.items.push(item);
        }
    } else {
        let methods = if let Some(inner) = opt_derive {
            let key_nav = attr
                .key_nav
                .map(|item| item.to_token_stream())
                .unwrap_or_else(|| {
                    quote! {
                        #[inline]
                        fn key_nav(&self) -> bool {
                            self.#inner.key_nav()
                        }
                    }
                });
            let hover_highlight = attr
                .hover_highlight
                .map(|item| item.to_token_stream())
                .unwrap_or_else(|| {
                    quote! {
                        #[inline]
                        fn hover_highlight(&self) -> bool {
                            self.#inner.hover_highlight()
                        }
                    }
                });
            let cursor_icon = attr
                .cursor_icon
                .map(|item| item.to_token_stream())
                .unwrap_or_else(|| {
                    quote! {
                        #[inline]
                        fn cursor_icon(&self) -> ::kas::event::CursorIcon {
                            self.#inner.cursor_icon()
                        }
                    }
                });
            quote! {
                #[inline]
                fn configure_recurse(
                    &mut self,
                    mgr: &mut ::kas::layout::SetRectMgr,
                    id: ::kas::WidgetId,
                ) {
                    self.#inner.configure_recurse(mgr, id);
                }
                #[inline]
                fn configure(&mut self, mgr: &mut ::kas::layout::SetRectMgr) {
                    self.#inner.configure(mgr);
                }
                #key_nav
                #hover_highlight
                #cursor_icon

                #[inline]
                fn translation(&self) -> ::kas::geom::Offset {
                    self.#inner.translation()
                }
                #[inline]
                fn spatial_nav(
                    &mut self,
                    mgr: &mut ::kas::layout::SetRectMgr,
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
                fn handle_event(
                    &mut self,
                    mgr: &mut ::kas::event::EventMgr,
                    event: ::kas::event::Event,
                ) -> ::kas::event::Response {
                    self.#inner.handle_event(mgr, event)
                }
                #[inline]
                fn handle_unused(
                    &mut self,
                    mgr: &mut ::kas::event::EventMgr,
                    index: usize,
                    event: ::kas::event::Event,
                ) -> ::kas::event::Response {
                    self.#inner.handle_unused(mgr, index, event)
                }
                #[inline]
                fn handle_message(
                    &mut self,
                    mgr: &mut ::kas::event::EventMgr,
                    index: usize,
                ) {
                    self.#inner.handle_message(mgr, index);
                }
                #[inline]
                fn handle_scroll(
                    &mut self,
                    mgr: &mut ::kas::event::EventMgr,
                    scroll: ::kas::event::Scroll,
                ) {
                    self.#inner.handle_scroll(mgr, scroll);
                }
            }
        } else {
            let mut toks = TokenStream::new();
            if let Some(item) = attr.key_nav {
                item.to_tokens(&mut toks);
            }
            if let Some(item) = attr.hover_highlight {
                item.to_tokens(&mut toks);
            }
            if let Some(item) = attr.cursor_icon {
                item.to_tokens(&mut toks);
            }
            toks
        };
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Widget
                    for #name #ty_generics #where_clause
            {
                #methods
            }
        });
    }

    Ok(())
}
