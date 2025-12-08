// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::widget_args::{Child, ChildIdent, member};
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::scope::{Scope, ScopeItem};
use proc_macro_error2::{emit_error, emit_warning};
use proc_macro2::{Span, TokenStream as Toks};
use quote::{ToTokens, TokenStreamExt, quote, quote_spanned};
use syn::ImplItem::{self, Verbatim};
use syn::parse::{Error, Result};
use syn::spanned::Spanned;
use syn::{FnArg, Ident, ImplItemFn, ItemImpl, MacroDelimiter, Member, Meta, Pat, Type};
use syn::{parse_quote, parse2};

/// Custom widget definition
///
/// This macro may inject impls and inject items into existing impls.
/// It may also inject code into existing methods such that the only observable
/// behaviour is a panic.
pub fn widget(attr_span: Span, scope: &mut Scope) -> Result<()> {
    scope.expand_impl_self();
    let layout = crate::make_layout::Tree::try_parse(scope)?;
    let name = &scope.ident;

    let mut widget_impl = None;
    let mut layout_impl = None;
    let mut viewport_impl = None;
    let mut tile_impl = None;
    let mut events_impl = None;

    let mut ty_data: Option<syn::ImplItemType> = None;

    let mut get_child = None;
    let mut child_node = None;
    let mut find_child_index = None;
    let mut make_child_id = None;
    let mut fn_probe_span = None;
    let mut translation_span = None;
    let mut handle_scroll = false;
    for (index, impl_) in scope.impls.iter_mut().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                if widget_impl.is_none() {
                    widget_impl = Some(index);
                }

                for item in &impl_.items {
                    if let ImplItem::Fn(item) = item {
                        if item.sig.ident == "child_node" {
                            child_node = Some(item.sig.ident.clone());
                        }
                    } else if let ImplItem::Type(item) = item {
                        if item.ident == "Data" {
                            if let Some(ref old) = ty_data {
                                emit_error!(
                                    item, "duplicate definitions with name `Data`";
                                    note = old.span() => "also defined here";
                                );
                            } else {
                                ty_data = Some(item.clone());
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
            } else if *path == parse_quote! { ::kas::Viewport }
                || *path == parse_quote! { kas::Viewport }
                || *path == parse_quote! { Viewport }
            {
                if viewport_impl.is_none() {
                    viewport_impl = Some(index);
                }
            } else if *path == parse_quote! { ::kas::Tile }
                || *path == parse_quote! { kas::Tile }
                || *path == parse_quote! { Tile }
            {
                if tile_impl.is_none() {
                    tile_impl = Some(index);
                }

                for item in &impl_.items {
                    if let ImplItem::Fn(item) = item {
                        if item.sig.ident == "get_child" {
                            get_child = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "find_child_index" {
                            find_child_index = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "translation" {
                            translation_span = Some(item.span());
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
                    if let ImplItem::Type(item) = item {
                        if item.ident == "Data" {
                            if let Some(ref old) = ty_data {
                                emit_error!(
                                    item, "duplicate definitions with name `Data`";
                                    note = old.span() => "also defined here";
                                );
                            } else {
                                ty_data = Some(item.clone());
                            }
                        }
                    } else if let ImplItem::Fn(item) = item {
                        if item.sig.ident == "probe" {
                            fn_probe_span = Some(item.span());
                        } else if item.sig.ident == "make_child_id" {
                            make_child_id = Some(item.span());
                        } else if item.sig.ident == "handle_scroll" {
                            handle_scroll = true;
                        }
                    }
                }
            }
        }
    }

    if find_child_index.is_none()
        && let Some(mci_span) = make_child_id
    {
        let (span, path) = if let Some(index) = tile_impl {
            (scope.impls[index].span(), "")
        } else {
            (attr_span, "Tile::")
        };
        emit_warning!(
            span, "Implementation of fn {}find_child_index is expected", path;
            note = mci_span => "Usage of custom child identifier";
        );
    }
    if !handle_scroll && let Some(tr_span) = translation_span {
        let (span, path) = if let Some(index) = events_impl {
            (scope.impls[index].span(), "")
        } else {
            (attr_span, "Events::")
        };
        emit_warning!(
            span, "Implementation of fn {}handle_scroll is expected", path;
            note = tr_span => "Scroll::Rect(_) must be translated from child coordinate space";
        );
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
    if let Some(ref layout) = layout {
        let fields: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| member(i, field.ident.clone()))
            .collect();
        layout.validate(&fields);
    }

    let mut core_data: Option<Member> = None;
    let mut children = Vec::with_capacity(fields.len());
    let mut collection: Option<(Span, Member, Type)> = None;

    for (i, field) in fields.iter_mut().enumerate() {
        let ident = member(i, field.ident.clone());
        let mut is_child = false;

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
                        status: ::kas::WidgetStatus,
                        #stor_ty
                    }

                    impl Default for #core_type {
                        fn default() -> Self {
                            #core_type {
                                _rect: Default::default(),
                                _id: Default::default(),
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

                    impl ::kas::WidgetCore for #core_type {
                        #[inline]
                        fn id_ref(&self) -> &::kas::Id {
                            &self._id
                        }

                        fn status(&self) -> ::kas::WidgetStatus {
                            self.status
                        }
                    }

                    impl ::kas::WidgetCoreRect for #core_type {
                        #[inline]
                        fn rect(&self) -> ::kas::geom::Rect {
                            self._rect
                        }

                        #[inline]
                        fn set_rect(&mut self, rect: ::kas::geom::Rect) {
                            self._rect = rect;
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
            if !is_child && *attr.path() == parse_quote! { widget } {
                is_child = true;
                let attr_span = Some(attr.span());
                let data_binding = match attr.meta {
                    Meta::Path(_) => None,
                    Meta::List(list) if matches!(&list.delimiter, MacroDelimiter::Paren(_)) => {
                        Some(parse2(list.tokens)?)
                    }
                    Meta::List(list) => {
                        let span = list.delimiter.span().join();
                        return Err(Error::new(span, "expected `#[widget]` or `#[widget(..)]`"));
                    }
                    Meta::NameValue(nv) => Some(nv.value),
                };
                children.push(Child {
                    ident: ChildIdent::Field(ident.clone()),
                    attr_span,
                    data_binding,
                });
            } else if !is_child && *attr.path() == parse_quote! { collection } {
                is_child = true;
                if let Some((coll_span, _, _)) = collection {
                    emit_error!(
                        attr, "multiple usages of #[collection] within a widget is not currently supported";
                        note = coll_span => "previous usage of #[collection]";
                        note = "write impls of fns Tile::child_indices, Tile::get_child and Widget::child_node instead";
                    );
                }
                collection = Some((attr.span(), ident.clone(), field.ty.clone()));
            } else {
                other_attrs.push(attr);
            }
        }
        field.attrs = other_attrs;
    }

    if ty_data.is_some() {
    } else if children.is_empty() && events_impl.is_none() && widget_impl.is_none() {
        if let Some((_, _, ref ty)) = collection {
            ty_data = Some(parse_quote! { type Data = <#ty as ::kas::Collection>::Data; });
        } else {
            ty_data = Some(parse_quote! { type Data = (); });
        }
    } else {
        let span = if let Some(index) = widget_impl {
            scope.impls[index].brace_token.span.open()
        } else if let Some(index) = events_impl {
            scope.impls[index].brace_token.span.open()
        } else {
            attr_span
        };
        emit_error!(
            span,
            "expected a definition of type Data in impl for Widget or impl for Events",
        );
    };

    let Some(core) = core_data else {
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

    if collection.is_none() {
        let named_child_iter =
            children
                .iter()
                .enumerate()
                .filter_map(|(i, child)| match child.ident {
                    ChildIdent::Field(ref member) => Some((i, member)),
                    ChildIdent::CoreField(_) => None,
                });
        crate::visitors::widget_index(named_child_iter, &mut scope.impls);
    }

    if let Some(span) = get_child.as_ref().or(child_node.as_ref()) {
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
        if let Some((coll_span, _, _)) = collection {
            emit_error!(
                span, "impl forbidden when using `#[collection]` on a field";
                note = coll_span => "this usage of #[collection]";
            );
        }
    }
    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let impl_generics = impl_generics.to_token_stream();
    let impl_target = quote! { #name #ty_generics #where_clause };

    let require_rect: syn::Stmt = parse_quote! {
        #core_path.status.require_rect(&#core_path._id);
    };

    let mut fn_role = None;
    let mut fn_child_indices = None;
    let mut fn_get_child = None;
    let mut fn_child_node = None;
    let get_child_span = get_child.as_ref().map(|item| item.span());
    if get_child.is_none() && child_node.is_none() {
        if !children.is_empty() {
            fn_role = Some(quote! {
                fn role(&self, _: &mut dyn ::kas::RoleCx) -> ::kas::Role<'_> {
                    ::kas::Role::None
                }
            });
        }

        if collection.is_none() {
            let mut get_rules = quote! {};
            for (index, child) in children.iter().enumerate() {
                get_rules.append_all(child.ident.get_rule(&core_path, index));
            }

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

            let count = children.len();

            fn_child_indices = Some(quote! {
                #[inline]
                fn child_indices(&self) -> ::kas::ChildIndices {
                    ::kas::ChildIndices::range(0..#count)
                }
            });
            fn_get_child = Some(quote! {
                fn get_child(&self, index: usize) -> Option<&dyn ::kas::Tile> {
                    match index {
                        #get_rules
                        _ => None,
                    }
                }
            });
            fn_child_node = Some(quote! {
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
            });
        } else if children.is_empty()
            && let Some((_, ident, _)) = collection
        {
            fn_child_indices = Some(quote! {
                #[inline]
                fn child_indices(&self) -> ::kas::ChildIndices {
                    ::kas::ChildIndices::range(0..self.#ident.len())
                }
            });
            fn_get_child = Some(quote! {
                fn get_child(&self, index: usize) -> Option<&dyn ::kas::Tile> {
                    self.#ident.get_tile(index)
                }
            });
            fn_child_node = Some(quote! {
                fn child_node<'__n>(
                    &'__n mut self,
                    data: &'__n Self::Data,
                    index: usize,
                ) -> Option<::kas::Node<'__n>> {
                    self.#ident.child_node(data, index)
                }
            });
        } else if let Some((span, _, _)) = collection {
            emit_error!(
                span, "unable to generate fns Tile::child_indices, Tile::get_child, Widget::child_node";
                note = "usage of #[collection] is not currently supported together with #[widget] fields or layout children";
            );
        }
    };

    let fn_nav_next;
    let fn_rect;
    let mut fn_size_rules = None;
    let fn_set_rect;
    let mut fn_try_probe = (fn_probe_span.is_some() || children.is_empty()).then_some(quote! {
        fn try_probe(&self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
            self.#core.status.require_rect(&self.#core._id);

            self.rect().contains(coord).then(|| ::kas::Events::probe(self, coord))
        }
    });

    let mut fn_draw = if viewport_impl.is_some() {
        Some(quote! {
            fn draw(&self, draw: ::kas::theme::DrawCx) {
                self.draw_with_offset(draw, self.rect(), ::kas::geom::Offset::ZERO);
            }
        })
    } else {
        None
    };

    let mut have_rect_from_layout = false;
    if let Some(tree) = layout {
        // TODO(opt): omit field widget.core._rect if not set here
        let get_rect;
        let set_core_rect;
        if let Some(expr) = tree.rect(&core_path) {
            have_rect_from_layout = true;
            get_rect = expr;
            set_core_rect = quote! {};
        } else {
            get_rect = quote! { #core_path._rect };
            set_core_rect = quote! {
                #core_path._rect = rect;
            };
        };
        let tree_size_rules = tree.size_rules(&core_path);
        let tree_set_rect = tree.set_rect(&core_path);
        let tree_draw = tree.draw(&core_path);
        fn_nav_next = tree.nav_next(children.iter());

        scope.generated.push(quote! {
            impl #impl_generics ::kas::MacroDefinedLayout for #impl_target {
                #[inline]
                fn rect(&self) -> ::kas::geom::Rect {
                    #get_rect
                }

                #[inline]
                fn size_rules(
                    &mut self,
                    cx: &mut ::kas::theme::SizeCx,
                    axis: ::kas::layout::AxisInfo,
                ) -> ::kas::layout::SizeRules {
                    #tree_size_rules
                }

                #[inline]
                fn set_rect(
                    &mut self,
                    cx: &mut ::kas::theme::SizeCx,
                    rect: ::kas::geom::Rect,
                    hints: ::kas::layout::AlignHints,
                ) {
                    #set_core_rect
                    #tree_set_rect
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
                cx: &mut ::kas::theme::SizeCx,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                #core_path.status.size_rules(&#core_path._id, axis);

                ::kas::MacroDefinedLayout::size_rules(self, cx, axis)
            }
        });

        fn_set_rect = quote! {
            fn set_rect(
                &mut self,
                cx: &mut ::kas::theme::SizeCx,
                rect: ::kas::geom::Rect,
                hints: ::kas::layout::AlignHints,
            ) {
                #core_path.status.require_size_determined(&#core_path._id);
                ::kas::MacroDefinedLayout::set_rect(self, cx, rect, hints);
                #core_path.status.set_sized();
            }
        };

        if fn_probe_span.is_none()
            && let Some(toks) = tree.try_probe(&core_path, &children)
        {
            fn_try_probe = Some(quote! {
                fn try_probe(&self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                    self.#core.status.require_rect(&self.#core._id);

                    #toks
                }
            });
        }

        if fn_draw.is_none() {
            fn_draw = Some(quote! {
                fn draw(&self, draw: ::kas::theme::DrawCx) {
                    #core_path.status.require_rect(&#core_path._id);

                    ::kas::MacroDefinedLayout::draw(self, draw);
                }
            });
        }
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
                _: &mut ::kas::theme::SizeCx,
                rect: ::kas::geom::Rect,
                _: ::kas::layout::AlignHints,
            ) {
                #core_path.status.require_size_determined(&#core_path._id);
                self.#core._rect = rect;
                #core_path.status.set_sized();
            }
        };

        fn_nav_next = Ok(quote! {
            fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
                ::kas::util::nav_next(reverse, from, self.child_indices())
            }
        });
    }

    fn modify_draw(f: &mut ImplItemFn, core: &Member) {
        f.block.stmts.insert(0, parse_quote! {
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

    let mut layout_draw_span = None;
    if let Some(index) = layout_impl {
        let layout_impl = &mut scope.impls[index];
        let item_idents = collect_idents(layout_impl);

        let has_fn_rect = item_idents
            .iter()
            .find(|(_, ident)| *ident == "rect")
            .is_some();
        if !has_fn_rect {
            layout_impl.items.push(Verbatim(fn_rect));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "size_rules") {
            if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                if let Some(FnArg::Typed(arg)) = f.sig.inputs.iter().nth(2) {
                    if let Pat::Ident(ref pat_ident) = *arg.pat {
                        let axis = &pat_ident.ident;
                        f.block.stmts.insert(0, parse_quote! {
                            self.#core.status.size_rules(&self.#core._id, #axis);
                        });
                    } else {
                        emit_error!(
                            arg.pat,
                            "hidden shenanigans require this parameter to have a name; suggestion: `_axis`"
                        );
                    }
                }
            }
        } else if let Some(method) = fn_size_rules {
            layout_impl.items.push(Verbatim(method));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "set_rect") {
            if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                if !has_fn_rect
                    && !have_rect_from_layout
                    && !crate::visitors::is_core_accessed(&core, &f.block)
                {
                    emit_warning!(
                        &f.block, "no call to self.{core}.set_rect(_) found";
                        note = "expected when not using an explicit definition of `fn rect`";
                        note = "this lint is a heuristic which may sometimes be wrong; it may be silenced via any access to the core field: `let _ = &self.core;`";
                    );
                }

                f.block.stmts.insert(0, parse_quote! {
                    self.#core.status.require_size_determined(&self.#core._id);
                });
                f.block.stmts.push(parse_quote! {
                    self.#core.status.set_sized();
                });
            }
        } else {
            layout_impl.items.push(Verbatim(fn_set_rect));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "draw") {
            if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                layout_draw_span = Some(f.span());

                modify_draw(f, &core);
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
                #fn_draw
            }
        });
    }

    if let Some(index) = viewport_impl {
        let viewport_impl = &mut scope.impls[index];
        let item_idents = collect_idents(viewport_impl);

        if let Some((index, _)) = item_idents
            .iter()
            .find(|(_, ident)| *ident == "draw_with_offset")
        {
            if let ImplItem::Fn(f) = &mut viewport_impl.items[*index] {
                if let Some(span) = layout_draw_span {
                    emit_error!(
                        span, "definition of `fn draw` is redundant";
                        note = f.span() => "definition of `fn draw_with_offset`"
                    );
                }

                modify_draw(f, &core);
            }
        }

        if !item_idents
            .iter()
            .any(|(_, ident)| *ident == "try_probe_with_offset")
        {
            viewport_impl.items.push(parse_quote! {
                fn try_probe_with_offset(
                    &self,
                    coord: ::kas::geom::Coord,
                    offset: ::kas::geom::Offset,
                ) -> Option<::kas::Id> {
                    self.#core.status.require_rect(&self.#core._id);

                    self.rect().contains(coord).then(|| ::kas::Events::probe(self, coord + offset))
                }
            });
        }
    }

    let required_tile_methods = required_tile_methods(&name.to_string(), &core_path);

    if let Some(index) = tile_impl {
        let tile_impl = &mut scope.impls[index];
        let item_idents = collect_idents(tile_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        tile_impl.items.push(Verbatim(required_tile_methods));

        if let Some(methods) = fn_get_child {
            tile_impl.items.push(Verbatim(methods));
        }

        if !has_item("role") {
            if let Some(method) = fn_role {
                tile_impl.items.push(Verbatim(method));
            } else {
                #[cfg(feature = "nightly-pedantic")]
                emit_warning!(tile_impl, "[pedantic] `fn role` is not defined");
            }
        }

        if !has_item("child_indices") {
            if let Some(method) = fn_child_indices {
                tile_impl.items.push(Verbatim(method));
            } else {
                let (span, reason) = if let Some(span) = get_child_span {
                    (span, "with explicit fn get_child implementation")
                } else {
                    (
                        child_node.expect("has fn child_node").span(),
                        "with explicit fn child_node implementation",
                    )
                };
                emit_error!(
                    tile_impl, "Implementation of fn child_indices is expected";
                    note = span => reason;
                );
            }
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "try_probe") {
            if let Some(span2) = fn_probe_span {
                let span = tile_impl.items[*index].span();
                emit_error!(
                    span2, "implementation conflicts with fn try_probe";
                    note = span => "this implementation overrides fn probe"
                );
            }
        } else if let Some(method) = fn_try_probe {
            tile_impl.items.push(Verbatim(method));
        } else {
            emit_error!(
                tile_impl,
                "expected definition of fn Events::probe or fn Tile::try_probe"
            );
        }

        if !has_item("nav_next") {
            match fn_nav_next {
                Ok(method) => tile_impl.items.push(Verbatim(method)),
                Err((span, msg)) => {
                    // We emit a warning here only if nav_next is not explicitly defined
                    emit_warning!(span, "unable to generate `fn nav_next`: {}", msg,);
                }
            }
        }

        tile_impl.items.push(Verbatim(widget_nav_next()));
    } else {
        #[cfg(feature = "nightly-pedantic")]
        if fn_role.is_none() {
            emit_warning!(attr_span, "[pedantic] `fn Tile::role` is not defined");
        }

        if fn_child_indices.is_none() {
            let (span, reason) = if let Some(span) = get_child_span {
                (span, "with explicit fn get_child implementation")
            } else {
                (
                    child_node.expect("has fn child_node").span(),
                    "with explicit fn child_node implementation",
                )
            };
            emit_error!(
                attr_span, "Implementation of fn Tile::child_indices is expected";
                note = span => reason;
            );
        }

        if fn_try_probe.is_none() {
            emit_error!(
                attr_span,
                "expected definition of fn Events::probe or fn Tile::try_probe"
            );
        }

        let fn_nav_next = match fn_nav_next {
            Ok(method) => Some(method),
            Err((span, msg)) => {
                emit_warning!(span, "unable to generate `fn Tile::nav_next`: {}", msg,);
                None
            }
        };

        let fn_r_nav_next = widget_nav_next();

        scope.generated.push(quote! {
            impl #impl_generics ::kas::Tile for #impl_target {
                #required_tile_methods
                #fn_role
                #fn_get_child
                #fn_child_indices
                #fn_try_probe
                #fn_nav_next
                #fn_r_nav_next
            }
        });
    }

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

    if let Some(index) = widget_impl {
        let widget_impl = &mut scope.impls[index];
        let item_idents = collect_idents(widget_impl);
        let has_item = |name| item_idents.iter().any(|(_, ident)| ident == name);

        // If the user impls Widget, they must supply type Data and fn child_node

        // Always impl fn as_node
        widget_impl.items.push(Verbatim(widget_as_node()));

        if !has_item("child_node") {
            if let Some(method) = fn_child_node {
                widget_impl.items.push(Verbatim(method));
            } else {
                emit_error!(
                    widget_impl, "refusing to generate fn child_node";
                    note = get_child_span.expect("get_child_span") => "due to explicit impl of fn Tile::get_child";
                );
            }
        }

        if !has_item("_send") {
            widget_impl
                .items
                .push(Verbatim(widget_recursive_methods(&core_path)));
        }
    } else {
        let fns_as_node = widget_as_node();

        if fn_child_node.is_none() {
            emit_error!(
                attr_span, "refusing to generate fn Widget::child_node";
                note = get_child_span.expect("get_child_span") => "due to explicit impl of fn Tile::get_child";
            );
        }

        let fns_recurse = widget_recursive_methods(&core_path);

        scope.generated.push(quote_spanned! {attr_span=>
            impl #impl_generics ::kas::Widget for #impl_target {
                #ty_data
                #fns_as_node
                #fn_child_node
                #fns_recurse
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
        fn status(&self) -> ::kas::WidgetStatus {
            #core_path.status
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
            ::kas::impls::_configure(self, cx, data);
            #core_path.status.set_configured();
        }

        fn _update(
            &mut self,
            cx: &mut ::kas::event::ConfigCx,
            data: &Self::Data,
        ) {
            #core_path.status.require_configured(&#core_path._id);
            ::kas::impls::_update(self, cx, data);
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
            &self,
            cx: &::kas::event::EventState,
            focus: Option<&::kas::Id>,
            advance: ::kas::event::NavAdvance,
        ) -> Option<::kas::Id> {
            ::kas::impls::_nav_next(self, cx, focus, advance)
        }
    }
}
