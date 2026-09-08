// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`MultiPartEditor`] widget

use super::editor::{Common, EventAction, Part};
use crate::edit::editor::{ActionResetStatus, PartList, TextIndex};
use crate::edit::highlight::{Highlighter, Plain};
use crate::{ScrollBar, ScrollBarMsg};
use kas::cast::Ceil;
use kas::event::components::ScrollComponent;
use kas::event::{CursorIcon, Scroll};
use kas::prelude::*;
use kas::text::{Direction, Status};
use kas::theme::{FrameStyle, TextClass};

#[derive(Debug)]
struct CallOnEdit;

#[impl_self]
mod MultiPartEditor {
    /// A multi-part text-editor widget
    #[autoimpl(Debug ignore self.on_edit where H: trait)]
    #[widget]
    pub struct MultiPartEditor<H: Highlighter = Plain> {
        core: widget_core!(),
        scroll: ScrollComponent,
        // NOTE: inner is a Viewport which doesn't use update methods, therefore we don't call them.
        #[widget(&())]
        inner: Inner<H>,
        #[widget(&())]
        vert_bar: ScrollBar<kas::dir::Down>,
        frame_offset: Offset,
        frame_size: Size,
        frame_offset_ex_margin: Offset,
        inner_margin: i32,
        clip_rect: Rect,
        on_edit: Option<Box<dyn Fn(&mut EventCx, &Self) + Send>>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, mut axis: AxisInfo) -> SizeRules {
            let size = self.frame_size.extract(axis.flipped());
            axis.map_other(|x| x - size);

            let mut rules = self.inner.size_rules(cx, axis);
            let bar_rules = self.vert_bar.size_rules(cx, axis);
            if axis.is_horizontal() {
                self.inner_margin = rules.margins_i32().1.max(bar_rules.margins_i32().0);
                rules.append(bar_rules);
            }

            let frame_rules = cx.frame(FrameStyle::EditBox, axis);
            self.frame_offset_ex_margin
                .set_component(axis, frame_rules.size());
            let (rules, offset, size) = frame_rules.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut SizeCx, outer_rect: Rect, hints: AlignHints) {
            self.core.set_rect(outer_rect);
            let mut rect = outer_rect;

            self.clip_rect = Rect {
                pos: rect.pos + self.frame_offset_ex_margin,
                size: rect.size - (self.frame_offset_ex_margin * 2).cast(),
            };

            rect.pos += self.frame_offset;
            rect.size -= self.frame_size;

            // Set bar position, dependent on text direction. TODO: move on text-dir-change.
            let bar_width = cx.scroll_bar_width();
            let (x0, x1);
            if !self
                .inner
                .parts
                .first()
                .map(|part| part.text_is_rtl())
                .unwrap_or_default()
            {
                x1 = rect.pos.0 + rect.size.0;
                x0 = x1 - bar_width;
            } else {
                x0 = rect.pos.0;
                x1 = x0 + bar_width;
                rect.pos.0 = x1;
            }
            let bar_rect = Rect::new(Coord(x0, rect.pos.1), Size(bar_width, rect.size.1));
            rect.size.0 = (rect.size.0 - bar_width - self.inner_margin).max(0);
            self.vert_bar.set_rect(cx, bar_rect, AlignHints::NONE);

            self.inner.set_rect(cx, rect, hints);
            self.update_content_size(cx);
        }

        fn draw(&self, mut draw: DrawCx) {
            let bg = self.inner.common.background_color();
            draw.frame(self.rect(), FrameStyle::EditBox, bg);

            self.inner
                .draw_with_offset(draw.re(), self.clip_rect, self.scroll.offset());

            if self.scroll.max_offset().1 > 0 {
                self.vert_bar.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll.offset(),
                max_offset: self.scroll.max_offset(),
            }
        }

