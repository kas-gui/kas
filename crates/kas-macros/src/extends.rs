// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Extends macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_quote, Expr, ImplItem, ImplItemMethod, ItemImpl, Token};

#[allow(non_camel_case_types)]
mod kw {
    syn::custom_keyword!(ThemeDraw);
    syn::custom_keyword!(base);
}

pub struct Extends {
    base: Expr,
}

impl Parse for Extends {
    fn parse(content: ParseStream) -> Result<Extends> {
        let _ = content.parse::<kw::ThemeDraw>()?;
        let _ = content.parse::<Token![,]>()?;
        let _ = content.parse::<kw::base>()?;
        let _ = content.parse::<Token![=]>()?;

        Ok(Extends {
            base: content.parse()?,
        })
    }
}

struct Methods(Vec<ImplItemMethod>);
impl Parse for Methods {
    fn parse(input: ParseStream) -> Result<Methods> {
        let mut vec = Vec::new();

        while !input.is_empty() {
            vec.push(input.parse()?);
        }

        Ok(Methods(vec))
    }
}

impl Extends {
    fn methods_theme_draw(self) -> Vec<ImplItemMethod> {
        let base = self.base;
        let methods: Methods = parse_quote! {
            fn new_pass<'_gen_a>(
                &mut self,
                rect: ::kas::geom::Rect,
                offset: ::kas::geom::Offset,
                class: ::kas::draw::PassType,
                f: Box<dyn FnOnce(&mut dyn ::kas::theme::ThemeDraw) + '_gen_a>,
            ) {
                (#base).new_pass(rect, offset, class, f);
            }

            fn get_clip_rect(&mut self) -> Rect {
                (#base).get_clip_rect()
            }

            fn frame(&mut self, id: &WidgetId, rect: Rect, style: FrameStyle, bg: Background) {
                (#base).frame(id, rect, style, bg);
            }

            fn separator(&mut self, rect: Rect) {
                (#base).separator(rect);
            }

            fn selection_box(&mut self, rect: Rect) {
                (#base).selection_box(rect);
            }

            fn text(&mut self, id: &WidgetId, pos: Coord, text: &TextDisplay, class: TextClass) {
                (#base).text(id, pos, text, class);
            }

            fn text_effects(&mut self, id: &WidgetId, pos: Coord, text: &dyn TextApi, class: TextClass) {
                (#base).text_effects(id, pos, text, class);
            }

            fn text_selected_range(
                &mut self,
                id: &WidgetId,
                pos: Coord,
                text: &TextDisplay,
                range: Range<usize>,
                class: TextClass,
            ) {
                (#base).text_selected_range(id, pos, text, range, class);
            }

            fn text_cursor(
                &mut self,
                id: &WidgetId,
                pos: Coord,
                text: &TextDisplay,
                class: TextClass,
                byte: usize,
            ) {
                (#base).text_cursor(id, pos, text, class, byte);
            }

            fn checkbox(&mut self, id: &WidgetId, rect: Rect, checked: bool, last_change: Option<Instant>) {
                (#base).checkbox(id, rect, checked, last_change);
            }

            fn radiobox(&mut self, id: &WidgetId, rect: Rect, checked: bool, last_change: Option<Instant>) {
                (#base).radiobox(id, rect, checked, last_change);
            }

            fn mark(&mut self, id: &WidgetId, rect: Rect, style: MarkStyle) {
                (#base).mark(id, rect, style);
            }

            fn scrollbar(
                &mut self,
                id: &WidgetId,
                id2: &WidgetId,
                rect: Rect,
                h_rect: Rect,
                dir: Direction,
            ) {
                (#base).scrollbar(id, id2, rect, h_rect, dir);
            }

            fn slider(&mut self, id: &WidgetId, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction) {
                (#base).slider(id, id2, rect, h_rect, dir);
            }

            fn progress_bar(&mut self, id: &WidgetId, rect: Rect, dir: Direction, value: f32) {
                (#base).progress_bar(id, rect, dir, value);
            }

            fn image(&mut self, id: ImageId, rect: Rect) {
                (#base).image(id, rect);
            }
        };
        methods.0
    }

    pub fn extend(self, item: TokenStream) -> Result<TokenStream> {
        let mut impl_: ItemImpl = syn::parse2(item)?;

        let mut methods = self.methods_theme_draw();
        methods.retain(|method| {
            let name = method.sig.ident.to_string();
            impl_.items.iter().all(|item| !matches!(item, ImplItem::Method(ImplItemMethod { sig, .. }) if sig.ident == name))
        });

        impl_
            .items
            .extend(methods.into_iter().map(|m| ImplItem::Method(m)));

        Ok(quote! { #impl_ })
    }
}
