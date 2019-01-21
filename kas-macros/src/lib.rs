// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#![recursion_limit="128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

mod args;

use std::env;
use std::fmt::Write;
use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{DeriveInput, FnArg, Ident, ImplItemMethod, Type, TypePath};
use syn::{parse_quote, parse_macro_input};
    use syn::spanned::Spanned;
use syn::token::Comma;
use syn::punctuated::Punctuated;

use self::args::ChildType;

#[cfg(not(feature = "cassowary"))] mod layout_extern;
#[cfg(not(feature = "cassowary"))] use self::layout_extern as layout;

#[cfg(feature = "cassowary")] mod layout_cw;
#[cfg(feature = "cassowary")] use self::layout_cw as layout;

/// Macro to derive widget traits
/// 
/// The [`Widget`] trait requires multiple base traits to be implemented:
/// [`Core`] and [`Layout`]. These base traits should be considered
/// implementation details and not used directly; this macro therefore
/// implements both base traits and [`Widget`] directly.
/// 
/// Additionally, widgets are usually required to implement the [`Handler`]
/// trait. If (and only if) the deriving struct is marked with a
/// `#[handler]` attribute, the [`Handler`] trait will also be implemented.
/// Note that it is also possible to implement this trait manually.
/// 
/// ### Type attributes
/// 
/// This `derive` attribute may only be used on structs. This struct must have
/// a `#[widget]` attribute and may have a `#[handler]` attribute, as follows.
/// 
/// ```notest
/// #[widget(class = Class::X, ...)]
/// #[derive(Clone, Debug, Widget)]
/// struct MyWidget {
///     ...
/// }
/// ```
/// 
/// The `#[widget]` attribute on the struct supports the following arguments:
/// 
/// -   `class = ...` (required) — an expression yielding the widget's [`Class`]
/// -   `layout = ...` (optional) — see below
/// 
/// Widgets with no children or only a single child do not need to specify the
/// `layout` (or may specify `single`), but those with multiple children must
/// specify this arguments, as follows:
/// 
/// -   `single` — single child only
/// -   `horizontal` — widgets are laid out in a row from left to right in the
///     order specified
/// -   `vertical` — same except top-to-bottom
/// -   `grid` — widgets are placed on a grid, with position specified by the
///     per-field `#[widget]` attribute
/// 
/// If there is a `#[handler]` attribute on the struct, then the [`Handler`]
/// trait will be implemented. This attribute expects the following arguments:
/// 
/// -   `response = ...` — the [`Handler<Response>`] associated type
/// -   `generics = < X, Y, ... > where CONDS` — see below
/// 
/// Commonly the [`Handler`] implementation requires extra bounds on generic
/// types, and sometimes also additional type parameters; the `generics`
/// argument allows this. This argument is optional and if present must be the
/// last argument. Note that the generic types and bounds given are *added to*
/// the generics defined on the struct itself. An example:
/// 
/// ```notest
/// struct MyWidget<W: Widget> {
///     #[core] core: CoreData,
///     #[widget(handler = handler)] child: W,
/// }
/// 
/// impl<W: Widget> MyWidget {
///     fn handler(&mut self, tk: &TkWidget, msg: ChildMessage) -> MyMessage {
///         // handle msg and respond somehow
///         ignore(msg)
///     }
/// }
/// ```
/// 
/// ### Fields
/// 
/// One struct field must be marked with `#[core]` and implement the [`Core`]
/// trait; usually this field has the specification `#[core] core: CoreData`.
/// 
/// A `#[widget]` attribute is used to denote fields as child widgets. This
/// attribute accepts the following optional arguments, for use with `grid`
/// layouts and for handlers:
/// 
/// -   `col = ...` — grid column, from left (defaults to 0)
/// -   `row = ...` — grid row, from top (defaults to 0)
/// -   `cspan = ...` — number of columns to span (defaults to 1)
/// -   `rspan = ...` — number of rows to span (defaults to 1)
/// -   `handler = ...` — the name (`f`) of a method defined on this type which
///     handles a message from the child (type `M`) and converts it to the
///     appropriate response type for this widget (`R`); this method should have
///     signature `fn f(&mut self, tk: &TkWidget, msg: M) -> R`.
///     
/// 
/// Example:
/// 
/// ```notest
/// #[widget(class = Class::Window, layout = single)]
/// #[handler(response = MyMessage,
///         generics = <> where W: Handler<ChildMessage>)]
/// #[derive(Widget)]
/// struct MyWidget<W: Widget> {
///     #[core] core: CoreData,
///     #[widget(handler = handler)] child: W,
/// }
/// 
/// impl<W: Widget> SimpleWindow<W> {
///     fn handle_msg(&mut self, tk: &TkWidget, msg: ChildMsg) -> MyResponse {
///         println!("Recieved message: {:?}", msg);
///         MyResponse::None
///     }
/// }
/// ```
/// 
/// Note: usage of this macro currently requires `#![feature(unrestricted_attribute_tokens)]`.
/// 
/// [`Class`]: ../kas/widget/enum.Class.html
/// [`Core`]: ../kas/widget/trait.Core.html
/// [`CoreData`]: ../kas/widget/struct.CoreData.html
/// [`Layout`]: ../kas/widget/trait.Layout.html
/// [`Widget`]: ../kas/widget/trait.Widget.html
/// [`Handler`]: ../kas/event/trait.Handler.html
/// [`Handler<Response>`]: TODO
#[proc_macro_derive(Widget, attributes(core, widget, handler))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let c = c();
    
    let args = match args::read_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let core = args.core;
    let class = args.widget.class;
    let count = args.children.len();
    
    let layout_fns = match layout::fns(&c, &args.children, args.widget.layout) {
        Ok(fns) => fns,
        Err(err) => return err.to_compile_error().into(),
    };
    
    fn make_match_rules(children: &Vec<args::Child>, mut_ref: TokenStream) -> TokenStream {
        let mut toks = TokenStream::new();
        for (i, child) in children.iter().enumerate() {
            let ident = &child.ident;
            toks.append_all(quote!{ #i => Some(&#mut_ref self.#ident), });
        }
        toks
    };
    let get_rules = make_match_rules(&args.children, quote!{});
    let get_mut_rules = make_match_rules(&args.children, quote!{mut});
    
    let mut toks = quote! {
        impl #impl_generics #c::Core
            for #name #ty_generics #where_clause
        {
            fn number(&self) -> u32 {
                use #c::Core;
                self.#core.number()
            }
            fn set_number(&mut self, number: u32) {
                use #c::Core;
                self.#core.set_number(number);
            }
            
            fn tkd(&self) -> #c::TkData {
                use #c::Core;
                self.#core.tkd()
            }
            fn set_tkd(&mut self, tkd: #c::TkData) {
                use #c::Core;
                self.#core.set_tkd(tkd)
            }
            
            fn rect(&self) -> &#c::Rect {
                use #c::Core;
                self.#core.rect()
            }
            fn rect_mut(&mut self) -> &mut #c::Rect {
                use #c::Core;
                self.#core.rect_mut()
            }
        }
        
        impl #impl_generics #c::Layout
                for #name #ty_generics #where_clause
        {
            #layout_fns
        }
        
        impl #impl_generics #c::Widget
                for #name #ty_generics #where_clause
        {
            fn class(&self) -> #c::Class { #class }

            fn len(&self) -> usize {
                #count
            }
            fn get(&self, _index: usize) -> Option<&#c::Widget> {
                match _index {
                    #get_rules
                    _ => None
                }
            }
            fn get_mut(&mut self, _index: usize) -> Option<&mut #c::Widget> {
                match _index {
                    #get_mut_rules
                    _ => None
                }
            }
        }
    };
    
    if let Some(handler) = args.handler {
        let response = handler.response;
        let mut generics = ast.generics.clone();
        if !handler.generics.params.is_empty() {
            if !generics.params.empty_or_trailing() {
                generics.params.push_punct(Default::default());
            }
            generics.params.extend(handler.generics.params.into_pairs());
        }
        if let Some(h_clauses) = handler.generics.where_clause {
            if let Some(ref mut clauses) = generics.where_clause {
                if !clauses.predicates.empty_or_trailing() {
                    clauses.predicates.push_punct(Default::default());
                }
                clauses.predicates.extend(h_clauses.predicates.into_pairs());
            } else {
                generics.where_clause = Some(h_clauses);
            }
        }
        // Note: we may have extra generic types used in where clauses, but we
        // don't want these in ty_generics.
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        
        let mut handler_toks = TokenStream::new();
        for child in args.children.iter() {
            let ident = &child.ident;
            let handler = if let Some(ref h) = child.args.handler {
                quote!{ self.#h(_tk, msg) }
            } else {
                quote!{ msg.into() }
            };
            handler_toks.append_all(quote!{
                else if num <= self.#ident.number() {
                    let msg = self.#ident.handle_action(_tk, action, num);
                    #handler
                }
            });
        }
        
        toks.append_all(quote! {
            impl #impl_generics #c::event::Handler
                    for #name #ty_generics #where_clause
            {
                type Response = #response;
                
                fn handle_action(&mut self, _tk: &#c::TkWidget, action: #c::event::Action,
                        num: u32) -> Self::Response
                {
                    use #c::{Core, event::{Handler, err_unhandled, err_num}};
                    
                    if num == self.number() {
                        // we may want to allow custom handlers on self here?
                        err_unhandled(action)
                    }
                    #handler_toks
                    else {
                        err_num()
                    }
                }
            }
        });
    };
    
    toks.into()
}

