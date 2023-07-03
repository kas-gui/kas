// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::{Scope, ScopeAttr, ScopeItem, SimplePath};
use proc_macro2::{Span, TokenStream as Toks};
use proc_macro_error::{emit_error, emit_warning};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Eq;
use syn::{parse2, parse_quote, Ident, ImplItem, Index, ItemImpl, Member, Meta, Token, Type};

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(layout);
    custom_keyword!(navigable);
    custom_keyword!(hover_highlight);
    custom_keyword!(cursor_icon);
    custom_keyword!(derive);
    custom_keyword!(Data);
}

#[derive(Debug)]
pub struct BoolToken {
    pub kw_span: Span,
    pub eq: Eq,
    pub lit: syn::LitBool,
}

#[derive(Debug)]
pub struct ExprToken {
    pub kw_span: Span,
    pub eq: Eq,
    pub expr: syn::Expr,
}

#[derive(Debug, Default)]
pub struct WidgetArgs {
    data_ty: Option<Type>,
    pub navigable: Option<Toks>,
    pub hover_highlight: Option<BoolToken>,
    pub cursor_icon: Option<ExprToken>,
    pub derive: Option<Member>,
    pub layout: Option<(kw::layout, make_layout::Tree)>,
    span_end: Option<Span>, // None if and only if #[widget] has no brackets
}

impl Parse for WidgetArgs {
    fn parse(content: ParseStream) -> Result<Self> {
        let mut data_ty = None;
        let mut navigable = None;
        let mut hover_highlight = None;
        let mut cursor_icon = None;
        let mut kw_derive = None;
        let mut derive = None;
        let mut layout = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::Data) && data_ty.is_none() {
                let kw = content.parse::<kw::Data>()?;
                let _: Eq = content.parse()?;
                data_ty = Some((kw, content.parse()?));
            } else if lookahead.peek(kw::navigable) && navigable.is_none() {
                let span = content.parse::<kw::navigable>()?.span();
                let _: Eq = content.parse()?;
                let value = content.parse::<syn::LitBool>()?;
                navigable = Some(quote_spanned! {span=>
                    fn navigable(&self) -> bool { #value }
                });
            } else if lookahead.peek(kw::hover_highlight) && hover_highlight.is_none() {
                hover_highlight = Some(BoolToken {
                    kw_span: content.parse::<kw::hover_highlight>()?.span(),
                    eq: content.parse()?,
                    lit: content.parse()?,
                });
            } else if lookahead.peek(kw::cursor_icon) && cursor_icon.is_none() {
                cursor_icon = Some(ExprToken {
                    kw_span: content.parse::<kw::cursor_icon>()?.span(),
                    eq: content.parse()?,
                    expr: content.parse()?,
                });
            } else if lookahead.peek(kw::derive) && derive.is_none() {
                kw_derive = Some(content.parse::<kw::derive>()?);
                let _: Eq = content.parse()?;
                let _: Token![self] = content.parse()?;
                let _: Token![.] = content.parse()?;
                derive = Some(content.parse()?);
            } else if lookahead.peek(kw::layout) && layout.is_none() {
                let kw = content.parse::<kw::layout>()?;
                let _: Eq = content.parse()?;
                layout = Some((kw, content.parse()?));
            } else {
                return Err(lookahead.error());
            }

            let _ = content.parse::<Token![;]>()?;
        }

        if let Some(_derive) = kw_derive {
            if let Some((kw, _)) = layout {
                return Err(Error::new(kw.span, "incompatible with widget derive"));
                // note = derive.span() => "this derive"
            }
            if let Some((kw, _)) = data_ty {
                return Err(Error::new(kw.span, "incompatible with widget derive"));
            }
        }

