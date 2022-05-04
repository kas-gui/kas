// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{Child, WidgetArgs};
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::{Scope, ScopeAttr, ScopeItem, SimplePath};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{emit_error, emit_warning};
use quote::{quote, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{parse2, parse_quote, Error, Ident, ImplItem, Index, ItemImpl, Member, Result, Type};

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

pub fn widget(mut args: WidgetArgs, scope: &mut Scope) -> Result<()> {
    scope.expand_impl_self();
    let name = &scope.ident;
    let opt_derive = &args.derive;

    let mut impl_widget_children = true;
    let mut layout_impl = None;
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

    let mut core_data: Option<Member> = None;
    let mut children = Vec::with_capacity(fields.len());
    for (i, field) in fields.iter_mut().enumerate() {
        if matches!(&field.ty, Type::Macro(mac) if mac.mac == parse_quote!{ widget_core!() }) {
            if let Some(ref cd) = core_data {
                emit_error!(
                    field.ty, "multiple fields of type widget_core!()";
                    note = cd.span() => "previous field of type widget_core!()";
                );
            } else {
                core_data = Some(member(i, field.ident.clone()));
            }

            if let Some((stor_ty, stor_def)) = args.layout.as_ref().and_then(|l| l.storage_fields())
            {
                let name = format!("Kas{}GeneratedCore", name);
                let core_type = Ident::new(&name, Span::call_site());
                scope.generated.push(quote! {
                    #[derive(Debug)]
                    struct #core_type {
                        rect: ::kas::geom::Rect,
                        id: ::kas::WidgetId,
                        #stor_ty
                    }

                    impl Default for #core_type {
                        fn default() -> Self {
                            #core_type {
                                rect: Default::default(),
                                id: Default::default(),
                                #stor_def
                            }
                        }
                    }

                    impl Clone for #core_type {
                        fn clone(&self) -> Self {
                            #core_type {
                                rect: self.rect,
                                .. #core_type::default()
                            }
                        }
                    }
                });
                field.ty = Type::Path(syn::TypePath {
                    qself: None,
                    path: core_type.into(),
                });
            } else {
                field.ty = parse_quote! { ::kas::CoreData };
            }

            continue;
        }

        let mut other_attrs = Vec::with_capacity(field.attrs.len());
        for attr in field.attrs.drain(..) {
            if attr.path == parse_quote! { widget } {
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
                if layout_impl.is_none() {
                    layout_impl = Some(index);
                }
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
                if widget_impl.is_none() {
                    widget_impl = Some(index);
                }
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let widget_name = name.to_string();

    let core_methods;
    if let Some(ref cd) = core_data {
        core_methods = quote! {
            #[inline]
            fn id_ref(&self) -> &::kas::WidgetId {
                &self.#cd.id
            }
            #[inline]
            fn rect(&self) -> ::kas::geom::Rect {
                self.#cd.rect
            }
        };
    } else if let Some(ref inner) = opt_derive {
        core_methods = quote! {
            #[inline]
            fn id_ref(&self) -> &::kas::WidgetId {
                self.#inner.id_ref()
            }
            #[inline]
            fn rect(&self) -> ::kas::geom::Rect {
                self.#inner.rect()
            }
        };
    } else {
        return Err(Error::new(fields.span(), "no field of type widget_core!()"));
    }

    scope.generated.push(quote! {
        impl #impl_generics ::kas::WidgetCore
            for #name #ty_generics #where_clause
        {
            #[inline]
            fn as_any(&self) -> &dyn std::any::Any { self }
            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

            #core_methods

            #[inline]
            fn widget_name(&self) -> &'static str {
                #widget_name
            }

            #[inline]
            fn as_widget(&self) -> &dyn ::kas::Widget { self }
            #[inline]
            fn as_widget_mut(&mut self) -> &mut dyn ::kas::Widget { self }
        }
    });

    if let Some(inner) = opt_derive {
        if impl_widget_children {
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
        }

        if layout_impl.is_none() {
            scope.generated.push(quote! {
                impl #impl_generics ::kas::Layout
                        for #name #ty_generics #where_clause
                {
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
        }

        if widget_impl.is_none() {
            let key_nav = args.key_nav.unwrap_or_else(|| {
                quote! {
                    #[inline]
                    fn key_nav(&self) -> bool {
                        self.#inner.key_nav()
                    }
                }
            });
            let hover_highlight = args.hover_highlight.unwrap_or_else(|| {
                quote! {
                    #[inline]
                    fn hover_highlight(&self) -> bool {
                        self.#inner.hover_highlight()
                    }
                }
            });
            let cursor_icon = args.cursor_icon.unwrap_or_else(|| {
                quote! {
                    #[inline]
                    fn cursor_icon(&self) -> ::kas::event::CursorIcon {
                        self.#inner.cursor_icon()
                    }
                }
            });
            scope.generated.push(quote! {
                impl #impl_generics ::kas::Widget
                        for #name #ty_generics #where_clause
                {
                    #[inline]
                    fn make_child_id(&mut self, index: usize) -> ::kas::WidgetId {
                        self.#inner.make_child_id(index)
                    }
                    #[inline]
                    fn pre_configure(
                        &mut self,
                        mgr: &mut ::kas::layout::SetRectMgr,
                        id: ::kas::WidgetId,
                    ) {
                        self.#inner.pre_configure(mgr, id)
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
            });
        }

        return Ok(());
    }

    let core_data = core_data.unwrap();

    if impl_widget_children {
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

    let mut fn_size_rules = None;
    let mut set_rect = None;
    let mut fn_find_id = None;
    let mut fn_draw = None;
    if let Some(layout) = args.layout.take() {
        let core = core_data.clone().into();
        let layout = layout.generate(&core, children.iter().map(|c| &c.ident))?;
        scope.generated.push(quote! {
            impl #impl_generics ::kas::layout::AutoLayout
                    for #name #ty_generics #where_clause
            {
                fn size_rules(
                    &mut self,
                    size_mgr: ::kas::theme::SizeMgr,
                    axis: ::kas::layout::AxisInfo,
                ) -> ::kas::layout::SizeRules {
                    use ::kas::{WidgetCore, layout};
                    (#layout).size_rules(size_mgr, axis)
                }

                fn set_rect(
                    &mut self,
                    mgr: &mut ::kas::layout::SetRectMgr,
                    rect: ::kas::geom::Rect,
                    align: ::kas::layout::AlignHints,
                ) {
                    use ::kas::{WidgetCore, layout};
                    (#layout).set_rect(mgr, rect, align);
                }

                fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                    use ::kas::{layout, WidgetCore, WidgetExt};
                    (#layout).find_id(coord).or_else(|| Some(self.id()))
                }

                fn draw(&mut self, draw: ::kas::theme::DrawMgr) {
                    use ::kas::{WidgetCore, layout};
                    (#layout).draw(draw);
                }
            }
        });

        fn_size_rules = Some(quote! {
            fn size_rules(
                &mut self,
                size_mgr: ::kas::theme::SizeMgr,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                <Self as ::kas::layout::AutoLayout>::size_rules(self, size_mgr, axis)
            }
        });
        set_rect = Some(quote! {
            <Self as ::kas::layout::AutoLayout>::set_rect(self, mgr, rect, align);
        });
        fn_find_id = Some(quote! {
            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                use ::kas::WidgetCore;
                if !self.rect().contains(coord) {
                    return None;
                }
                let coord = coord + self.translation();
                <Self as ::kas::layout::AutoLayout>::find_id(self, coord)
            }
        });
        fn_draw = Some(quote! {
            fn draw(&mut self, draw: ::kas::theme::DrawMgr) {
                <Self as ::kas::layout::AutoLayout>::draw(self, draw);
            }
        });
    }
    let fn_set_rect = quote! {
        fn set_rect(
            &mut self,
            mgr: &mut ::kas::layout::SetRectMgr,
            rect: ::kas::geom::Rect,
            align: ::kas::layout::AlignHints,
        ) {
            self.#core_data.rect = rect;
            #set_rect
        }
    };

    fn has_method(item_impl: &ItemImpl, name: &str) -> bool {
        item_impl
            .items
            .iter()
            .any(|item| matches!(item, ImplItem::Method(m) if m.sig.ident == name))
    }

    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        if let Some(method) = fn_size_rules {
            if !has_method(&layout_impl, "size_rules") {
                layout_impl.items.push(parse2(method)?);
            }
        }
        if !has_method(&layout_impl, "set_rect") {
            layout_impl.items.push(parse2(fn_set_rect)?);
        }
        if let Some(method) = fn_draw {
            if !has_method(&layout_impl, "draw") {
                layout_impl.items.push(parse2(method)?);
            }
        }
    } else if let Some(fn_size_rules) = fn_size_rules {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Layout for #name #ty_generics #where_clause {
                #fn_size_rules
                #fn_set_rect
                #fn_draw
            }
        });
    }

    let fn_pre_configure = quote! {
        fn pre_configure(&mut self, _: &mut ::kas::layout::SetRectMgr, id: ::kas::WidgetId) {
            self.#core_data.id = id;
        }
    };

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        if !has_method(&widget_impl, "pre_configure") {
            widget_impl.items.push(parse2(fn_pre_configure)?);
        }
        if let Some(method) = fn_find_id {
            if !has_method(&widget_impl, "find_id") {
                widget_impl.items.push(parse2(method)?);
            }
        }
        if let Some(item) = args.key_nav {
            widget_impl.items.push(parse2(item)?);
        }
        if let Some(item) = args.hover_highlight {
            widget_impl.items.push(parse2(item)?);
        }
        if let Some(item) = args.cursor_icon {
            widget_impl.items.push(parse2(item)?);
        }
    } else {
        let key_nav = args.key_nav;
        let hover_highlight = args.hover_highlight;
        let cursor_icon = args.cursor_icon;
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Widget
                    for #name #ty_generics #where_clause
            {
                #fn_pre_configure
                #key_nav
                #hover_highlight
                #cursor_icon
                #fn_find_id
            }
        });
    }

    Ok(())
}