        fn translation(&self, index: usize) -> Offset {
            if index == widget_index!(self.inner) {
                self.scroll.offset()
            } else {
                Offset::ZERO
            }
        }
    }

    impl Events for Self {
        type Data = ();

        fn probe(&self, coord: Coord) -> Id {
            if self.scroll.max_offset().1 > 0 {
                if let Some(id) = self.vert_bar.try_probe(coord) {
                    return id;
                }
            }

            // If coord is over self but not over self.vert_bar, we assign
            // the event to self.inner without further question.
            self.inner.id()
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            let rect = Rect {
                pos: self.rect().pos + self.frame_offset,
                size: self.rect().size - self.frame_size,
            };
            let used = self.scroll.scroll_by_event(cx, event, self.id(), rect);
            self.update_scroll_offset(cx);
            used
        }

        fn handle_messages(&mut self, cx: &mut EventCx<'_>, _: &()) {
            if let Some(CallOnEdit) = cx.try_pop() {
                if let Some(ref method) = self.on_edit {
                    method(cx, self);
                }
            }

            let offset = if cx.last_child() == Some(widget_index![self.vert_bar])
                && let Some(ScrollBarMsg(y)) = cx.try_pop()
            {
                Offset(self.scroll.offset().0, y)
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                offset
            } else {
                return;
            };

            if let Some(moved) = self.scroll.set_offset(offset) {
                cx.action_moved(moved);
                self.update_scroll_offset(cx);
            }
        }

        fn handle_resize(&mut self, cx: &mut ConfigCx) -> bool {
            // Assumption: content does not require re-evaluation of size_rules() or set_rect()
            self.update_content_size(cx);
            true
        }

        fn handle_scroll(&mut self, cx: &mut EventCx<'_>, _: &(), scroll: Scroll) {
            let rect = self.inner.rect();
            self.scroll.scroll(cx, self.id(), rect, scroll);
            self.update_scroll_offset(cx);
        }
    }

    impl Default for MultiPartEditor<Plain> {
        #[inline]
        fn default() -> Self {
            MultiPartEditor::new("")
        }
    }

    impl MultiPartEditor<Plain> {
        /// Construct a `MultiPartEditor`
        #[inline]
        pub fn new(text: &str) -> Self {
            MultiPartEditor {
                core: Default::default(),
                scroll: Default::default(),
                inner: Inner::default().with_text(text),
                vert_bar: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                frame_offset_ex_margin: Default::default(),
                inner_margin: Default::default(),
                clip_rect: Default::default(),
                on_edit: None,
            }
        }
    }

    impl Self {
        /// Replace the highlighter
        ///
        /// This function reconstructs the text with a new highlighter.
        ///
        /// Note: this method discards the edit handler; see [`Self::on_edit`].
        #[inline]
        pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> MultiPartEditor<H2> {
            debug_assert!(self.on_edit.is_none());
            MultiPartEditor {
                core: self.core,
                scroll: self.scroll,
                inner: self.inner.with_highlighter(highlighter),
                vert_bar: self.vert_bar,
                frame_offset: self.frame_offset,
                frame_size: self.frame_size,
                frame_offset_ex_margin: self.frame_offset_ex_margin,
                inner_margin: self.inner_margin,
                clip_rect: self.clip_rect,
                on_edit: None,
            }
        }

        /// Set a new highlighter of the same type
        pub fn set_highlighter(&mut self, highlighter: H) {
            self.inner.highlighter = highlighter;
        }

        /// Call the handler `f` on edit
        ///
        /// Note: this method should not be called before [`Self::with_highlighter`].
        #[inline]
        #[must_use]
        pub fn on_edit(mut self, f: impl Fn(&mut EventCx, &Self) + Send + 'static) -> Self {
            debug_assert!(self.on_edit.is_none());
            self.on_edit = Some(Box::new(f));
            self
        }

        /// Read text contents from parts
        ///
        /// The whole contents equals the concatenation of parts. FIXME
        pub fn text_parts(&self) -> impl Iterator<Item = &Part> {
            self.inner.parts.iter()
        }

        /// Copy text contents to a `String`
        pub fn text_to_string(&self) -> String {
            let mut s = String::new();
            let mut iter = self.text_parts();
            if let Some(first) = iter.next() {
                s.push_str(first.as_str());
            }
            for part in iter {
                s.push('\n'); // TODO
                s.push_str(part.as_str());
            }
            s
        }

        fn update_content_size(&mut self, cx: &mut EventState) {
            if !self.core.status.is_sized() {
                return;
            }
            let size = self.inner.rect().size;
            let _ = self.scroll.set_sizes(size, self.inner.content_size());
            let max_offset = self.scroll.max_offset().1;
            self.vert_bar.set_limits(cx, max_offset, size.1);
            self.update_scroll_offset(cx);
        }

        fn update_scroll_offset(&mut self, cx: &mut EventState) {
            self.vert_bar.set_value(cx, self.scroll.offset().1);
        }

        /// Set the base text direction (inline)
        ///
        /// If [`Direction::Auto`] or [`Direction::AutoRtl`] is used, the direction
        /// will be updated on edit to persist the last used text direction to
        /// non-directional content.
        #[inline]
        pub fn with_direction(mut self, direction: Direction) -> Self {
            let _ = self.inner.common.set_direction(direction);
            self
        }

        /// Set the initial text (inline)
        ///
        /// This method should only be used on a new `MultiPartEditor`.
        #[inline]
        #[must_use]
        pub fn with_text(mut self, text: &str) -> Self {
            self.inner = self.inner.with_text(text);
            self
        }

        /// Adjust the width allocation
        #[inline]
        pub fn set_width_em(&mut self, min_em: f32, ideal_em: f32) {
            self.inner.width = (min_em, ideal_em);
        }

        /// Adjust the width allocation (inline)
        #[inline]
        #[must_use]
        pub fn with_width_em(mut self, min_em: f32, ideal_em: f32) -> Self {
            self.set_width_em(min_em, ideal_em);
            self
        }
    }
}