/// Macro to create a widget with anonymous type
/// 
/// This exists purely to save you some typing. You could instead make your own
/// struct, derive `Widget` (with attributes to enable Core, Layout and Widget
/// implementation), manually implement `event::Handler`, and instantiate an
/// object.
/// 
/// Syntax should match the following Backus-Naur Form:
/// 
/// ```bnf
/// <input>     ::= <layout> "=>" <response> ";" <class_spec> <fields> ";" <funcs>
/// <layout>    ::= "single" | "horizontal" | "vertical" | "grid"
/// <response>  ::= <type>
/// <class_spec> ::= "" | "class" "=" <path> ";"
/// <fields>    ::= "" | <field> | <field> "," <fields>
/// <field>     ::= <w_attr> <opt_ident> <field_ty> = <expr>
/// <opt_ident> ::= "_" | <ident>
/// <field_ty>  ::= "" | ":" <type> | ":" impl <bound> | "->" <type> | ":" impl <bound> "->" <type>
/// <w_attr>    ::= "" | "#" "[" <widget> <w_params> "]"
/// <w_params>  ::= "" | "(" <w_args> ")"
/// <w_args>    ::= <w_arg> | <w_arg> "," <w_args>
/// <w_arg>     ::= <pos_arg> "=" <lit> | "handler" = <ident>
/// <pos_arg>   ::= "col" | "row" | "cspan" | "rspan"
/// <funcs>     ::= "" | <func> <funcs>
/// ```
/// where `<type>` is a type expression, `<expr>` is a (value) expression,
/// `<ident>` is an identifier, `<lit>` is a literal, `<path>` is a path,
/// `<bound>` is a trait object bound, and
/// `<func>` is a Rust method definition. `""` is the empty string (i.e. nothing).
/// 
/// The effect of this macro is to create an anonymous struct with the above
/// fields (plus an implicit `core`), implement [`Core`], [`Layout`], [`Widget`]
/// and [`Handler`] (with the specified `<response>` type), implement the
/// additional `<funcs>` listed on this type, then construct and return an
/// instance using the given value expressions to initialise each field.
/// 
/// Each field is considered a child widget if the `#[widget]` attribute is
/// present, or a simple data field otherwise. The specification of this
/// attribute is identical to that used when deriving `Widget`.
/// 
/// The `layout` specifier should be self-explanatory, with the exception of
/// `grid`, where each widget's position must be specified via attribute
/// arguments (e.g. `#[widget(col=1, row=2)]`). The `col` and `row` parameters
/// both default to 0, while `cspan` and `rspan` (column and row spans) both
/// default to 1.
/// 
/// Fields may have an identifier or may be anonymous (via usage of `_`). This
/// is often convenient for child widgets which don't need to be referred to.
/// 
/// Fields may have an explicit type (`ident : type = ...`), or the type may be
/// skipped, or (for widgets only) just the response type can be specified via
/// `ident -> type = ...`. Note that some type specification is usually needed
/// when referring to the field later.
/// 
/// Optionally, a message handler may be specified for child widgets via
/// `#[widget(handler = f)] ident = value` where `f` is a method defined on the
/// anonymous struct with signature `fn f(&mut self, tk: &TkWidget, msg: M) -> R`
/// where `M` is the type of response received from the child widget, and `R` is
/// the type of response sent from this widget.
/// 
/// Currently usage of this macro requires `#![feature(proc_macro_hygiene)]`.
/// 
/// [`Core`]: ../kas/widget/trait.Core.html
/// [`Layout`]: ../kas/widget/trait.Layout.html
/// [`Widget`]: ../kas/widget/trait.Widget.html
/// [`Handler`]: ../kas/event/trait.Handler.html
#[proc_macro]
pub fn make_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut find_handler_ty_buf: Vec<(Ident, Type)> = vec![];
    // find type of handler's message; return None on error
    let mut find_handler_ty = |handler: &Ident,
            impls: &Vec<(Option<TypePath>, Vec<ImplItemMethod>)>|
            -> Option<Type>
    {
        // check the buffer in case we did this already
        for (ident, ty) in &find_handler_ty_buf {
            if ident == handler {
                return Some(ty.clone());
            }
        }
        
        let mut x: Option<(Ident, Type)> = None;
        
        for impl_block in impls {
            for f in &impl_block.1 {
                if f.sig.ident == *handler {
                    if let Some(x) = x {
                        handler.span()
                            .unstable()
                            .error("multiple methods with this name")
                            .emit();
                        x.0.span()
                            .unstable()
                            .error("first method with this name")
                            .emit();
                        f.sig.ident.span()
                            .unstable()
                            .error("second method with this name")
                            .emit();
                        return None;
                    }
                    if f.sig.decl.inputs.len() != 3 {
                        f.sig.span()
                            .unstable()
                            .error("handler functions must have signature: fn handler(&mut self, tk: &TkWidget, msg: T)")
                            .emit();
                        return None;
                    }
                    let pair = f.sig.decl.inputs.last().unwrap();
                    let arg = pair.value();
                    let ty = match arg {
                        FnArg::Captured(arg) => arg.ty.clone(),
                        _ => panic!("expected captured argument"),  // nothing else is possible here?
                    };
                    x = Some((f.sig.ident.clone(), ty));
                }
            }
        }
        if let Some(x) = x {
            find_handler_ty_buf.push((handler.clone(), x.1.clone()));
            Some(x.1)
        } else {
            handler.span()
                .unstable()
                .error("no methods with this name found")
                .emit();
            None
        }
    };
    
    let mut args = parse_macro_input!(input as args::MakeWidget);
    
    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    
    let c = c();
    
    // fields of anonymous struct:
    let mut field_toks = quote!{ #[core] core: #c::CoreData, };
    // initialisers for these fields:
    let mut field_val_toks = quote!{ core: Default::default(), };
    // debug impl
    let mut debug_fields = TokenStream::new();
    
    // generic types on struct, without constraints:
    let mut gen_tys = Punctuated::<_, Comma>::new();
    // generic types on struct, with constraints:
    let mut gen_ptrs = Punctuated::<_, Comma>::new();
    // extra generic types and where clause for handler impl
    let mut handler_extra = Punctuated::<_, Comma>::new();
    let mut handler_clauses = Punctuated::<_, Comma>::new();
    
    let layout = &args.layout;
    let response = &args.response;
    
    for (index, field) in args.fields.drain(..).enumerate() {
        let attr = &field.widget_attr;
        
        let ident = match &field.ident {
            Some(ref ident) => ident.clone(),
            None => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("mw_anon_{}", index)).unwrap();
                Ident::new(&name_buf, Span::call_site())
            }
        };
        
        let ty: Type = match field.ty {
            ChildType::Fixed(ty) => ty.clone(),
            ChildType::Generic(gen_response, gen_bound) => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("MWAnon{}", index)).unwrap();
                let ty = Ident::new(&name_buf, Span::call_site());
                
                gen_tys.push(ty.clone());
                if let Some(ref wattr) = attr {
                    if let Some(tyr) = gen_response {
                        handler_clauses.push(quote!{ #ty: #c::event::Handler<Response = #tyr> });
                    } else {
                        // No typing. If a handler is specified, then the child must implement
                        // Handler<Response = X> where the handler takes type X; otherwise
                        // we use `msg.into()` and this conversion must be supported.
                        if let Some(ref handler) = wattr.args.handler {
                            if let Some(ty_bound) = find_handler_ty(handler, &args.impls) {
                                handler_clauses.push(quote!{ #ty: #c::event::Handler<Response = #ty_bound> });
                            } else {
                                return quote!{}.into(); // exit after emitting error
                            }
                        } else {
                            name_buf.push_str("R");
                            let tyr = Ident::new(&name_buf, Span::call_site());
                            handler_extra.push(tyr.clone());
                            handler_clauses.push(quote!{ #ty: #c::event::Handler<Response = #tyr> });
                            handler_clauses.push(quote!{ #tyr: From<#c::event::NoResponse> });
                            handler_clauses.push(quote!{ #response: From<#tyr> });
                        }
                    }
                    
                    if let Some(mut bound) = gen_bound {
                        bound.bounds.push(parse_quote!{ #c::Widget });
                        gen_ptrs.push(quote!{ #ty: #bound });
                    } else {
                        gen_ptrs.push(quote!{ #ty: #c::Widget });
                    }
                } else {
                    gen_ptrs.push(quote!{ #ty });
                }
                
                Type::Path(TypePath { qself: None, path: ty.into() })
            }
        };
        
        let value = &field.value;
        
        field_toks.append_all(quote!{ #attr #ident: #ty, });
        field_val_toks.append_all(quote!{ #ident: #value, });
        debug_fields.append_all(quote!{ write!(f, ", {}: {:?}", stringify!(#ident), self.#ident)?; });
    }
    
    let handler_where = if handler_clauses.is_empty() {
        quote!{}
    } else {
        quote!{ where #handler_clauses }
    };
    
    let mut impls = TokenStream::new();
    for impl_block in args.impls {
        let mut contents = TokenStream::new();
        for method in impl_block.1 {
            contents.append_all(std::iter::once(method));
        }
        let target = if let Some(t) = impl_block.0 {
            quote!{ #t for }
        } else {
            quote!{}
        };
        impls.append_all(quote!{
            impl<#gen_ptrs> #target AnonWidget<#gen_tys> {
                #contents
            }
        });
    };
    
    let class = args.class.unwrap_or_else(|| parse_quote!{ #c::Class::Container });
    
    // TODO: we should probably not rely on recursive macro expansion here!
    // (I.e. use direct code generation for Widget derivation, instead of derive.)
    let toks = (quote!{ {
        #[widget(class = #class, layout = #layout)]
        #[handler(response = #response, generics = < #handler_extra > #handler_where)]
        #[derive(Clone, Debug, #c::macros::Widget)]
        struct AnonWidget<#gen_ptrs> {
            #field_toks
        }
        
        #impls

        AnonWidget {
            #field_val_toks
        }
    } }).into();
    
    toks
}

/// Macro to derive `From<NoResponse>`
/// 
/// This macro assumes the type is an enum with a simple variant named `None`.
// TODO: add diagnostics to check against mis-use?
#[proc_macro_derive(NoResponse)]
pub fn derive_no_response(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let c = c();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let toks = quote!{
        impl #impl_generics From<#c::event::NoResponse>
            for #name #ty_generics #where_clause
        {
            fn from(_: #c::event::NoResponse) -> Self {
                #name::None
            }
        }
    };
    toks.into()
}

// Our stand-in for $crate. Imperfect, but works (excepting other crates in
// the same package, i.e. doc-tests, examples, integration tests, benches).
fn c() -> TokenStream {
    if env::var("CARGO_PKG_NAME") == Ok("kas".to_string()) {
        parse_quote!( crate )
    } else {
        parse_quote!( kas )
    }
}
