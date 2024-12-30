// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::widget_args::{member, Child, ChildIdent, Layout, WidgetArgs};
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::scope::{Scope, ScopeItem};
use proc_macro2::{Span, TokenStream as Toks};
use proc_macro_error2::{emit_error, emit_warning};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Error, Result};
use syn::spanned::Spanned;
use syn::ImplItem::{self, Verbatim};
use syn::{parse2, parse_quote};
use syn::{FnArg, Ident, ItemImpl, MacroDelimiter, Member, Meta, Pat, Type};

/// Custom widget definition
///
/// This macro may inject impls and inject items into existing impls.
/// It may also inject code into existing methods such that the only observable
/// behaviour is a panic.
pub fn widget(attr_span: Span, mut args: WidgetArgs, scope: &mut Scope) -> Result<()> {
    assert!(args.derive.is_none());
    scope.expand_impl_self();
    let name = &scope.ident;
    let mut data_ty = args.data_ty.map(|data_ty| data_ty.ty);

    let mut widget_impl = None;
    let mut layout_impl = None;
    let mut events_impl = None;

    let mut num_children = None;
    let mut get_child = None;
    let mut for_child_node = None;
    let mut find_child_index = None;
    let mut make_child_id = None;
    for (index, impl_) in scope.impls.iter().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                widget_impl = Some(index);

                for item in &impl_.items {
                    if let ImplItem::Fn(ref item) = item {
                        if item.sig.ident == "for_child_node" {
                            for_child_node = Some(item.sig.ident.clone());
                        }
                    } else if let ImplItem::Type(ref item) = item {
                        if item.ident == "Data" {
                            if let Some(ref ty) = data_ty {
                                emit_error!(
                                    ty, "depulicate definition";
                                    note = item.ty.span() => "also defined here";
                                );
                            } else {
                                data_ty = Some(item.ty.clone());
                            }
                        }
                    }
                }
            } else if *path == parse_quote! { ::kas::Layout }
                || *path == parse_quote! { kas::Layout }
                || *path == parse_quote! { Layout }
            {
                if layout_impl.is_none() {
                    layout_impl = Some(index);
                }

                for item in &impl_.items {
                    if let ImplItem::Fn(ref item) = item {
                        if item.sig.ident == "num_children" {
                            num_children = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "get_child" {
                            get_child = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "find_child_index" {
                            find_child_index = Some(item.sig.ident.clone());
                        }
                    }
                }
            } else if *path == parse_quote! { ::kas::Events }
                || *path == parse_quote! { kas::Events }
                || *path == parse_quote! { Events }
            {
                if events_impl.is_none() {
                    events_impl = Some(index);
                }

                for item in &impl_.items {
                    if let ImplItem::Type(ref item) = item {
                        if item.ident == "Data" {
                            if let Some(ref ty) = data_ty {
                                emit_error!(
                                    ty, "depulicate definition";
                                    note = item.ty.span() => "also defined here";
                                );
                            } else {
                                data_ty = Some(item.ty.clone());
                            }
                        }
                    } else if let ImplItem::Fn(ref item) = item {
                        if item.sig.ident == "make_child_id" {
                            make_child_id = Some(item.sig.ident.clone());
                        }
                    }
                }
            }
        }
    }

    if let Some(ref span) = find_child_index {
        if make_child_id.is_none() {
            emit_warning!(span, "fn find_child_index without fn make_child_id");
        }
    } else if let Some(ref span) = make_child_id {
        emit_warning!(span, "fn make_child_id without fn find_child_index");
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

    let data_ty = if let Some(ty) = data_ty {
        ty
    } else {
        let span = if let Some(index) = widget_impl {
            scope.impls[index].brace_token.span.open()
        } else if let Some(index) = events_impl {
            scope.impls[index].brace_token.span.open()
        } else {
            attr_span
        };
        return Err(Error::new(
            span,
            "expected a definition of Data in Widget, Events or via #[widget { Data = ...; }]",
        ));
    };

    let mut core_data: Option<Member> = None;
    let mut children = Vec::with_capacity(fields.len());

    for (i, field) in fields.iter_mut().enumerate() {
        let ident = member(i, field.ident.clone());

        if matches!(&field.ty, Type::Macro(mac) if mac.mac == parse_quote!{ widget_core!() }) {
            if let Some(ref cd) = core_data {
                emit_warning!(
                    field.ty, "multiple fields of type widget_core!()";
                    note = cd.span() => "previous field of type widget_core!()";
                );
                field.ty = parse_quote! { () };
                continue;
            }

            core_data = Some(ident.clone());

            let mut stor_defs = Default::default();
            if let Some(Layout { ref tree, .. }) = args.layout {
                stor_defs = tree.storage_fields(&mut children, &data_ty);
            }
            if !stor_defs.ty_toks.is_empty() {
                let name = format!("_{name}CoreTy");
                let core_type = Ident::new(&name, Span::call_site());
                let stor_ty = &stor_defs.ty_toks;
                let stor_def = &stor_defs.def_toks;
                scope.generated.push(quote! {
                    struct #core_type {
                        rect: ::kas::geom::Rect,
                        id: ::kas::Id,
                        #[cfg(debug_assertions)]
                        status: ::kas::WidgetStatus,
                        #stor_ty
                    }

                    impl Default for #core_type {
                        fn default() -> Self {
                            #core_type {
                                rect: Default::default(),
                                id: Default::default(),
                                #[cfg(debug_assertions)]
                                status: ::kas::WidgetStatus::New,
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

        let mut is_widget = false;
        let mut other_attrs = Vec::with_capacity(field.attrs.len());
        for attr in field.attrs.drain(..) {
            if *attr.path() == parse_quote! { widget } {
                let data_binding = match &attr.meta {
                    Meta::Path(_) => None,
                    Meta::List(list) if matches!(&list.delimiter, MacroDelimiter::Paren(_)) => {
                        Some(parse2(list.tokens.clone())?)
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
                is_widget = true;
                children.push(Child {
                    ident: ChildIdent::Field(ident.clone()),
                    attr_span: Some(attr.span()),
                    data_binding,
                });
            } else {
                other_attrs.push(attr);
            }
        }
        field.attrs = other_attrs;

        if !is_widget {
            if let Some(Layout { ref tree, .. }) = args.layout {
                if let Some(span) = tree.span_in_layout(&ident) {
                    emit_error!(
                        span, "fields used in layout must be widgets";
                        note = field.span() => "this field is missing a #[widget] attribute?"
                    );
                }
            }
        }
    }

    let named_child_iter = children
        .iter()
        .enumerate()
        .filter_map(|(i, child)| match child.ident {
            ChildIdent::Field(ref member) => Some((i, member)),
            ChildIdent::CoreField(_) => None,
        });
    crate::widget_index::visit_impls(named_child_iter, &mut scope.impls);

    if let Some(ref span) = num_children {
        if get_child.is_none() {
            emit_warning!(span, "fn num_children without fn get_child");
        }
        if for_child_node.is_none() {
            emit_warning!(span, "fn num_children without fn for_child_node");
        }
    }
    if let Some(span) = get_child.as_ref().or(for_child_node.as_ref()) {
        if num_children.is_none() {
            emit_warning!(
                span,
                "associated impl of `fn Layout::num_children` required"
            );
        }
        if !children.is_empty() {
            if children
                .iter()
                .any(|child| matches!(child.ident, ChildIdent::Field(_)))
            {
                emit_error!(span, "impl forbidden when using `#[widget]` on fields");
            } else {
                emit_error!(span, "impl forbidden when using layout-defined children");
            }
        }
    }
    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let impl_generics = impl_generics.to_token_stream();
    let impl_target = quote! { #name #ty_generics #where_clause };

    let Some(core) = core_data.clone() else {
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
            span.join(),
            "expected: a field with type `widget_core!()`",
        ));
    };
    let core_path = quote! { self.#core };

    let require_rect: syn::Stmt = parse_quote! {
        #[cfg(debug_assertions)]
        #core_path.status.require_rect(&#core_path.id);
    };

    let mut required_layout_methods = impl_core_methods(&name.to_string(), &core_path);

    let do_impl_widget_children = get_child.is_none() && for_child_node.is_none();
    if do_impl_widget_children {
        let mut get_rules = quote! {};
        for (index, child) in children.iter().enumerate() {
            get_rules.append_all(child.ident.get_rule(&core_path, index));
        }

        let count = children.len();
        required_layout_methods.append_all(quote! {
            fn num_children(&self) -> usize {
                #count
            }
            fn get_child(&self, index: usize) -> Option<&dyn ::kas::Layout> {
                use ::kas::Layout;
                match index {
                    #get_rules
                    _ => None,
                }
            }
        });
    }

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        let item_idents = collect_idents(widget_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        // If the user impls Widget, they must supply type Data and fn for_child_node

        // Always impl fn as_node
        widget_impl.items.push(Verbatim(widget_as_node()));

        if !has_item("_send") {
            widget_impl
                .items
                .push(Verbatim(widget_recursive_methods(&core_path)));
        }

        if !has_item("_nav_next") {
            widget_impl.items.push(Verbatim(widget_nav_next()));
        }
    } else {
        scope.generated.push(impl_widget(
            &impl_generics,
            &impl_target,
            &data_ty,
            &core_path,
            &children,
            do_impl_widget_children,
        ));
    }

    let fn_nav_next;
    let mut fn_nav_next_err = None;
    let mut fn_size_rules = None;
    let mut set_rect = quote! { self.#core.rect = rect; };
    let mut probe = quote! {
        use ::kas::{Layout, LayoutExt};
            self.id()
    };
    let mut fn_draw = None;
    if let Some(Layout { tree, .. }) = args.layout.take() {
        fn_nav_next = match tree.nav_next(children.iter()) {
            Ok(toks) => Some(toks),
            Err((span, msg)) => {
                fn_nav_next_err = Some((span, msg));
                None
            }
        };

        let layout_visitor = tree.layout_visitor(&core_path)?;
        scope.generated.push(quote! {
                impl #impl_generics ::kas::layout::LayoutVisitor for #impl_target {
                    fn layout_visitor(&mut self) -> ::kas::layout::Visitor<impl ::kas::layout::Visitable> {
                        use ::kas::layout;
                        #layout_visitor
                    }
                }
            });

        fn_size_rules = Some(quote! {
            fn size_rules(
                &mut self,
                sizer: ::kas::theme::SizeCx,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                #[cfg(debug_assertions)]
                #core_path.status.size_rules(&#core_path.id, axis);
                ::kas::layout::LayoutVisitor::layout_visitor(self).size_rules(sizer, axis)
            }
        });
        set_rect = quote! {
            #core_path.rect = rect;
            ::kas::layout::LayoutVisitor::layout_visitor(self).set_rect(cx, rect, hints);
        };
        probe = quote! {
            use ::kas::{Layout, LayoutExt, layout::LayoutVisitor};

            let coord = coord + self.translation();
            self.layout_visitor()
                .try_probe(coord)
                    .unwrap_or_else(|| self.id())
        };
        fn_draw = Some(quote! {
            fn draw(&mut self, draw: ::kas::theme::DrawCx) {
                #[cfg(debug_assertions)]
                #core_path.status.require_rect(&#core_path.id);

                ::kas::layout::LayoutVisitor::layout_visitor(self).draw(draw);
            }
        });
    } else {
        fn_nav_next = Some(quote! {
            fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
                ::kas::util::nav_next(reverse, from, self.num_children())
            }
        });
    }
    let fn_set_rect = quote! {
        fn set_rect(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            rect: ::kas::geom::Rect,
            hints: ::kas::layout::AlignHints,
        ) {
            #[cfg(debug_assertions)]
            #core_path.status.set_rect(&#core_path.id);
            #set_rect
        }
    };
    let fn_probe = quote! {
        fn probe(&mut self, coord: ::kas::geom::Coord) -> ::kas::Id {
            #[cfg(debug_assertions)]
            #core_path.status.require_rect(&#core_path.id);

            #probe
        }
    };

    let hover_highlight = args
        .hover_highlight
        .map(|tok| tok.lit.value)
        .unwrap_or(false);
    let icon_expr = args.cursor_icon.map(|tok| tok.expr);
    let fn_handle_hover = match (hover_highlight, icon_expr) {
        (false, None) => quote! {},
        (true, None) => quote! {
            #[inline]
            fn handle_hover(&mut self, cx: &mut EventCx, _: bool) {
                cx.redraw(self);
            }
        },
        (false, Some(icon_expr)) => quote! {
            #[inline]
            fn handle_hover(&mut self, cx: &mut EventCx, state: bool) {
                if state {
                    cx.set_hover_cursor(#icon_expr);
                }
            }
        },
        (true, Some(icon_expr)) => quote! {
            #[inline]
            fn handle_hover(&mut self, cx: &mut EventCx, state: bool) {
                cx.redraw(self);
                if state {
                    cx.set_hover_cursor(#icon_expr);
                }
            }
        },
    };

    let fn_navigable = args.navigable;
    let fn_handle_event = quote! {
            fn handle_event(
            &mut self,
            _: &mut ::kas::event::EventCx,
            _: &Self::Data,
            _: ::kas::event::Event,
        ) -> ::kas::event::IsUsed {
            #require_rect
            ::kas::event::Unused
        }
    };

    if let Some(index) = events_impl {
        let events_impl = &mut scope.impls[index];
        let item_idents = collect_idents(events_impl);

        if let Some(method) = fn_navigable {
            events_impl.items.push(Verbatim(method));
        }

        events_impl.items.push(Verbatim(fn_handle_hover));

        if let Some((index, _)) = item_idents
            .iter()
            .find(|(_, ident)| *ident == "handle_event")
        {
            if let ImplItem::Fn(f) = &mut events_impl.items[*index] {
                f.block.stmts.insert(0, require_rect);
            }
        } else {
            events_impl.items.push(Verbatim(fn_handle_event));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "Data") {
            // Remove "type Data" item; it belongs in Widget impl.
            // Do this last to avoid affecting item indices.
            events_impl.items.remove(*index);
        }
    } else {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Events for #impl_target {
                #fn_navigable
                #fn_handle_hover
                #fn_handle_event
            }
        });
    }

    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let item_idents = collect_idents(layout_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        layout_impl.items.push(Verbatim(required_layout_methods));

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "size_rules") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    if let Some(FnArg::Typed(arg)) = f.sig.inputs.iter().nth(2) {
                        if let Pat::Ident(ref pat_ident) = *arg.pat {
                            let axis = &pat_ident.ident;
                            f.block.stmts.insert(0, parse_quote! {
                                #[cfg(debug_assertions)]
                                self.#core.status.size_rules(&self.#core.id, #axis);
                            });
                        } else {
                            emit_error!(arg.pat, "hidden shenanigans require this parameter to have a name; suggestion: `_axis`");
                        }
                    } else {
                        panic!("size_rules misses args!");
                    }
                }
            }
        } else if let Some(method) = fn_size_rules {
            layout_impl.items.push(Verbatim(method));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "set_rect") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    f.block.stmts.insert(0, parse_quote! {
                        #[cfg(debug_assertions)]
                        self.#core.status.set_rect(&self.#core.id);
                    });
                }
            }
        } else {
            layout_impl.items.push(Verbatim(fn_set_rect));
        }

        if !has_item("nav_next") {
            if let Some(method) = fn_nav_next {
                layout_impl.items.push(Verbatim(method));
            } else if let Some((span, msg)) = fn_nav_next_err {
                // We emit a warning here only if nav_next is not explicitly defined
                emit_warning!(span, "unable to generate `fn Layout::nav_next`: {}", msg,);
            }
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "probe") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    f.block.stmts.insert(0, parse_quote! {
                        #[cfg(debug_assertions)]
                        self.#core.status.require_rect(&self.#core.id);
                    });
                }
            }
        } else {
            layout_impl.items.push(Verbatim(fn_probe));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "try_probe") {
            if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                emit_warning!(
                    f,
                    "Implementations are expected to impl `fn probe`, not `try_probe`"
                );
            }
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "draw") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    f.block.stmts.insert(0, parse_quote! {
                        #[cfg(debug_assertions)]
                        self.#core.status.require_rect(&self.#core.id);
                    });
                }
            }
        } else if let Some(method) = fn_draw {
            layout_impl.items.push(Verbatim(method));
        }
    } else if let Some(fn_size_rules) = fn_size_rules {
        if fn_nav_next.is_none() {
            if let Some((span, msg)) = fn_nav_next_err {
                emit_warning!(span, "unable to generate `fn Layout::nav_next`: {}", msg,);
            }
        }

        scope.generated.push(quote! {
            impl #impl_generics ::kas::Layout for #impl_target {
                #required_layout_methods
                #fn_size_rules
                #fn_set_rect
                #fn_nav_next
                #fn_probe
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

pub fn collect_idents(item_impl: &ItemImpl) -> Vec<(usize, Ident)> {
    item_impl
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            ImplItem::Fn(m) => Some((i, m.sig.ident.clone())),
            ImplItem::Type(t) => Some((i, t.ident.clone())),
            _ => None,
        })
        .collect()
}

pub fn impl_core_methods(name: &str, core_path: &Toks) -> Toks {
    quote! {
        #[inline]
        fn as_layout(&self) -> &dyn ::kas::Layout {
            self
        }
        #[inline]
        fn id_ref(&self) -> &::kas::Id {
            &#core_path.id
        }
        #[inline]
        fn rect(&self) -> ::kas::geom::Rect {
            #core_path.rect
        }

        #[inline]
        fn widget_name(&self) -> &'static str {
            #name
        }
    }
}

