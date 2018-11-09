#![recursion_limit="128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

mod args;

use std::env;
use std::fmt::Write;
use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, TokenStreamExt};
use syn::{DeriveInput, Ident, Type, TypePath};
use syn::{parse_quote, parse_macro_input};

use self::args::ChildType;

#[cfg(not(feature = "cassowary"))] mod layout_extern;
#[cfg(not(feature = "cassowary"))] use self::layout_extern as layout;

#[cfg(feature = "cassowary")] mod layout_cw;
#[cfg(feature = "cassowary")] use self::layout_cw as layout;

/// Macro to derive widget traits
/// 
/// Unlike normal derive macros, this one implements multiple traits. [`Core`]
/// is always derived; other traits are optional.
/// 
/// One struct field must be marked with `#[core]` and implement the [`Core`]
/// trait. It is recommended to use the [`CoreData`] type.
/// 
/// If there is a `#[layout(...)]` attribute on the struct, then the [`Layout`]
/// trait will be implemented. This attribute expects one of the following
/// arguments:
/// 
/// -   `single` — single child only
/// -   `horizontal` — widgets are laid out in a row from left to right in the
///     order specified
/// -   `vertical` — same except top-to-bottom
/// -   `grid` — see per-field `#[widget]` attribute specification
/// 
/// If there is a `#[widget(...)]` attribute on the struct (in addition to the
/// `#[derive(Widget)]` attribute), then the [`Widget`] trait will be
/// implemented. All child widgets must be a field marked with `#[widget]`.
/// The `#[widget(...)]` attribute on the struct itself supports the following
/// parameters:
/// 
/// -   `class = ...` (required) — an expression yielding the widget's [`Class`]
/// -   `label = ...`(optional) — an expression yielding the widget's [`label`]
/// 
/// When deriving `Layout` or `Widget`, a `#[widget]` attribute should *also*
/// be used on each field which is a child widget. This attribute accepts the
/// following arguments (for use when using the `grid` layout).
/// 
/// -   `col = ...` — first column, from left (defaults to 0)
/// -   `row = ...` — first row, from top (defaults to 0)
/// -   `cspan = ...` — number of columns to span (defaults to 1)
/// -   `rspan = ...` — number of rows to span (defaults to 1)
/// 
/// Example:
/// 
/// ```notest
/// #[widget(class = Class::Window)]
/// #[derive(Widget)]
/// pub struct SimpleWindow<W: Widget> {
///     #[core] core: CoreData,
///     min_size: Coord,
///     #[widget] w: W
/// }
/// ```
/// 
/// Note: usage of this macro currently requires `#![feature(unrestricted_attribute_tokens)]`.
/// 
/// [`label`]: ../mygui/widget/trait.Widget.html#tymethod.label
/// [`Class`]: ../mygui/widget/enum.Class.html
/// [`Core`]: ../mygui/widget/trait.Core.html
/// [`CoreData`]: ../mygui/widget/struct.CoreData.html
/// [`Layout`]: ../mygui/widget/trait.Layout.html
/// [`Widget`]: ../mygui/widget/trait.Widget.html
#[proc_macro_derive(Widget, attributes(core, layout, widget))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let c = c();
    
    let args = match args::read_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let mut toks = TokenStream::default();
    
    if true {
        let core = args.core;
        toks.append_all(quote! {
            impl #impl_generics #c::widget::Core
                for #name #ty_generics #where_clause
            {
                fn number(&self) -> u32 {
                    self.#core.number()
                }
                fn set_number(&mut self, number: u32) {
                    self.#core.set_number(number);
                }
                
                fn tkd(&self) -> #c::toolkit::TkData {
                    self.#core.tkd()
                }
                fn set_tkd(&mut self, tkd: #c::toolkit::TkData) {
                    self.#core.set_tkd(tkd)
                }
                
                fn rect(&self) -> &#c::widget::Rect {
                    self.#core.rect()
                }
                fn rect_mut(&mut self) -> &mut #c::widget::Rect {
                    self.#core.rect_mut()
                }
            }
        });
    }
    
    if let Some(layout) = args.layout {
        let fns = match layout::fns(&c, &args.children, layout) {
            Ok(fns) => fns,
            Err(err) => return err.to_compile_error().into(),
        };
        
        toks.append_all(quote! {
            impl #impl_generics #c::widget::Layout
                    for #name #ty_generics #where_clause
            {
                #fns
            }
        });
    }
    
    if let Some(widget) = args.widget {
        let class = widget.class;
        let label = widget.label.unwrap_or_else(|| parse_quote!{ None });
        let count = args.children.len();
        
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
        
        toks.append_all(quote! {
            impl #impl_generics #c::widget::Widget
                    for #name #ty_generics #where_clause
            {
                fn class(&self) -> #c::widget::Class { #class }
                fn label(&self) -> Option<&str> { #label }

                fn len(&self) -> usize {
                    #count
                }
                fn get(&self, _index: usize) -> Option<&#c::widget::Widget> {
                    match _index {
                        #get_rules
                        _ => None
                    }
                }
                fn get_mut(&mut self, _index: usize) -> Option<&mut #c::widget::Widget> {
                    match _index {
                        #get_mut_rules
                        _ => None
                    }
                }
            }
        });
    }
    
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
/// <input>     ::= <layout> "=>" <response> ";" <fields>
/// <layout>    ::= "single" | "horizontal" | "vertical" | "grid"
/// <response>  ::= <type>
/// <fields>    ::= "" | <field> | <field> "," <fields>
/// <field>     ::= <w_attr> <opt_ident> <field_ty> = <expr> <handler>
/// <opt_ident> ::= "_" | <ident>
/// <field_ty>  ::= "" | ":" <type> | "->" <type>
/// <handler>   ::= "" | "=>" <block>
/// <w_attr>    ::= "" | "#" "[" <widget> <w_params> "]"
/// <w_params>  ::= "" | "(" <w_args> ")"
/// <w_args>    ::= <w_arg> | <w_arg> "," <w_args>
/// <w_arg>     ::= <pos_arg> "=" <lit>
/// <pos_arg>   ::= "col" | "row" | "cspan" | "rspan"
/// ```
/// where `<type>` is a type expression, `<expr>` is a (value) expression,
/// `<ident>` is an identifier, `<block>` is a block (`{ ... }`) and `<lit>` is
/// a literal. `""` is the empty string (i.e. nothing).
/// 
/// The effect of this macro is to create an anonymous struct with the above
/// fields (plus an implicit `core`), implement [`Core`], [`Layout`], [`Widget`]
/// and [`Handler`] (with the specified `<response>` type), then construct an
/// instance using the given value expressions, and return it.
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
/// `#[widget] ident = value => handler` where `handler` is a block of code
/// which receives a response from the child widget (via `msg`) and returns a
/// response from this widget. Two additional identifiers are available, `self`
/// (allowing e.g. `self.some_field`) and `tk` (the toolkit).
/// 
/// Currently usage of this macro requires `#![feature(proc_macro_hygiene)]`.
/// 
/// [`Core`]: ../mygui/widget/trait.Core.html
/// [`Layout`]: ../mygui/widget/trait.Layout.html
/// [`Widget`]: ../mygui/widget/trait.Widget.html
/// [`Handler`]: ../mygui/event/trait.Handler.html
#[proc_macro]
pub fn make_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as args::MakeWidget);
    
    // Used to make fresh identifiers for generic types
    let mut name_buf = String::with_capacity(32);
    
    let c = c();
    let comma = TokenTree::from(Punct::new(',', Spacing::Alone));
    
    // fields of anonymous struct:
    let mut field_toks = quote!{ #[core] core: CoreData, };
    // initialisers for these fields:
    let mut field_val_toks = quote!{ core: Default::default(), };
    
    // generic types on struct, without constraints:
    let mut gen_tys = TokenStream::new();
    // generic types on struct, with constraints:
    let mut gen_ptrs = TokenStream::new();
    // generic types on handler impl, with constraints:
    let mut gen_response_ptrs = TokenStream::new();
    // where clause on handler impl:
    let mut handler_where = quote!{ where };
    // Has the above been appended to? (Difficult to test directly.)
    let mut have_where = false;
    // per-child-widget result handlering:
    let mut handler_toks = TokenStream::new();
    
    let layout = &args.layout;
    let response = &args.response;
    
    for (index, field) in args.fields.iter().enumerate() {
        let attr = &field.widget_attr;
        
        let ident = match &field.ident {
            Some(ref ident) => ident.clone(),
            None => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("mw_anon_{}", index)).unwrap();
                Ident::new(&name_buf, Span::call_site())
            }
        };
        
        let ty: Type = match &field.ty {
            ChildType::Type(ty) => ty.clone(),
            cty @ ChildType::Generic |
            cty @ ChildType::Response(_) => {
                name_buf.clear();
                name_buf.write_fmt(format_args!("MWAnon{}", index)).unwrap();
                let ty = Ident::new(&name_buf, Span::call_site());
                
                if !gen_tys.is_empty() {
                    gen_tys.append(comma.clone());
                    gen_ptrs.append(comma.clone());
                    gen_response_ptrs.append(comma.clone());
                }
                
                gen_tys.append_all(quote!{ #ty });
                if attr.is_some() {
                    gen_ptrs.append_all(quote!{ #ty: Widget + 'static });
                    
                    match cty {
                        ChildType::Generic => {
                            name_buf.push_str("R");
                            let tyr = Ident::new(&name_buf, Span::call_site());
                            gen_response_ptrs.append_all(quote!{
                                #tyr, #ty: Widget + Handler<Response = #tyr>
                            });
                            if have_where {
                                handler_where.append(comma.clone());
                            }
                            handler_where.append_all(quote!{
                                #tyr: From<NoResponse>, #response: From<#tyr>
                            });
                            have_where = true;
                        }
                        ChildType::Response(tyr) => {
                            gen_response_ptrs.append_all(quote!{
                                #ty: Widget + Handler<Response = #tyr>
                            });
                        }
                        _ => unreachable!()
                    }
                } else {
                    gen_ptrs.append_all(quote!{ #ty });
                    gen_response_ptrs.append_all(quote!{ #ty });
                }
                
                Type::Path(TypePath { qself: None, path: ty.into() })
            }
        };
        
        let value = &field.value;
        
        field_toks.append_all(quote!{ #attr #ident: #ty, });
        field_val_toks.append_all(quote!{ #ident: #value, });
        
        if attr.is_some() {
            let handler = if let Some(ref h) = field.handler {
                quote!{ #h }
            } else {
                quote!{ msg.into() }
            };
            handler_toks.append_all(quote!{
                if num <= self.#ident.number() {
                    let msg = self.#ident.handle_action(tk, action, num);
                    return #handler;
                }
            });
        }
    }
    
    if !have_where {
        handler_where = TokenStream::new();
    }
    
    let toks = (quote!{ {
        use #c::event::{Action, Handler, ignore};
        use #c::macros::Widget;
        use #c::toolkit::Toolkit;
        use #c::widget::{Class, Core, CoreData, Widget};

        #[layout(#layout)]
        #[widget(class = Class::Container)]
        #[derive(Clone, Debug, Widget)]
        struct L<#gen_ptrs> {
            #field_toks
        }

        impl<#gen_response_ptrs> Handler for L<#gen_tys> #handler_where {
            type Response = #response;
            
            fn handle_action(&mut self, tk: &Toolkit, action: Action, num: u32) -> #response {
                #handler_toks
                
                if num != self.number() {
                    println!("Warning: incorrect widget number");
                }
                ignore(action)  // no actions handled by this widget
            }
        }

        L {
            #field_val_toks
        }
    } }).into();
    toks
}

// Our stand-in for $crate. Imperfect, but works (excepting other crates in
// the same package, i.e. doc-tests, examples, integration tests, benches).
fn c() -> TokenStream {
    if env::var("CARGO_PKG_NAME") == Ok("mygui".to_string()) {
        parse_quote!( crate )
    } else {
        parse_quote!( mygui )
    }
}
