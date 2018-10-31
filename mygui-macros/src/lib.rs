#![recursion_limit="128"]

extern crate proc_macro;

use std::env;
use std::iter::once;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Ident, Index, Member};
use syn::{parse_quote, parse_macro_input, parenthesized};
use syn::parse::{Parse, ParseStream, Result};
use syn::token::{Comma, Eq};

/// Macro to derive widget traits
/// 
/// Unlike normal derive macros, this one implements multiple traits. Each of
/// these is optional.
/// 
/// If a struct field is marked with `#[core]`, then the [`Core`] trait
/// will be derived. The marked field must implement [`Core`] itself (it is
/// recommended to use the [`CoreData`] type).
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
/// [`Widget`]: ../mygui/widget/trait.Widget.html
#[proc_macro_derive(Widget, attributes(core, widget))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    
    // Our stand-in for $crate. Imperfect, but works (excepting other crates in
    // the same package, i.e. doc-tests, examples, integration tests, benches).
    let c = if env::var("CARGO_PKG_NAME").unwrap() == "mygui" {
        quote!( crate )
    } else {
        quote!( mygui )
    };
    
    let widget = match read_impl_attrs(&mut ast) {
        Ok(w) => w,
        Err(err) => return err.to_compile_error().into(),
    };
    let (core, children) = read_field_attrs(&ast);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    
    let mut toks = TokenStream::default();
    
    if let Some(core) = core {
        toks.extend(once(quote! {
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
        }));
    }
    
    if let Some(widget) = widget {
        let class = widget.class;
        let label = widget.label.unwrap_or_else(|| parse_quote!{ None });
        let count = children.len();
        let child1 = children.iter();
        let child2 = children.iter();
        
        // TODO: iteration could generate nicer code if done without quote
        toks.extend(once(quote! {
            impl #impl_generics #c::widget::Widget
                    for #name #ty_generics #where_clause
            {
                fn class(&self) -> #c::widget::Class { #class }
                fn label(&self) -> Option<&str> { #label }

                fn len(&self) -> usize {
                    #count
                }
                fn get(&self, _index: usize) -> Option<&#c::widget::Widget> {
                    let _i = 0;
                    #(
                        if _index == _i {
                            return Some(&self.#child1);
                        }
                        let _i = _i + 1;
                    )*
                    return None;
                }
                fn get_mut(&mut self, _index: usize) -> Option<&mut #c::widget::Widget> {
                    let _i = 0;
                    #(
                        if _index == _i {
                            return Some(&mut self.#child2);
                        }
                        let _i = _i + 1;
                    )*
                    return None;
                }
            }
        }));
    }
    
    toks.into()
}


fn read_impl_attrs(ast: &mut DeriveInput) -> Result<Option<WidgetArgs>> {
    let mut widget = None;
    for attr in ast.attrs.drain(..) {
        if attr.path == parse_quote!{ widget } {
            if widget.is_none() {
                let tts = attr.tts;
                widget = Some(syn::parse2(tts)?);
            } else {
                panic!("Error: multiple #[widget(..)] attributes on type");
            }
        }
    }
    Ok(widget)
}

fn read_field_attrs(ast: &DeriveInput) -> (Option<Member>, Vec<Member>) {
    match &ast.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(FieldsNamed { named: fields, .. }) |
                Fields::Unnamed(FieldsUnnamed { unnamed: fields, ..}) => {
                    let mut core = None;
                    let mut children = vec![];
                    
                    for (i, field) in fields.iter().enumerate() {
                        for attr in field.attrs.iter() {
                            if attr.path == parse_quote!{ core } {
                                if core.is_none() {
                                    core = Some(member(i, field.ident.clone()));
                                } else {
                                    panic!("Multiple fields marked as #[core]");
                                }
                            } else if attr.path == parse_quote!{ widget } {
                                children.push(member(i, field.ident.clone()));
                            }
                        }
                    }
                    
                    (core, children)
                },
                Fields::Unit => {
                    panic!("Error: cannot derive Widget on a unit struct")
                }
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            panic!("Error: cannot derive Widget on an enum or a union")
        }
    }
}

fn member(index: usize, ident: Option<Ident>) -> Member {
    match ident {
        None => Member::Unnamed(Index{
                index: index as u32,
                span: Span::call_site()
            }),
        Some(ident) => Member::Named(ident),
    }
}

mod kw {
    use syn::custom_keyword;
    
    custom_keyword!(class);
    custom_keyword!(label);
}

struct WidgetArgs {
    class: Expr,
    label: Option<Expr>,
}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = parenthesized!(content in input);
        
        let mut class = None;
        let mut label = None;
        
        loop {
            if content.is_empty() {
                break;
            }
            
            let lookahead = content.lookahead1();
            if class.is_none() && lookahead.peek(kw::class) {
                let _: kw::class = content.parse()?;
                let _: Eq = content.parse()?;
                let expr: Expr = content.parse()?;
                class = Some(expr);
            } else if label.is_none() && lookahead.peek(kw::label) {
                let _: kw::label = content.parse()?;
                let _: Eq = content.parse()?;
                let expr: Expr = content.parse()?;
                label = Some(expr);
            } else {
                return Err(lookahead.error());
            }
            
            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }
        
        Ok(WidgetArgs {
            class: class.ok_or_else(|| content.error("expected `class = ...`"))?,
            label,
        })
    }
}
