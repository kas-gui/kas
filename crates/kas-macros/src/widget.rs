// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use crate::make_layout;
use impl_tools_lib::fields::{Fields, FieldsNamed, FieldsUnnamed};
use impl_tools_lib::scope::{Scope, ScopeAttr, ScopeItem};
use impl_tools_lib::SimplePath;
use proc_macro2::{Span, TokenStream as Toks};
use proc_macro_error2::{emit_error, emit_warning};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Eq;
use syn::ImplItem::{self, Verbatim};
use syn::{parse2, parse_quote};
use syn::{Expr, FnArg, Ident, Index, ItemImpl, MacroDelimiter, Member, Meta, Pat, Token, Type};

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
    #[allow(dead_code)]
    pub kw_span: Span,
    #[allow(dead_code)]
    pub eq: Eq,
    pub lit: syn::LitBool,
}

#[derive(Debug)]
pub struct ExprToken {
    #[allow(dead_code)]
    pub kw_span: Span,
    #[allow(dead_code)]
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
        let span = attr.span();
        let args = match &attr.meta {
            Meta::Path(_) => WidgetArgs::default(),
            _ => attr.parse_args()?,
        };
        widget(span, args, scope)
    }
}

#[derive(Debug)]
pub enum ChildIdent {
    /// Child is a direct field
    Field(Member),
    /// Child is a hidden field (under #core_path)
    CoreField(Member),
}
impl ChildIdent {
    pub fn get_rule(&self, core_path: &Toks, i: usize) -> Toks {
        match self {
            ChildIdent::Field(ident) => quote! { #i => Some(self.#ident.as_layout()), },
            ChildIdent::CoreField(ident) => quote! { #i => Some(#core_path.#ident.as_layout()), },
        }
    }
}

pub struct Child {
    pub ident: ChildIdent,
    pub attr_span: Option<Span>,
    pub data_binding: Option<Expr>,
}
impl Child {
    pub fn new_core(ident: Member) -> Self {
        Child {
            ident: ChildIdent::CoreField(ident),
            attr_span: None,
            data_binding: None,
        }
    }
}

/// Custom widget definition
///
/// This macro may inject impls and inject items into existing impls.
/// It may also inject code into existing methods such that the only observable
/// behaviour is a panic.
pub fn widget(attr_span: Span, mut args: WidgetArgs, scope: &mut Scope) -> Result<()> {
    scope.expand_impl_self();
    let name = &scope.ident;
    let opt_derive = &args.derive;
    let mut data_ty = args.data_ty;

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

                if let Some(mem) = opt_derive {
                    emit_error!(
                        mem, "derive is incompatible with Events impl";
                        note = path.span() => "this Events impl";
                    );
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

    let data_ty = if let Some(ident) = opt_derive.as_ref() {
        'outer: {
            for (i, field) in fields.iter_mut().enumerate() {
                if *ident == member(i, field.ident.clone()) {
                    let ty = &field.ty;
                    break 'outer parse_quote! { <#ty as ::kas::Widget>::Data };
                }
            }
            return Err(Error::new(ident.span(), "field not found"));
        }
    } else if let Some(ty) = data_ty {
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
            if let Some((_, ref layout)) = args.layout {
                stor_defs = layout.storage_fields(&mut children, &data_ty);
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
                if Some(&ident) == opt_derive.as_ref() {
                    emit_error!(attr, "#[widget] must not be used on widget derive target");
                }
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
        if opt_derive.is_some() {
            emit_error!(span, "impl forbidden when using #[widget(derive=FIELD)]");
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
    let do_impl_widget_children = get_child.is_none() && for_child_node.is_none();

    let (impl_generics, ty_generics, where_clause) = scope.generics.split_for_impl();
    let impl_generics = impl_generics.to_token_stream();
    let impl_target = quote! { #name #ty_generics #where_clause };
    let widget_name = name.to_string();

    let mut required_layout_methods;
    let mut fn_size_rules = None;
    let mut fn_translation = None;
    let (fn_set_rect, fn_nav_next, fn_find_id);
    let mut fn_nav_next_err = None;
    let mut fn_draw = None;

    if let Some(inner) = opt_derive {
        required_layout_methods = quote! {
            #[inline]
            fn as_layout(&self) -> &dyn Layout {
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
            fn get_child(&self, index: usize) -> Option<&dyn Layout> {
                self.#inner.get_child(index)
            }
            #[inline]
            fn find_child_index(&self, id: &::kas::Id) -> Option<usize> {
                self.#inner.find_child_index(id)
            }
        };

        fn_size_rules = Some(quote! {
            #[inline]
            fn size_rules(&mut self,
                sizer: ::kas::theme::SizeCx,
                axis: ::kas::layout::AxisInfo,
            ) -> ::kas::layout::SizeRules {
                self.#inner.size_rules(sizer, axis)
            }
        });
        fn_set_rect = quote! {
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
            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                self.#inner.find_id(coord)
            }
        };
        fn_draw = Some(quote! {
            #[inline]
            fn draw(&mut self, draw: ::kas::theme::DrawCx) {
                self.#inner.draw(draw);
            }
        });

        // Widget methods are derived. Cost: cannot override any Events methods or translation().
        let fns_as_node = widget_as_node();
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
    } else {
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

        required_layout_methods = impl_core_methods(&widget_name, &core_path);

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

            widget_impl.items.push(Verbatim(widget_as_node()));
            if !has_item("_send") {
                widget_impl
                    .items
                    .push(Verbatim(widget_recursive_methods(&core_path)));
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

        let mut set_rect = quote! { self.#core.rect = rect; };
        let mut find_id = quote! {
            use ::kas::{Layout, LayoutExt};
            self.rect().contains(coord).then(|| self.id())
        };
        if let Some((_, layout)) = args.layout.take() {
            fn_nav_next = match layout.nav_next(children.iter()) {
                Ok(toks) => Some(toks),
                Err((span, msg)) => {
                    fn_nav_next_err = Some((span, msg));
                    None
                }
            };

            let layout_visitor = layout.layout_visitor(&core_path)?;
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
            find_id = quote! {
                use ::kas::{Layout, LayoutExt, layout::LayoutVisitor};

                if !self.rect().contains(coord) {
                    return None;
                }
                let coord = coord + self.translation();
                self.layout_visitor()
                    .find_id(coord)
                    .or_else(|| Some(self.id()))
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
        fn_set_rect = quote! {
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
        fn_find_id = quote! {
            fn find_id(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
                #[cfg(debug_assertions)]
                #core_path.status.require_rect(&#core_path.id);

                #find_id
            }
        };

        let fn_navigable = args.navigable;
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

        if let Some(ident) = item_idents
            .iter()
            .find_map(|(_, ident)| (*ident == "translation").then_some(ident))
        {
            if opt_derive.is_some() {
                emit_error!(ident, "method not supported in derive mode");
            }
        } else if let Some(method) = fn_translation {
            layout_impl.items.push(Verbatim(method));
        }

        if let Some((index, _)) = item_idents.iter().find(|(_, ident)| *ident == "find_id") {
            if let Some(ref core) = core_data {
                if let ImplItem::Fn(f) = &mut layout_impl.items[*index] {
                    f.block.stmts.insert(0, parse_quote! {
                        #[cfg(debug_assertions)]
                        self.#core.status.require_rect(&self.#core.id);
                    });
                }
            }
        } else {
            layout_impl.items.push(Verbatim(fn_find_id));
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
                #fn_translation
                #fn_find_id
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

fn collect_idents(item_impl: &ItemImpl) -> Vec<(usize, Ident)> {
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

    quote! {
        impl #impl_generics ::kas::Widget for #impl_target {
            type Data = #data_ty;
            #fns_as_node
            #fns_for_child
            #fns_recurse
        }
    }
}

fn widget_as_node() -> Toks {
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