#[impl_self]
mod Inner {
    /// Inner ([`Viewport`]) widget of [`MultiPartEditor`]
    #[derive(Debug)]
    #[widget]
    pub struct Inner<H: Highlighter> {
        core: widget_core!(),
        width: (f32, f32),
        content_size: Size,
        common: Common,
        highlighter: H,
        parts: Vec<Part>,
    }

    impl Default for Self
    where
        H: Default,
    {
        fn default() -> Self {
            Inner {
                core: Default::default(),
                width: (8.0, 16.0),
                content_size: Size::ZERO,
                common: Common::new(true),
                highlighter: H::default(),
                parts: vec![Part::default()],
            }
        }
    }

    impl Self {
        /// Set the initial text (inline)
        ///
        /// This method should only be used on a new `Inner`.
        #[must_use]
        fn with_text(mut self, text: &str) -> Self {
            debug_assert!(self.common.is_unedited());

            self.common.set_cursor(&self.parts, TextIndex::new(0, 0));

            self.parts = kas::text::Lines::new(text)
                .map(|(line, _)| Part::from(line))
                .collect();
            debug_assert!(!self.parts.is_empty());

            self
        }

        /// Replace the highlighter
        ///
        /// This function reconstructs the text with a new highlighter.
        #[inline]
        pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> Inner<H2> {
            Inner {
                core: self.core,
                width: self.width,
                content_size: self.content_size,
                common: self.common,
                highlighter,
                parts: self.parts,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let (min, mut ideal): (i32, i32);
            if axis.is_horizontal() {
                let dpem = cx.dpem(TextClass::Editor);
                min = (self.width.0 * dpem).cast_to(Ceil);
                ideal = (self.width.1 * dpem).cast_to(Ceil);
            } else if let Some(width) = axis.other()
                && let Some(first) = self.parts.first()
                && let Ok(h) = first.measure_height(width.cast(), None)
            {
                min = h.cast_to(Ceil);
                ideal = min;
            } else {
                debug_assert!(false);
                return SizeRules::EMPTY;
            };

            let rules = if axis.is_horizontal() {
                self.parts
                    .iter_mut()
                    .map(|part| part.size_rules(&self.common, cx, axis))
                    .fold(SizeRules::default(), |rules, item| rules.max(item))
            } else {
                self.parts
                    .iter_mut()
                    .map(|part| part.size_rules(&self.common, cx, axis))
                    .reduce(|a, b| a.appended(b))
                    .unwrap_or_default()
            };
            ideal = ideal.max(rules.ideal_size());

            SizeRules::new(min, ideal, Stretch::High).with_margins(cx.text_margins().extract(axis))
        }

        #[inline]
        fn set_rect(&mut self, cx: &mut SizeCx, mut rect: Rect, _: AlignHints) {
            self.core.set_rect(rect);

            let mut content_size = Size::ZERO;
            for part in &mut self.parts {
                let part_size = part.content_size();
                rect.size.1 = part_size.1;
                part.set_rect(&self.common, cx, rect);

                content_size.0 = content_size.0.max(part_size.0);
                content_size.1 += part_size.1;
                rect.pos.1 += part_size.1;
            }
            self.content_size = content_size;
        }
    }

