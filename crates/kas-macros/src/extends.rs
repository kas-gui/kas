// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Extends macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::{Expr, ImplItem, ImplItemFn, ItemImpl, parse_quote};

#[allow(non_camel_case_types)]
mod kw {
    syn::custom_keyword!(ThemeDraw);
    syn::custom_keyword!(using);
}

pub enum Extends {
    ThemeDraw { base: Expr },
}

impl Parse for Extends {
    fn parse(input: ParseStream) -> Result<Extends> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::ThemeDraw) {
            let _ = input.parse::<kw::ThemeDraw>()?;
            let _ = input.parse::<kw::using>()?;
            let base = input.parse()?;
            Ok(Extends::ThemeDraw { base })
        } else {
            Err(lookahead.error())
        }
    }
}

struct Methods(Vec<ImplItemFn>);
impl Parse for Methods {
    fn parse(input: ParseStream) -> Result<Methods> {
        let mut vec = Vec::new();

        while !input.is_empty() {
            vec.push(input.parse()?);
        }

        Ok(Methods(vec))
    }
}

impl Methods {
    fn theme_draw(base: &Expr) -> Self {
        parse_quote! {
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

            fn event_state_overlay(&mut self) {
                (#base).event_state_overlay();
            }

            fn frame(&mut self, id: &Id, rect: Rect, style: ::kas::theme::FrameStyle, bg: Background) {
                (#base).frame(id, rect, style, bg);
            }

            fn separator(&mut self, rect: Rect) {
                (#base).separator(rect);
            }

            fn selection(&mut self, rect: Rect, style: ::kas::theme::SelectionStyle) {
                (#base).selection(rect, style);
            }

            fn text_effects(
                &mut self,
                id: &Id,
                pos: Coord,
                rect: Rect,
                text: &TextDisplay,
                colors: &[Rgba],
                effects: &[::kas::text::Effect],
            ) {
                (#base).text_effects(id, pos, rect, text, colors, effects);
            }

            fn text_selected_range(
                &mut self,
                id: &Id,
                pos: Coord,
                rect: Rect,
                text: &TextDisplay,
                range: Range<usize>,
            ) {
                (#base).text_selected_range(id, pos, rect, text, range);
            }

            fn text_cursor(
                &mut self,
                id: &Id,
                pos: Coord,
                rect: Rect,
                text: &TextDisplay,
                byte: usize,
            ) {
                (#base).text_cursor(id, pos, rect, text, byte);
            }

            fn check_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
                (#base).check_box(id, rect, checked, last_change);
            }

            fn radio_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
                (#base).radio_box(id, rect, checked, last_change);
            }

            fn mark(&mut self, id: &Id, rect: Rect, style: MarkStyle) {
                (#base).mark(id, rect, style);
            }

            fn scroll_bar(
                &mut self,
                id: &Id,
                id2: &Id,
                rect: Rect,
                h_rect: Rect,
                dir: Direction,
            ) {
                (#base).scroll_bar(id, id2, rect, h_rect, dir);
            }

            fn slider(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, dir: Direction) {
                (#base).slider(id, id2, rect, h_rect, dir);
            }

            fn progress_bar(&mut self, id: &Id, rect: Rect, dir: Direction, value: f32) {
                (#base).progress_bar(id, rect, dir, value);
            }

            fn image(&mut self, id: ImageId, rect: Rect) {
                (#base).image(id, rect);
            }
        }
    }
}

impl Extends {
    pub fn extend(self, item: TokenStream) -> Result<TokenStream> {
        let mut impl_: ItemImpl = syn::parse2(item)?;

        let mut methods = match &self {
            Extends::ThemeDraw { base } => Methods::theme_draw(base).0,
        };
        methods.retain(|item| {
            let name = item.sig.ident.to_string();
            impl_.items.iter().all(
                |item| !matches!(item, ImplItem::Fn(ImplItemFn { sig, .. }) if sig.ident == name),
            )
        });

        impl_.items.extend(methods.into_iter().map(ImplItem::Fn));

        Ok(quote! { #impl_ })
    }
}