        Ok(WidgetArgs {
            data_ty: data_ty.map(|(_, ty)| ty),
            navigable,
            hover_highlight,
            cursor_icon,
            derive,
            layout,
            span_end: Some(content.span()),
        })
    }
}

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
        let args = match &attr.meta {
            Meta::Path(_) => WidgetArgs::default(),
            _ => attr.parse_args()?,
        };
        widget(args, scope)
    }
}

pub fn widget(mut args: WidgetArgs, scope: &mut Scope) -> Result<()> {
    scope.expand_impl_self();
    let name = &scope.ident;
    let opt_derive = &args.derive;
    let mut data_ty = args.data_ty;

    let mut widget_impl = None;
    let mut do_data_ty_impl = true;
    let mut do_recursive_methods = true;
    let mut layout_impl = None;
    let mut events_impl = None;

    let mut num_children = None;
    let mut get_child = None;
    let mut get_child_mut = None;
    for (index, impl_) in scope.impls.iter().enumerate() {
        if let Some((_, ref path, _)) = impl_.trait_ {
            if *path == parse_quote! { ::kas::Widget }
                || *path == parse_quote! { kas::Widget }
                || *path == parse_quote! { Widget }
            {
                widget_impl = Some(index);

                for item in &impl_.items {
                    if let ImplItem::Fn(ref item) = item {
                        if item.sig.ident == "get_child" {
                            get_child = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "get_child_mut" {
                            get_child_mut = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "_send" {
                            do_recursive_methods = false;
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
                            do_data_ty_impl = false;
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

                let mut find_child_index = None;
                let mut make_child_id = None;
                for item in &impl_.items {
                    if let ImplItem::Fn(ref item) = item {
                        if item.sig.ident == "num_children" {
                            num_children = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "find_child_index" {
                            find_child_index = Some(item.sig.ident.clone());
                        } else if item.sig.ident == "make_child_id" {
                            make_child_id = Some(item.sig.ident.clone());
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
            } else if *path == parse_quote! { ::kas::Events }
                || *path == parse_quote! { kas::Events }
                || *path == parse_quote! { Events }
            {
                if events_impl.is_none() {
                    events_impl = Some(index);
                }
            }
        }
    }

    // TODO: as a temporary measure, we default to Data = ()
    if data_ty.is_none() {
        data_ty = Some(parse_quote! { () });
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

    let mut derive_ty_def = None;
    let mut core_data: Option<Member> = None;
    let mut children = Vec::with_capacity(fields.len());
    let mut layout_children = Vec::new();
    for (i, field) in fields.iter_mut().enumerate() {
        let ident = member(i, field.ident.clone());

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

            core_data = Some(ident.clone());

            let mut stor_defs = Default::default();
            if let Some((kw, ref layout)) = args.layout {
                let missing_data_ty = parse_quote! { MissingData };
                let dty = data_ty.as_ref().unwrap_or(&missing_data_ty);
                stor_defs = layout.storage_fields(&mut layout_children, dty);
                if data_ty.is_none() && stor_defs.used_data_ty {
                    emit_error!(
                        args.span_end.unwrap(), "expected: `Data = TYPE;`";
                        note = kw.span => "required by this layout";
                    );
                }
            }
            if !stor_defs.ty_toks.is_empty() {
                let name = format!("_{name}CoreTy");
                let core_type = Ident::new(&name, Span::call_site());
                let stor_ty = &stor_defs.ty_toks;
                let stor_def = &stor_defs.def_toks;
                scope.generated.push(quote! {
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
        } else if Some(&ident) == opt_derive.as_ref() {
            let ty = &field.ty;
            derive_ty_def = Some(quote! { type Data = <#ty as ::kas::Widget>::Data; });
        }

        let mut is_widget = false;
        let mut other_attrs = Vec::with_capacity(field.attrs.len());
        for attr in field.attrs.drain(..) {
            if *attr.path() == parse_quote! { widget } {
                if let Some(span) = match &attr.meta {
                    Meta::Path(_) => None,
                    Meta::List(list) => Some(list.delimiter.span().join()),
                    Meta::NameValue(nv) => nv.eq_token.span().join(nv.value.span()),
                } {
                    emit_error!(span, "unexpected");
                }
                if Some(&ident) == opt_derive.as_ref() {
                    emit_error!(attr, "#[widget] must not be used on widget derive target");
                }
                is_widget = true;
                children.push(ident.clone());
            } else {
                other_attrs.push(attr);
            }
        }
        field.attrs = other_attrs;

        if !is_widget {
            if let Some(span) = args
                .layout
                .as_ref()
                .and_then(|layout| layout.1.span_in_layout(&ident))
            {
                emit_error!(
                    span, "fields used in layout must be widgets";
                    note = field.span() => "this field is missing a #[widget] attribute?"
                );
            }
        }
    }

    crate::widget_index::visit_impls(children.iter(), &mut scope.impls);

    if let Some(ref span) = num_children {
        if get_child.is_none() {
            emit_warning!(span, "fn num_children without fn get_child");
        }
        if get_child_mut.is_none() {
            emit_warning!(span, "fn num_children without fn get_child_mut");
        }
    }
    if let Some(span) = get_child.as_ref().or_else(|| get_child_mut.as_ref()) {
        if num_children.is_none() {
            emit_warning!(span, "fn get_child[_mut] without fn num_children");
        }
        if opt_derive.is_some() {
            emit_error!(span, "impl conflicts with use of #[widget(derive=FIELD)]");
        }
        if !children.is_empty() {
            emit_error!(
                span,
                "custom `Widget::get_child` implementation when using `#[widget]` on fields"
            );
        } else if !layout_children.is_empty() {
            emit_error!(
                span,
                "custom `Widget::get_child` implementation when using layout-defined children"
            );
        }
    }
    let do_impl_widget_children = get_child.is_none() && get_child_mut.is_none();

    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let impl_generics = impl_generics.to_token_stream();
    let impl_target = quote! { #name #ty_generics #where_clause };
    let widget_name = name.to_string();

    let mut required_layout_methods = quote! {};
    let mut fn_size_rules = None;
    let mut fn_translation = None;
    let (fn_set_rect, fn_nav_next, fn_find_id);
    let mut fn_nav_next_err = None;
    let mut fn_draw = None;
    let mut gen_layout = false;

    if let Some(inner) = opt_derive {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::WidgetCore for #impl_target
            {
                #[inline]
                fn id_ref(&self) -> &::kas::WidgetId {
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
            }
        });

        required_layout_methods = quote! {
            fn num_children(&self) -> usize {
                self.#inner.num_children()
            }
            #[inline]
            fn find_child_index(&self, id: &::kas::WidgetId) -> Option<usize> {
                self.#inner.find_child_index(id)
            }
            #[inline]
            fn make_child_id(&mut self, index: usize) -> ::kas::WidgetId {
                self.#inner.make_child_id(index)
            }
        };

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
            ) {
                self.#inner.set_rect(mgr, rect);
            }
        };
        fn_nav_next = Some(quote! {
            fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
                self.#inner.nav_next(reverse, from)
            }
        });
        fn_translation = Some(quote! {
            #[inline]
            fn translation(&self) -> ::kas::geom::Offset {
                self.#inner.translation()
            }
        });
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

        // Widget methods are derived. Cost: cannot override any Events methods or translation().
        let fns_as_node = widget_as_node_methods();
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Widget for #impl_target {
                #derive_ty_def
                #fns_as_node

                #[inline]
                fn get_child(&self, index: usize) -> Option<::kas::Node<'_>> {
                    self.#inner.get_child(index)
                }
                #[inline]
                fn get_child_mut(&mut self, index: usize) -> Option<::kas::NodeMut<'_>> {
                    self.#inner.get_child_mut(index)
                }

                fn _configure(
                    &mut self,
                    cx: &mut ::kas::event::ConfigMgr,
                    id: ::kas::WidgetId,
                ) {
                    self.#inner._configure(cx, id);
                }

                fn _broadcast(
                    &mut self,
                    cx: &mut ::kas::event::EventMgr,
                    count: &mut usize,
                    event: ::kas::event::Event,
                ) {
                    self.#inner._broadcast(cx, count, event);
                }

                fn _send(
                    &mut self,
                    cx: &mut ::kas::event::EventMgr,
                    id: ::kas::WidgetId,
                    disabled: bool,
                    event: ::kas::event::Event,
                ) -> ::kas::event::Response {
                    self.#inner._send(cx, id, disabled, event)
                }

                fn _replay(
                    &mut self,
                    cx: &mut ::kas::event::EventMgr,
                    id: ::kas::WidgetId,
                    msg: ::kas::Erased,
                ) {
                    self.#inner._replay(cx, id, msg);
                }

                fn _nav_next(
                    &mut self,
                    cx: &mut ::kas::event::EventMgr,
                    focus: Option<&::kas::WidgetId>,
                    advance: ::kas::NavAdvance,
                ) -> Option<::kas::WidgetId> {
                    self.#inner._nav_next(cx, focus, advance)
                }
            }
        });
    } else {
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

        scope.generated.push(impl_core(
            &impl_generics,
            &impl_target,
            &widget_name,
            &core_path,
        ));

        if do_impl_widget_children {
            let count = children.len() + layout_children.len();
            required_layout_methods = quote! {
                fn num_children(&self) -> usize {
                    #count
                }
            };
        }

        if let Some(index) = widget_impl {
            use syn::ImplItem::Verbatim;
            let widget_impl = &mut scope.impls[index];
            if do_data_ty_impl {
                let data_ty = data_ty.as_ref().unwrap();
                widget_impl.items.push(Verbatim(quote! {
                    type Data = #data_ty;
                }));
            }
            widget_impl.items.push(Verbatim(widget_as_node_methods()));
            if do_recursive_methods {
                widget_impl.items.push(Verbatim(widget_recursive_methods()));
            }
        } else {
            scope.generated.push(impl_widget(
                &impl_generics,
                &impl_target,
                &data_ty.expect("has data_ty"),
                &core_path,
                &children,
                layout_children,
                do_impl_widget_children,
                do_recursive_methods,
            ));
        }

        let mut set_rect = quote! { self.#core.rect = rect; };
        let mut find_id = quote! {
            use ::kas::{WidgetCore, WidgetExt};
            self.rect().contains(coord).then(|| self.id())
        };
        if let Some((_, layout)) = args.layout.take() {
            gen_layout = true;
            fn_nav_next = match layout.nav_next(children.iter()) {
                Ok(toks) => Some(toks),
                Err((span, msg)) => {
                    fn_nav_next_err = Some((span, msg));
                    None
                }
            };

            let layout_methods = layout.layout_methods(&quote! { self.#core })?;
            scope.generated.push(quote! {
                impl #impl_generics ::kas::layout::AutoLayout for #impl_target {
                    #layout_methods
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
                <Self as ::kas::layout::AutoLayout>::set_rect(self, mgr, rect);
            };
            find_id = quote! { <Self as ::kas::layout::AutoLayout>::find_id(self, coord) };
            fn_draw = Some(quote! {
                fn draw(&mut self, draw: ::kas::theme::DrawMgr) {
                    <Self as ::kas::layout::AutoLayout>::draw(self, draw);
                }
            });
        } else {
            fn_nav_next = Some(quote! {
                fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
                    ::kas::util::nav_next(reverse, from, self.num_children())
                }
            });
        }
        fn_set_rect = quote! {
            fn set_rect(
                &mut self,
                mgr: &mut ::kas::event::ConfigMgr,
                rect: ::kas::geom::Rect,
            ) {
                #set_rect
            }
        };
        fn_find_id = quote! {
            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::WidgetId> {
                #find_id
            }
        };

        let fn_pre_configure = quote! {
            fn pre_configure(&mut self, _: &mut ::kas::event::ConfigMgr, id: ::kas::WidgetId) {
                self.#core.id = id;
            }
        };

        let fn_navigable = args.navigable;
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
        let fn_pre_handle_event = quote! {
            fn pre_handle_event(
                &mut self,
                mgr: &mut ::kas::event::EventMgr,
                event: ::kas::event::Event,
            ) -> ::kas::event::Response {
                use ::kas::{event::{Event, Response, Scroll}, WidgetExt, WidgetCore};
                if event == Event::NavFocus(true) {
                    mgr.set_scroll(Scroll::Rect(self.rect()));
                }
                #pre_handle_event
                self.handle_event(mgr, event)
            }
        };
        let fn_handle_event = None;

        if let Some(index) = events_impl {
            let events_impl = &mut scope.impls[index];
            let method_idents = collect_idents(events_impl);
            let has_method = |name| method_idents.iter().any(|ident| ident == name);

            if opt_derive.is_some() || !has_method("pre_configure") {
                events_impl.items.push(parse2(fn_pre_configure)?);
            }
            if let Some(method) = fn_navigable {
                events_impl.items.push(parse2(method)?);
            }
            events_impl.items.push(parse2(fn_pre_handle_event)?);
            if let Some(item) = fn_handle_event {
                events_impl.items.push(parse2(item)?);
            }
        } else {
            scope.generated.push(quote! {
                impl #impl_generics ::kas::Events for #impl_target {
                    type Data = ();
                    #fn_pre_configure
                    #fn_navigable
                    #fn_pre_handle_event
                    #fn_handle_event
                }
            });
        }
    }

    if let Some(index) = layout_impl {
        use syn::ImplItem::Verbatim;

        let layout_impl = &mut scope.impls[index];
        let method_idents = collect_idents(layout_impl);
        let has_method = |name| method_idents.iter().any(|ident| ident == name);

        layout_impl.items.push(Verbatim(required_layout_methods));

        if let Some(method) = fn_size_rules {
            if !has_method("size_rules") {
                layout_impl.items.push(Verbatim(method));
            }
        }
        if !has_method("set_rect") {
            layout_impl.items.push(Verbatim(fn_set_rect));
        }

        if !has_method("nav_next") {
            if let Some(method) = fn_nav_next {
                layout_impl.items.push(Verbatim(method));
            } else if gen_layout {
                // We emit a warning here only if nav_next is not explicitly defined
                let (span, msg) = fn_nav_next_err.unwrap();
                emit_warning!(span, "unable to generate `fn Layout::nav_next`: {}", msg,);
            }
        }

        if let Some(ident) = method_idents.iter().find(|ident| *ident == "translation") {
            if opt_derive.is_some() {
                emit_error!(ident, "method not supported in derive mode");
            }
        } else if let Some(method) = fn_translation {
            layout_impl.items.push(Verbatim(method));
        }

        if !has_method("find_id") {
            layout_impl.items.push(Verbatim(fn_find_id));
        }
        if let Some(method) = fn_draw {
            if !has_method("draw") {
                layout_impl.items.push(Verbatim(method));
            }
        }
    } else if let Some(fn_size_rules) = fn_size_rules {
        scope.generated.push(quote! {
            impl #impl_generics ::kas::Layout for #impl_target {
                #required_layout_methods
                #fn_size_rules
                #fn_set_rect
                #fn_nav_next
                #fn_translation
                #fn_find_id
                #fn_draw
            }
        });
    }

    // println!("{}", scope.to_token_stream());
    Ok(())
}

fn collect_idents(item_impl: &ItemImpl) -> Vec<Ident> {
    item_impl
        .items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(m) => Some(m.sig.ident.clone()),
            _ => None,
        })
        .collect()
}

pub fn impl_core(impl_generics: &Toks, impl_target: &Toks, name: &str, core_path: &Toks) -> Toks {
    quote! {
        impl #impl_generics ::kas::WidgetCore for #impl_target {
            #[inline]
            fn id_ref(&self) -> &::kas::WidgetId {
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
}

pub fn impl_widget(
    impl_generics: &Toks,
    impl_target: &Toks,
    data_ty: &Type,
    core_path: &Toks,
    children: &Vec<Member>,
    layout_children: Vec<Toks>,
    do_impl_widget_children: bool,
    do_recursive_methods: bool,
) -> Toks {
    let fns_as_node = widget_as_node_methods();

    let fns_get_child = if do_impl_widget_children {
        let count = children.len();
        let mut get_rules = quote! {};
        let mut get_mut_rules = quote! {};
        for (i, child) in children.iter().enumerate() {
            let ident = child;
            get_rules.append_all(quote! { #i => Some(self.#ident.as_node()), });
            get_mut_rules.append_all(quote! { #i => Some(self.#ident.as_node_mut()), });
        }
        for (i, path) in layout_children.iter().enumerate() {
            let index = count + i;
            get_rules.append_all(quote! { #index => Some(#core_path.#path.as_node()), });
            get_mut_rules.append_all(quote! { #index => Some(#core_path.#path.as_node_mut()), });
        }

        quote! {
            fn get_child(&self, _index: usize) -> Option<::kas::Node<'_>> {
                use ::kas::WidgetCore;
                match _index {
                    #get_rules
                    _ => None
                }
            }
            fn get_child_mut(&mut self, _index: usize) -> Option<::kas::NodeMut<'_>> {
                use ::kas::WidgetCore;
                match _index {
                    #get_mut_rules
                    _ => None
                }
            }
        }
    } else {
        quote! {}
    };

    let fns_recurse = if do_recursive_methods {
        widget_recursive_methods()
    } else {
        quote! {}
    };

    quote! {
        impl #impl_generics ::kas::Widget for #impl_target {
            type Data = #data_ty;
            #fns_as_node
            #fns_get_child
            #fns_recurse
        }
    }
}

fn widget_as_node_methods() -> Toks {
    quote! {
        #[inline]
        fn as_node(&self) -> ::kas::Node<'_> { ::kas::Node::new(self) }
        #[inline]
        fn as_node_mut(&mut self) -> ::kas::NodeMut<'_> {
            ::kas::NodeMut::new(self)
        }
    }
}

fn widget_recursive_methods() -> Toks {
    quote! {
        fn _configure(
            &mut self,
            cx: &mut ::kas::event::ConfigMgr,
            id: ::kas::WidgetId,
        ) {
            ::kas::impls::_configure(self, cx, id);
        }

        fn _broadcast(
            &mut self,
            cx: &mut ::kas::event::EventMgr,
            count: &mut usize,
            event: ::kas::event::Event,
        ) {
            ::kas::impls::_broadcast(self, cx, count, event);
        }

        fn _send(
            &mut self,
            cx: &mut ::kas::event::EventMgr,
            id: ::kas::WidgetId,
            disabled: bool,
            event: ::kas::event::Event,
        ) -> ::kas::event::Response {
            ::kas::impls::_send(self, cx, id, disabled, event)
        }

        fn _replay(
            &mut self,
            cx: &mut ::kas::event::EventMgr,
            id: ::kas::WidgetId,
            msg: ::kas::Erased,
        ) {
            ::kas::impls::_replay(self, cx, id, msg);
        }

        fn _nav_next(
            &mut self,
            cx: &mut ::kas::event::EventMgr,
            focus: Option<&::kas::WidgetId>,
            advance: ::kas::NavAdvance,
        ) -> Option<::kas::WidgetId> {
            ::kas::impls::_nav_next(self, cx, focus, advance)
        }
    }
}
