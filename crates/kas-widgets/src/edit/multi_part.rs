// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`MultiPartEditor`] widget

use super::editor::{Component, EventAction};
use crate::edit::highlight::{Highlighter, Plain};
use crate::{ScrollBar, ScrollBarMsg};
use kas::cast::Ceil;
use kas::event::components::ScrollComponent;
use kas::event::{CursorIcon, Scroll};
use kas::prelude::*;
use kas::text::Direction;
use kas::theme::{FrameStyle, TextClass};
use std::rc::Rc;

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
            if !self.inner.editor.0.text_is_rtl() {
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
            let bg = self.inner.editor.background_color();
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
            self.update_content_size(cx);
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
        pub fn new(text: impl ToString) -> Self {
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
            self.inner.editor.set_highlighter(highlighter);
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
        /// The whole contents equals the concatenation of parts.
        pub fn text_parts(&self) -> impl Iterator<Item = &Rc<String>> {
            std::iter::once(self.inner.editor.part().text())
        }

        /// Copy text contents to a `String`
        pub fn text_to_string(&self) -> String {
            let mut s = String::new();
            for part in self.text_parts() {
                s.push_str(part);
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
            self.inner.editor.set_direction(direction);
            self
        }

        /// Set the initial text (inline)
        ///
        /// This method should only be used on a new `MultiPartEditor`.
        #[inline]
        #[must_use]
        pub fn with_text(mut self, text: impl ToString) -> Self {
            self.inner.editor = self.inner.editor.with_text(text);
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
    #[autoimpl(Debug where H: trait)]
    #[widget]
    #[layout(self.editor)]
    struct Inner<H: Highlighter = Plain> {
        core: widget_core!(),
        width: (f32, f32),
        editor: Component<H>,
    }

    impl Default for Self
    where
        H: Default,
    {
        fn default() -> Self {
            Inner {
                core: Default::default(),
                width: (8.0, 16.0),
                editor: Component::new(true),
            }
        }
    }

    impl Self {
        /// Set the initial text (inline)
        ///
        /// This method should only be used on a new `Inner`.
        #[inline]
        #[must_use]
        fn with_text(mut self, text: impl ToString) -> Self {
            self.editor = self.editor.with_text(text);
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
                editor: self.editor.with_highlighter(highlighter),
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
            } else if let Some(width) = axis.other() {
                let h = self.editor.measure_height(width.cast(), None).cast_to(Ceil);
                min = h;
                ideal = h;
            } else {
                unreachable!()
            };

            let rules = self.editor.size_rules(cx, axis);
            ideal = ideal.max(rules.ideal_size());

            SizeRules::new(min, ideal, Stretch::High).with_margins(cx.text_margins().extract(axis))
        }

        #[inline]
        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, mut hints: AlignHints) {
            hints.vert = Some(Align::Default);
            self.editor.set_rect(cx, rect, hints);
        }
    }

    impl Viewport for Self {
        #[inline]
        fn content_size(&self) -> Size {
            self.editor.part().content_size()
        }

        #[inline]
        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            self.editor.draw_with_offset(draw, rect, offset);
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
            self.editor.configure(cx, self.id());
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &(), event: Event) -> IsUsed {
            match self.editor.handle_event(cx, event) {
                EventAction::Used
                | EventAction::Cursor
                | EventAction::FocusGained
                | EventAction::FocusLost
                | EventAction::Preedit => Used,
                EventAction::Edit => {
                    cx.push(CallOnEdit);
                    Used
                }
                EventAction::Unused | EventAction::Activate(_) => Unused,
            }
        }
    }
}