    impl Viewport for Self {
        #[inline]
        fn content_size(&self) -> Size {
            self.content_size
        }

        #[inline]
        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            for (i, part) in self.parts.iter().enumerate() {
                part.draw_with_offset(draw.re(), &self.common, i.cast(), rect, offset);
            }
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            // TODO: update role and make compliant with AccessKit
            Role::Unknown
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = ();

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }

        #[inline]
        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Text)
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            let mut opt_reset = self.common.configure(&cx.size_cx(), self.core.id());

            if let Some(_) = self.highlighter.configure(cx) {
                self.common.set_colors(self.highlighter.scheme_colors());
                opt_reset = Some(ActionResetStatus);
            }

            for (i, part) in self.parts.iter_mut().enumerate() {
                if opt_reset.is_some() {
                    part.require_reprepare();
                } else if part.status() >= Status::Shaped {
                    continue;
                }

                part.prepare_runs(&self.common, &mut self.highlighter);

                if i == 0 {
                    self.common.update_direction(part);
                }
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &(), event: Event) -> IsUsed {
            let action = self.common.handle_event(&mut self.parts, cx, event);
            let set_view_offset = if action.requires_repreparation() {
                // TODO(opt): skip updating unchanged parts
                let mut any_resized = false;
                let mut content_size = Size::ZERO;
                let mut rect = self.rect();

                for part in &mut self.parts {
                    if !part.is_ready() {
                        if part.status() < Status::Shaped {
                            part.prepare_runs(&self.common, &mut self.highlighter);
                        }
                        any_resized |= part.prepare_wrap(&self.common, rect.size.0);
                    }

                    let part_size = part.content_size();
                    rect.size.1 = part_size.1;
                    part.set_rect(&self.common, &mut cx.size_cx(), rect);

                    content_size.0 = content_size.0.max(part_size.0);
                    content_size.1 += part_size.1;
                    rect.pos.1 += part_size.1;
                }

                self.content_size = content_size;

                cx.redraw();
                if any_resized {
                    cx.resize();
                }
                any_resized
            } else {
                action.requires_set_view_offset()
            };
            if set_view_offset {
                cx.redraw();
                self.common.set_view_offset_from_cursor(&self.parts, cx);
            }

            match action {
                EventAction::Used
                | EventAction::Cursor
                | EventAction::FocusGained
                | EventAction::FocusLost
                | EventAction::Preedit => Used,
                EventAction::Edit { .. } => {
                    cx.push(CallOnEdit);
                    Used
                }
                EventAction::Unused | EventAction::Activate(_) => Unused,
            }
        }
    }
}