pub fn impl_widget(
    impl_generics: &Toks,
    impl_target: &Toks,
    data_ty: &Type,
    core_path: &Toks,
    children: &[Child],
    do_impl_widget_children: bool,
) -> Toks {
    let fns_as_node = widget_as_node();

    let fns_for_child = if do_impl_widget_children {
        let mut get_mut_rules = quote! {};
        for (i, child) in children.iter().enumerate() {
            let path = match &child.ident {
                ChildIdent::Field(ident) => quote! { self.#ident },
                ChildIdent::CoreField(ident) => quote! { #core_path.#ident },
            };
            // TODO: incorrect or unconstrained data type of child causes a poor error
            // message here. Add a constaint like this (assuming no mapping fn):
            // <#ty as WidgetNode::Data> == Self::Data
            // But this is unsupported: rust#20041
            // predicates.push(..);

            get_mut_rules.append_all(if let Some(ref data) = child.data_binding {
                quote! { #i => closure(#path.as_node(#data)), }
            } else {
                if let Some(ref span) = child.attr_span {
                    quote_spanned! {*span=> #i => closure(#path.as_node(data)), }
                } else {
                    quote! { #i => closure(#path.as_node(data)), }
                }
            });
        }

        quote! {
            fn for_child_node(
                &mut self,
                data: &Self::Data,
                index: usize,
                closure: Box<dyn FnOnce(::kas::Node<'_>) + '_>,
            ) {
                use ::kas::Layout;
                match index {
                    #get_mut_rules
                    _ => (),
                }
            }
        }
    } else {
        quote! {}
    };

    let fns_recurse = widget_recursive_methods(core_path);
    let fn_nav_next = widget_nav_next();

    quote! {
        impl #impl_generics ::kas::Widget for #impl_target {
            type Data = #data_ty;
            #fns_as_node
            #fns_for_child
            #fns_recurse
            #fn_nav_next
        }
    }
}

pub fn widget_as_node() -> Toks {
    quote! {
        #[inline]
        fn as_node<'a>(&'a mut self, data: &'a Self::Data) -> ::kas::Node<'a> {
            ::kas::Node::new(self, data)
        }
    }
}

fn widget_recursive_methods(core_path: &Toks) -> Toks {
    quote! {
        fn _configure(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
            id: ::kas::Id,
        ) {
            #core_path.id = id;
            #[cfg(debug_assertions)]
            #core_path.status.configure(&#core_path.id);

            ::kas::Events::configure(self, cx);
            ::kas::Events::update(self, cx, data);
            ::kas::Events::configure_recurse(self, cx, data);
        }

        fn _update(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
        ) {
            #[cfg(debug_assertions)]
            #core_path.status.update(&#core_path.id);

            ::kas::Events::update(self, cx, data);
            ::kas::Events::update_recurse(self, cx, data);
        }

        fn _send(
            &mut self,
            cx: &mut ::kas::event::EventCx,
            data: &Self::Data,
            id: ::kas::Id,
            event: ::kas::event::Event,
        ) -> ::kas::event::IsUsed {
            ::kas::impls::_send(self, cx, data, id, event)
        }

        fn _replay(
            &mut self,
            cx: &mut ::kas::event::EventCx,
            data: &Self::Data,
            id: ::kas::Id,
        ) {
            ::kas::impls::_replay(self, cx, data, id);
        }
    }
}

fn widget_nav_next() -> Toks {
    quote! {
        fn _nav_next(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
            focus: Option<&::kas::Id>,
            advance: ::kas::NavAdvance,
        ) -> Option<::kas::Id> {
            ::kas::impls::_nav_next(self, cx, data, focus, advance)
        }
    }
}
