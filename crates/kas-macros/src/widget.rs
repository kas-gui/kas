// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::widget_args::{member, Child, ChildIdent, WidgetArgs};
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
pub fn widget(attr_span: Span, args: WidgetArgs, scope: &mut Scope) -> Result<()> {
    scope.expand_impl_self();
    let name = &scope.ident;
    let mut data_ty = args.data_ty.map(|data_ty| data_ty.ty);

    let mut layout: Option<crate::make_layout::Tree> = None;
    let mut other_attrs = Vec::with_capacity(scope.attrs.len());
    for attr in scope.attrs.drain(..) {
        if *attr.path() == parse_quote! { layout } {
            match attr.meta {
                Meta::List(list) => {
                    layout = Some(parse2(list.tokens)?);
                }
                _ => {
                    return Err(Error::new(attr.span(), "expected `#[layout(...)]`"));
                }
            };
        } else {
            other_attrs.push(attr);
        }
    }
    scope.attrs = other_attrs;

    let mut widget_impl = None;
    let mut layout_impl = None;
    let mut tile_impl = None;
    let mut events_impl = None;

    let mut num_children = None;
    let mut get_child = None;
    let mut child_node = None;
    let mut find_child_index = None;
    let mut make_child_id = None;
    for (index, impl_) in scope.impls.iter().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                if widget_impl.is_none() {
                    widget_impl = Some(index);
                }

                for item in &impl_.items {
                    if let ImplItem::Fn(ref item) = item {
                        if item.sig.ident == "child_node" {
                            child_node = Some(item.sig.ident.clone());
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
            } else if *path == parse_quote! { ::kas::Tile }
                || *path == parse_quote! { kas::Tile }
                || *path == parse_quote! { Tile }
            {
                if tile_impl.is_none() {
                    tile_impl = Some(index);
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
            if let Some(ref tree) = layout {
                stor_defs = tree.storage_fields(&mut children);
            }
            if !stor_defs.ty_toks.is_empty() {
                let name = format!("_{name}CoreTy");
                let core_type = Ident::new(&name, Span::call_site());
                let stor_ty = &stor_defs.ty_toks;
                let stor_def = &stor_defs.def_toks;
                scope.generated.push(quote! {
                    struct #core_type {
                        _rect: ::kas::geom::Rect,
                        _id: ::kas::Id,
                        #[cfg(debug_assertions)]
                        status: ::kas::WidgetStatus,
                        #stor_ty
                    }

                    impl Default for #core_type {
                        fn default() -> Self {
                            #core_type {
                                _rect: Default::default(),
                                _id: Default::default(),
                                #[cfg(debug_assertions)]
                                status: ::kas::WidgetStatus::New,
                                #stor_def
                            }
                        }
                    }

                    impl Clone for #core_type {
                        fn clone(&self) -> Self {
                            #core_type {
                                _rect: self._rect,
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
                field.ty = parse_quote! { ::kas::DefaultCoreType };
            }

            continue;
        }

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
    }

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

    let named_child_iter = children
        .iter()
        .enumerate()
        .filter_map(|(i, child)| match child.ident {
            ChildIdent::Field(ref member) => Some((i, member)),
            ChildIdent::CoreField(_) => None,
        });
    crate::visitors::widget_index(named_child_iter, &mut scope.impls);

    if let Some(ref span) = num_children {
        if get_child.is_none() {
            emit_warning!(span, "fn num_children without fn get_child");
        }
        if child_node.is_none() {
            emit_warning!(span, "fn num_children without fn child_node");
        }
    }
    if let Some(span) = get_child.as_ref().or(child_node.as_ref()) {
        if num_children.is_none() {
            emit_warning!(span, "associated impl of `fn Tile::num_children` required");
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

    let require_rect: syn::Stmt = parse_quote! {
        #[cfg(debug_assertions)]
        #core_path.status.require_rect(&#core_path._id);
    };

    let do_impl_widget_children = get_child.is_none() && child_node.is_none();
    let fns_get_child = if do_impl_widget_children {
        let mut get_rules = quote! {};
        for (index, child) in children.iter().enumerate() {
            get_rules.append_all(child.ident.get_rule(&core_path, index));
        }

        let count = children.len();
        Some(quote! {
            fn num_children(&self) -> usize {
                #count
            }
            fn get_child(&self, index: usize) -> Option<&dyn ::kas::Tile> {
                match index {
                    #get_rules
                    _ => None,
                }
            }
        })
    } else {
        None
    };

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        let item_idents = collect_idents(widget_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        // If the user impls Widget, they must supply type Data and fn child_node

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
    let fn_rect;
    let mut fn_size_rules = None;
    let fn_set_rect;
    let mut probe = quote! {
        ::kas::Tile::id(self)
    };
    let fn_try_probe;
    let mut fn_draw = None;
    if let Some(tree) = layout {
        // TODO(opt): omit field widget.core._rect if not set here
        let mut set_rect = quote! {};
        let tree_rect = tree.rect(&core_path).unwrap_or_else(|| {
            set_rect = quote! {
                #core_path._rect = rect;
            };
            quote! { #core_path._rect }
        });
        let tree_size_rules = tree.size_rules(&core_path);
        let tree_set_rect = tree.set_rect(&core_path);
        let tree_try_probe = tree.try_probe(&core_path);
        let tree_draw = tree.draw(&core_path);
        fn_nav_next = tree.nav_next(children.iter());

        scope.generated.push(quote! {
            impl #impl_generics ::kas::MacroDefinedLayout for #impl_target {
                #[inline]
                fn rect(&self) -> ::kas::geom::Rect {
                    #tree_rect
                }

                #[inline]
                fn size_rules(
                    &mut self,
                    sizer: ::kas::theme::SizeCx,
                    axis: ::kas::layout::AxisInfo,
                ) -> ::kas::layout::SizeRules {
                    #tree_size_rules
                }

                #[inline]
                fn set_rect(
                    &mut self,
                    cx: &mut ::kas::event::ConfigCx,
                    rect: ::kas::geom::Rect,
                    hints: ::kas::layout::AlignHints,
                ) {
                    #set_rect
                    #tree_set_rect
                }

                #[inline]
                fn try_probe(&self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                    #tree_try_probe
                }

                #[inline]
                fn draw(&self, mut draw: ::kas::theme::DrawCx) {
                    draw.set_id(::kas::Tile::id(self));
                    #tree_draw
                }
            }
        });

        fn_rect = quote! {
            #[inline]
            fn rect(&self) -> ::kas::geom::Rect {
                ::kas::MacroDefinedLayout::rect(self)
            }
        };

        fn_size_rules = Some(quote! {
            fn size_rules(
                &mut self,
                sizer: ::kas::theme::SizeCx,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                #[cfg(debug_assertions)]
                #core_path.status.size_rules(&#core_path._id, axis);

                ::kas::MacroDefinedLayout::size_rules(self, sizer, axis)
            }
        });

        fn_set_rect = quote! {
            fn set_rect(
                &mut self,
                cx: &mut ::kas::event::ConfigCx,
                rect: ::kas::geom::Rect,
                hints: ::kas::layout::AlignHints,
            ) {
                #[cfg(debug_assertions)]
                #core_path.status.set_rect(&#core_path._id);

                ::kas::MacroDefinedLayout::set_rect(self, cx, rect, hints);
            }
        };

        fn_try_probe = quote! {
            fn try_probe(&self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                #[cfg(debug_assertions)]
                self.#core.status.require_rect(&self.#core._id);

                self.rect().contains(coord).then(|| ::kas::Tile::probe(self, coord))
            }
        };

        fn_draw = Some(quote! {
            fn draw(&self, draw: ::kas::theme::DrawCx) {
                #[cfg(debug_assertions)]
                #core_path.status.require_rect(&#core_path._id);

                ::kas::MacroDefinedLayout::draw(self, draw);
            }
        });

        probe = quote! {
            let coord = coord + ::kas::Tile::translation(self);
            ::kas::MacroDefinedLayout::try_probe(self, coord)
                    .unwrap_or_else(|| ::kas::Tile::id(self))
        };
    } else {
        // TODO(opt): omit field widget.core._rect if a custom `fn rect` defintion is used
        fn_rect = quote! {
            #[inline]
            fn rect(&self) -> ::kas::geom::Rect {
                #core_path._rect
            }
        };

        fn_set_rect = quote! {
            fn set_rect(
                &mut self,
                cx: &mut ::kas::event::ConfigCx,
                rect: ::kas::geom::Rect,
                hints: ::kas::layout::AlignHints,
            ) {
                #[cfg(debug_assertions)]
                #core_path.status.set_rect(&#core_path._id);

                self.#core._rect = rect;
            }
        };

        fn_try_probe = quote! {
            fn try_probe(&self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                #[cfg(debug_assertions)]
                self.#core.status.require_rect(&self.#core._id);

                self.rect().contains(coord).then(|| ::kas::Tile::probe(self, coord))
            }
        };

        fn_nav_next = Ok(quote! {
            fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
                ::kas::util::nav_next(reverse, from, self.num_children())
            }
        });
    }

    let fn_probe = quote! {
        #[inline]
        fn probe(&self, coord: ::kas::geom::Coord) -> ::kas::Id {
            #probe
        }
    };

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
                #fn_handle_event
            }
        });
    }

    let mut widget_set_rect_span = None;
    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let item_idents = collect_idents(layout_impl);
        let mut fn_rect_is_provided = fn_size_rules.is_some();

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "size_rules") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    if let Some(FnArg::Typed(arg)) = f.sig.inputs.iter().nth(2) {
                        if let Pat::Ident(ref pat_ident) = *arg.pat {
                            let axis = &pat_ident.ident;
                            f.block.stmts.insert(0, parse_quote! {
                                #[cfg(debug_assertions)]
                                self.#core.status.size_rules(&self.#core._id, #axis);
                            });
                        } else {
                            emit_error!(arg.pat, "hidden shenanigans require this parameter to have a name; suggestion: `_axis`");
                        }
                    }
                }
            }
        } else if let Some(method) = fn_size_rules {
            layout_impl.items.push(Verbatim(method));
        }

        let mut fn_set_rect_span = None;
        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "set_rect") {
            if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                fn_set_rect_span = Some(f.span());

                let path_rect = quote! { #core_path._rect };
                widget_set_rect_span = crate::visitors::widget_set_rect(path_rect, &mut f.block);
                fn_rect_is_provided |= widget_set_rect_span.is_some();

                if let Some(ref core) = core_data {
                    f.block.stmts.insert(0, parse_quote! {
                        #[cfg(debug_assertions)]
                        self.#core.status.set_rect(&self.#core._id);
                    });
                }
            }
        } else {
            layout_impl.items.push(Verbatim(fn_set_rect));
            fn_rect_is_provided = true;
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "rect") {
            if let Some(span) = widget_set_rect_span {
                let fn_rect_span = layout_impl.items[*index].span();
                emit_warning!(
                    span, "assignment `widget_set_rect!` has no effect when `fn rect` is defined";
                    note = fn_rect_span => "this `fn rect`";
                );
            } else if fn_set_rect_span.is_none() {
                let fn_rect_span = layout_impl.items[*index].span();
                emit_warning!(
                    fn_rect_span,
                    "definition of `Layout::set_rect` is expected when `fn rect` is defined"
                );
            }
        } else if fn_rect_is_provided {
            layout_impl.items.push(Verbatim(fn_rect));
        } else if let Some(span) = fn_set_rect_span {
            emit_warning!(span, "cowardly refusing to provide an impl of `fn rect` with custom `fn set_rect` without usage of `widget_set_rect!` and without a property-defined layout");
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "try_probe") {
            if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                emit_error!(
                    f,
                    "Implementations are expected to impl `fn probe`, not `try_probe`"
                );
            }
        } else {
            layout_impl.items.push(Verbatim(fn_try_probe));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "draw") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    f.block.stmts.insert(0, parse_quote! {
                        #[cfg(debug_assertions)]
                        self.#core.status.require_rect(&self.#core._id);
                    });

                    if let Some(FnArg::Typed(arg)) = f.sig.inputs.iter().nth(1) {
                        // NOTE: if the 'draw' parameter is unnamed or not 'mut'
                        // then we don't need to call DrawCx::set_id since no
                        // calls to draw methods are possible.
                        if let Pat::Ident(ref pat_ident) = *arg.pat {
                            if pat_ident.mutability.is_some() {
                                let draw = &pat_ident.ident;
                                f.block.stmts.insert(0, parse_quote! {
                                    #draw.set_id(::kas::Tile::id(self));
                                });
                            }
                        }
                    }
                }
            }
        } else if let Some(method) = fn_draw {
            layout_impl.items.push(Verbatim(method));
        }
    } else if let Some(fn_size_rules) = fn_size_rules {
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

    let required_tile_methods = required_tile_methods(&name.to_string(), &core_path);

    if let Some(index) = tile_impl {
        let tile_impl = &mut scope.impls[index];
        let item_idents = collect_idents(tile_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        tile_impl.items.push(Verbatim(required_tile_methods));

        if let Some(methods) = fns_get_child {
            tile_impl.items.push(Verbatim(methods));
        }

        if !has_item("nav_next") {
            match fn_nav_next {
                Ok(method) => tile_impl.items.push(Verbatim(method)),
                Err((span, msg)) => {
                    // We emit a warning here only if nav_next is not explicitly defined
                    emit_warning!(span, "unable to generate `fn Tile::nav_next`: {}", msg,);
                }
            }
        }

        if !has_item("probe") {
            tile_impl.items.push(Verbatim(fn_probe));
        }
    } else {
        let fn_nav_next = match fn_nav_next {
            Ok(method) => Some(method),
            Err((span, msg)) => {
                emit_warning!(span, "unable to generate `fn Tile::nav_next`: {}", msg,);
                None
            }
        };

        scope.generated.push(quote! {
            impl #impl_generics ::kas::Tile for #impl_target {
                #required_tile_methods
                #fns_get_child
                #fn_nav_next
                #fn_probe
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

pub fn required_tile_methods(name: &str, core_path: &Toks) -> Toks {
    quote! {
        #[inline]
        fn as_tile(&self) -> &dyn ::kas::Tile {
            self
        }
        #[inline]
        fn id_ref(&self) -> &::kas::Id {
            &#core_path._id
        }

        #[inline]
        fn identify(&self) -> ::kas::util::IdentifyWidget<'_> {
            ::kas::util::IdentifyWidget::simple(#name, self.id_ref())
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

            get_mut_rules.append_all(if let Some(ref data) = child.data_binding {
                quote! { #i => Some(#path.as_node(#data)), }
            } else {
                if let Some(ref span) = child.attr_span {
                    quote_spanned! {*span=> #i => Some(#path.as_node(data)), }
                } else {
                    quote! { #i => Some(#path.as_node(data)), }
                }
            });
        }

        quote! {
            fn child_node<'__n>(
                &'__n mut self,
                data: &'__n Self::Data,
                index: usize,
            ) -> Option<::kas::Node<'__n>> {
                match index {
                    #get_mut_rules
                    _ => None,
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
        fn as_node<'__a>(&'__a mut self, data: &'__a Self::Data) -> ::kas::Node<'__a> {
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
            debug_assert!(id.is_valid(), "Widget::_configure called with invalid id!");

            #core_path._id = id;
            #[cfg(debug_assertions)]
            #core_path.status.configure(&#core_path._id);

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
            #core_path.status.update(&#core_path._id);

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
