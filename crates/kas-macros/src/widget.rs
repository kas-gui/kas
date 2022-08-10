// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::args::{Child, WidgetArgs};
use crate::make_layout::NavNextResult;
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::{Scope, ScopeAttr, ScopeItem, SimplePath};
use proc_macro2::Span;
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

    fn apply(&self, attr: syn::Attribute, scope: &mut Scope) -> Result<()> {
        let args = if attr.tokens.is_empty() {
            WidgetArgs::default()
        } else {
            attr.parse_args()?
        };
        widget(args, scope)
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
    let mut layout_children = Vec::new();
    for (i, field) in fields.iter_mut().enumerate() {
        if matches!(&field.ty, Type::Macro(mac) if mac.mac == parse_quote!{ widget_core!() }) {
            if let Some(member) = opt_derive {
                emit_warning!(
                    field.ty, "unused field of type widget_core!()";
                    note = member.span() => "not used due to derive mode";
                );
                field.ty = parse_quote! { () };
                continue;
            } else if let Some(ref cd) = core_data {
                emit_warning!(
                    field.ty, "multiple fields of type widget_core!()";
                    note = cd.span() => "previous field of type widget_core!()";
                );
                field.ty = parse_quote! { () };
                continue;
            }

            core_data = Some(member(i, field.ident.clone()));

            if let Some((stor_ty, stor_def)) = args
                .layout
                .as_ref()
                .and_then(|(_, l)| l.storage_fields(&mut layout_children))
            {
                let name = format!("_{name}CoreTy");
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
                    emit_error!(attr.tokens, "unexpected token");
                }
                let ident = member(i, field.ident.clone());
                if Some(&ident) == opt_derive.as_ref() {
                    emit_error!(attr, "#[widget] must not be used on widget derive target");
                }
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
                    emit_error!(impl_, "impl conflicts with use of #[widget(derive=FIELD)]");
                }
                if !children.is_empty() {
                    emit_error!(
                        impl_,
                        "custom `WidgetChildren` implementation when using `#[widget]` on fields"
                    );
                } else if !layout_children.is_empty() {
                    emit_error!(
                        impl_,
                        "custom `WidgetChildren` implementation when using layout-defined children"
                    );
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
        let span = match scope.item {
            ScopeItem::Struct {
                fields: Fields::Named(ref fields),
                ..
            } => fields.brace_token.span,
            ScopeItem::Struct {
                fields: Fields::Unnamed(ref fields),
                ..
            } => fields.paren_token.span,
            _ => unreachable!(),
        };
        return Err(Error::new(
            span,
            "expected: a field with type `widget_core!()`",
        ));
    }

    scope.generated.push(quote! {
        impl #impl_generics ::kas::WidgetCore
            for #name #ty_generics #where_clause
        {
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

    let mut fn_size_rules = None;
    let (fn_set_rect, fn_find_id);
    let mut fn_draw = None;
    let mut kw_layout = None;

    let fn_pre_configure;
    let fn_pre_handle_event;
    let fn_handle_event;
    let mut fn_navigable = args.navigable;
    let mut fn_nav_next = None;
    let mut fn_nav_next_err = None;
    let widget_methods;

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
                #[inline]
                fn make_child_id(&mut self, index: usize) -> ::kas::WidgetId {
                    self.#inner.make_child_id(index)
                }
            }
        });

        fn_size_rules = Some(quote! {
            #[inline]
            fn size_rules(&mut self,
                size_mgr: ::kas::theme::SizeMgr,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                self.#inner.size_rules(size_mgr, axis)
            }
        });
        fn_set_rect = quote! {
            #[inline]
            fn set_rect(
                &mut self,
                mgr: &mut ::kas::event::ConfigMgr,
                rect: ::kas::geom::Rect,
                align: ::kas::layout::AlignHints,
            ) {
                self.#inner.set_rect(mgr, rect, align);
            }
        };
        fn_find_id = quote! {
            #[inline]
            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                self.#inner.find_id(coord)
            }
        };
        fn_draw = Some(quote! {
            #[inline]
            fn draw(&mut self, draw: ::kas::theme::DrawMgr) {
                self.#inner.draw(draw);
            }
        });

        fn_pre_configure = quote! {
            #[inline]
            fn pre_configure(&mut self, mgr: &mut ::kas::event::ConfigMgr, id: ::kas::WidgetId) {
                self.#inner.pre_configure(mgr, id)
            }
        };
        if fn_navigable.is_none() {
            fn_navigable = Some(quote! {
                #[inline]
                fn navigable(&self) -> bool {
                    self.#inner.navigable()
                }
            });
        }

        let configure = quote! {
            #[inline]
            fn configure(&mut self, mgr: &mut ::kas::event::ConfigMgr) {
                self.#inner.configure(mgr);
            }
        };
        let translation = quote! {
            #[inline]
            fn translation(&self) -> ::kas::geom::Offset {
                self.#inner.translation()
            }
        };
        fn_nav_next = Some(quote! {
            #[inline]
            fn nav_next(
                &mut self,
                mgr: &mut ::kas::event::ConfigMgr,
                reverse: bool,
                from: Option<usize>,
            ) -> Option<usize> {
                self.#inner.nav_next(mgr, reverse, from)
            }
        });

        if let Some(tok) = args.hover_highlight {
            emit_error!(tok.kw_span, "incompatible with widget derive");
        }
        if let Some(tok) = args.cursor_icon {
            emit_error!(tok.kw_span, "incompatible with widget derive");
        }
        fn_pre_handle_event = quote! {
            fn pre_handle_event(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                event: ::kas::event::Event,
            ) -> ::kas::event::Response {
                self.#inner.pre_handle_event(mgr, event)
            }
        };
        fn_handle_event = Some(quote! {
            #[inline]
            fn handle_event(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                event: ::kas::event::Event,
            ) -> ::kas::event::Response {
                self.#inner.handle_event(mgr, event)
            }
        });
        let handle_unused = quote! {
            #[inline]
            fn handle_unused(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                index: usize,
                event: ::kas::event::Event,
            ) -> ::kas::event::Response {
                self.#inner.handle_unused(mgr, index, event)
            }
        };
        let handle_message = quote! {
            #[inline]
            fn handle_message(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                index: usize,
            ) {
                self.#inner.handle_message(mgr, index);
            }
        };
        let handle_scroll = quote! {
            #[inline]
            fn handle_scroll(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                scroll: ::kas::event::Scroll,
            ) {
                self.#inner.handle_scroll(mgr, scroll);
            }
        };
        widget_methods = vec![
            ("configure", configure),
            ("translation", translation),
            ("handle_unused", handle_unused),
            ("handle_message", handle_message),
            ("handle_scroll", handle_scroll),
        ];
    } else {
        let core = core_data.unwrap();
        widget_methods = vec![];

        if impl_widget_children {
            let mut count = children.len();

            let mut get_rules = quote! {};
            let mut get_mut_rules = quote! {};
            for (i, child) in children.iter().enumerate() {
                let ident = &child.ident;
                get_rules.append_all(quote! { #i => Some(&self.#ident), });
                get_mut_rules.append_all(quote! { #i => Some(&mut self.#ident), });
            }
            for (i, path) in layout_children.iter().enumerate() {
                let index = count + i;
                get_rules.append_all(quote! { #index => Some(&self.#core.#path), });
                get_mut_rules.append_all(quote! { #index => Some(&mut self.#core.#path), });
            }
            count += layout_children.len();

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

        let mut set_rect = quote! { self.#core.rect = rect; };
        let mut find_id = quote! {
            use ::kas::{WidgetCore, WidgetExt};
            self.rect().contains(coord).then(|| self.id())
        };
        if let Some((kw_span, layout)) = args.layout.take() {
            kw_layout = Some(kw_span);
            fn_nav_next = match layout.nav_next(&children) {
                NavNextResult::Err(msg) => {
                    fn_nav_next_err = Some(msg);
                    None
                }
                NavNextResult::Slice(dir) => Some(quote! {
                    fn nav_next(
                        &mut self,
                        _: &mut ::kas::event::ConfigMgr,
                        reverse: bool,
                        from: Option<usize>,
                    ) -> Option<usize> {
                        let reverse = reverse ^ (#dir).is_reversed();
                        kas::util::nav_next(reverse, from, self.num_children())
                    }
                }),
                NavNextResult::List(order) => Some(quote! {
                    fn nav_next(
                        &mut self,
                        _: &mut ::kas::event::ConfigMgr,
                        reverse: bool,
                        from: Option<usize>,
                    ) -> Option<usize> {
                        let mut iter = [#(#order),*].into_iter();
                        if !reverse {
                            if let Some(wi) = from {
                                let _ = iter.find(|x| *x == wi);
                            }
                            iter.next()
                        } else {
                            let mut iter = iter.rev();
                            if let Some(wi) = from {
                                let _ = iter.find(|x| *x == wi);
                            }
                            iter.next()
                        }
                    }
                }),
            };

            let layout = layout.generate(&core)?;
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
                        mgr: &mut ::kas::event::ConfigMgr,
                        rect: ::kas::geom::Rect,
                        align: ::kas::layout::AlignHints,
                    ) {
                        use ::kas::{WidgetCore, layout};
                        self.#core.rect = (#layout).set_rect(mgr, rect, align);
                    }

                    fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                        use ::kas::{layout, Widget, WidgetCore, WidgetExt};
                        if !self.rect().contains(coord) {
                            return None;
                        }
                        let coord = coord + self.translation();
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
            set_rect = quote! {
                <Self as ::kas::layout::AutoLayout>::set_rect(self, mgr, rect, align);
            };
            find_id = quote! {
                <Self as ::kas::layout::AutoLayout>::find_id(self, coord)
            };
            fn_draw = Some(quote! {
                fn draw(&mut self, draw: ::kas::theme::DrawMgr) {
                    <Self as ::kas::layout::AutoLayout>::draw(self, draw);
                }
            });
        }
        fn_set_rect = quote! {
            fn set_rect(
                &mut self,
                mgr: &mut ::kas::event::ConfigMgr,
                rect: ::kas::geom::Rect,
                align: ::kas::layout::AlignHints,
            ) {
                #set_rect
            }
        };
        fn_find_id = quote! {
            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                #find_id
            }
        };

        fn_pre_configure = quote! {
            fn pre_configure(&mut self, _: &mut ::kas::event::ConfigMgr, id: ::kas::WidgetId) {
                self.#core.id = id;
            }
        };

        let hover_highlight = args
            .hover_highlight
            .map(|tok| tok.lit.value)
            .unwrap_or(false);
        let icon_expr = args.cursor_icon.map(|tok| tok.expr);
        let pre_handle_event = match (hover_highlight, icon_expr) {
            (false, None) => quote! {},
            (true, None) => quote! {
                if matches!(event, Event::MouseHover | Event::LostMouseHover) {
                    mgr.redraw(self.id());
                    return Response::Used;
                }
            },
            (false, Some(icon_expr)) => quote! {
                if matches!(event, Event::MouseHover) {
                    mgr.set_cursor_icon(#icon_expr);
                    return Response::Used;
                }
            },
            (true, Some(icon_expr)) => quote! {
                if matches!(event, Event::MouseHover | Event::LostMouseHover) {
                    if matches!(event, Event::MouseHover) {
                        mgr.set_cursor_icon(#icon_expr);
                    }
                    mgr.redraw(self.id());
                    return Response::Used;
                }
            },
        };
        fn_pre_handle_event = quote! {
            fn pre_handle_event(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                event: ::kas::event::Event,
            ) -> ::kas::event::Response {
                use ::kas::{event::{Event, Response}, WidgetExt};
                #pre_handle_event
                self.handle_event(mgr, event)
            }
        };
        fn_handle_event = None;
    }

    fn collect_idents(item_impl: &ItemImpl) -> Vec<Ident> {
        item_impl
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Method(m) => Some(m.sig.ident.clone()),
                _ => None,
            })
            .collect()
    }

    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let method_idents = collect_idents(layout_impl);
        let has_method = |name| method_idents.iter().any(|ident| ident == name);

        if let Some(method) = fn_size_rules {
            if !has_method("size_rules") {
                layout_impl.items.push(parse2(method)?);
            }
        }
        if !has_method("set_rect") {
            layout_impl.items.push(parse2(fn_set_rect)?);
        }
        if !has_method("find_id") {
            layout_impl.items.push(parse2(fn_find_id)?);
        }
        if let Some(method) = fn_draw {
            if !has_method("draw") {
                layout_impl.items.push(parse2(method)?);
            }
        }
    } else if let Some(fn_size_rules) = fn_size_rules {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Layout for #name #ty_generics #where_clause {
                #fn_size_rules
                #fn_set_rect
                #fn_find_id
                #fn_draw
            }
        });
    }

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        let method_idents = collect_idents(widget_impl);
        let has_method = |name| method_idents.iter().any(|ident| ident == name);

        if opt_derive.is_some() || !has_method("pre_configure") {
            widget_impl.items.push(parse2(fn_pre_configure)?);
        }
        if let Some(method) = fn_navigable {
            widget_impl.items.push(parse2(method)?);
        }
        widget_impl.items.push(parse2(fn_pre_handle_event)?);
        if let Some(item) = fn_handle_event {
            widget_impl.items.push(parse2(item)?);
        }

        if !has_method("nav_next") {
            if let Some(method) = fn_nav_next {
                widget_impl.items.push(parse2(method)?);
            } else if let Some(span) = kw_layout {
                emit_warning!(
                    span,
                    "unable to generate method `Widget::nav_next` for this layout: {}",
                    fn_nav_next_err.unwrap(),
                );
            }
        }

        for (name, method) in widget_methods {
            if !has_method(name) {
                widget_impl.items.push(parse2(method)?);
            }
        }
    } else {
        let other_methods = widget_methods.into_iter().map(|pair| pair.1);
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Widget
                    for #name #ty_generics #where_clause
            {
                #fn_pre_configure
                #fn_navigable
                #fn_pre_handle_event
                #fn_handle_event
                #(#other_methods)*
            }
        });
    }

    Ok(())
}
