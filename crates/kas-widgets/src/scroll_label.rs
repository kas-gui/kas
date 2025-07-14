// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scrollable and selectable label

use super::{ScrollBar, ScrollMsg};
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{Command, CursorIcon, FocusSource, Scroll};
use kas::prelude::*;
use kas::text::SelectionHelper;
use kas::text::format::FormattableText;
use kas::theme::{Text, TextClass};

#[impl_self]
mod SelectableLabel {
    /// A text label supporting selection
    ///
    /// Line-wrapping is enabled; default alignment is derived from the script
    /// (usually top-left).
    #[derive(Clone, Default, Debug)]
    #[widget]
    #[layout(self.text)]
    pub struct SelectableLabel<T: FormattableText + 'static> {
        core: widget_core!(),
        text: Text<T>,
        selection: SelectionHelper,
        has_sel_focus: bool,
        input_handler: TextInput,
    }

    impl Layout for Self {
        fn draw(&self, mut draw: DrawCx) {
            if self.selection.is_empty() {
                draw.text(self.rect(), &self.text);
            } else {
                // TODO(opt): we could cache the selection rectangles here to make
                // drawing more efficient (self.text.highlight_lines(range) output).
                // The same applies to the edit marker below.
                draw.text_selected(self.rect(), &self.text, self.selection.range());
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Text {
                text: self.text.as_str(),
                editable: false,
                edit_pos: self.selection.edit_pos(),
                sel_pos: self.selection.sel_pos(),
            }
        }
    }

    impl Self {
        /// Construct a `SelectableLabel` with the given inital `text`
        #[inline]
        pub fn new(text: T) -> Self {
            SelectableLabel {
                core: Default::default(),
                text: Text::new(text, TextClass::LabelScroll),
                selection: SelectionHelper::new(0, 0),
                has_sel_focus: false,
                input_handler: Default::default(),
            }
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: T) -> bool {
            self.text.set_text(text);
            if !self.text.prepare() {
                return false;
            }

            self.selection.set_max_len(self.text.str_len());
            true
        }

        fn set_edit_pos_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
            let rel_pos = (coord - self.rect().pos).cast();
            if let Ok(pos) = self.text.text_index_nearest(rel_pos) {
                if pos != self.selection.edit_pos() {
                    self.selection.set_edit_pos(pos);
                    self.set_view_offset_from_edit_pos(cx, pos);
                    cx.redraw(self);
                }
            }
        }

        fn set_primary(&self, cx: &mut EventCx) {
            if self.has_sel_focus && !self.selection.is_empty() && cx.has_primary() {
                let range = self.selection.range();
                cx.set_primary(String::from(&self.text.as_str()[range]));
            }
        }

        /// Update view_offset from edit_pos
        ///
        /// This method is mostly identical to its counterpart in `EditField`.
        fn set_view_offset_from_edit_pos(&mut self, cx: &mut EventCx, edit_pos: usize) {
            if let Some(marker) = self
                .text
                .text_glyph_pos(edit_pos)
                .ok()
                .and_then(|mut m| m.next_back())
            {
                let y0 = (marker.pos.1 - marker.ascent).cast_floor();
                let pos = Coord(marker.pos.0.cast_nearest(), y0);
                let size = Size(0, i32::conv_ceil(marker.pos.1 - marker.descent) - y0);
                cx.set_scroll(Scroll::Rect(Rect { pos, size }));
            }
        }

        /// Get text contents
        #[inline]
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl SelectableLabel<String> {
        /// Set text contents from a string
        #[inline]
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            if self.text.set_string(string) {
                self.text.prepare();
                cx.action(self, Action::SET_RECT);
            }
        }
    }

    impl Events for Self {
        type Data = ();

        #[inline]
        fn hover_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Text)
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, _) => match cmd {
                    Command::Escape | Command::Deselect if !self.selection.is_empty() => {
                        self.selection.set_empty();
                        cx.redraw(self);
                        Used
                    }
                    Command::SelectAll => {
                        self.selection.set_sel_pos(0);
                        self.selection.set_edit_pos(self.text.str_len());
                        self.set_primary(cx);
                        cx.redraw(self);
                        Used
                    }
                    Command::Cut | Command::Copy => {
                        let range = self.selection.range();
                        cx.set_clipboard((self.text.as_str()[range]).to_string());
                        Used
                    }
                    _ => Unused,
                },
                Event::SelFocus(source) => {
                    self.has_sel_focus = true;
                    if source == FocusSource::Pointer {
                        self.set_primary(cx);
                    }
                    Used
                }
                Event::LostSelFocus => {
                    self.has_sel_focus = false;
                    self.selection.set_empty();
                    cx.redraw(self);
                    Used
                }
                event => match self.input_handler.handle(cx, self.id(), event) {
                    TextInputAction::Used | TextInputAction::Finish => Used,
                    TextInputAction::Unused => Unused,
                    TextInputAction::Pan(delta, kinetic) => {
                        cx.push((delta, kinetic));
                        Used
                    }
                    TextInputAction::Focus { coord, action } => {
                        self.set_edit_pos_from_coord(cx, coord);
                        self.selection.action(&self.text, action);

                        if self.has_sel_focus {
                            self.set_primary(cx);
                        } else {
                            cx.request_sel_focus(self.id(), FocusSource::Pointer);
                        }
                        Used
                    }
                },
            }
        }
    }
}

#[impl_self]
mod ScrollLabel {
    /// A text label supporting scrolling and selection
    ///
    /// Line-wrapping is enabled; default alignment is derived from the script
    /// (usually top-left).
    #[derive(Clone, Default, Debug)]
    #[widget]
    pub struct ScrollLabel<T: FormattableText + 'static> {
        core: widget_core!(),
        offset: Offset,
        max_offset: Offset,
        #[widget]
        label: SelectableLabel<T>,
        #[widget]
        bar: ScrollBar<kas::dir::Down>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = self.label.size_rules(sizer.re(), axis);
            let _ = self.bar.size_rules(sizer.re(), axis);
            if axis.is_vertical() {
                rules.reduce_min_to((sizer.dpem() * 4.0).cast_ceil());
            }
            rules.with_stretch(Stretch::Low)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, mut rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.label.set_rect(cx, rect, hints);

            let inner_size = Offset::conv(self.label.rect().size);
            let self_size = Offset::conv(self.rect().size);
            self.max_offset = (inner_size - self_size).max(Offset::ZERO);
            self.offset = self.offset.min(self.max_offset);

            let w = cx.size_cx().scroll_bar_width().min(rect.size.0);
            rect.pos.0 += rect.size.0 - w;
            rect.size.0 = w;
            self.bar.set_rect(cx, rect, AlignHints::NONE);
            self.bar.set_limits(cx, self.max_offset.1, rect.size.1);
            self.bar.set_value(cx, self.offset.1);
        }

        fn draw(&self, mut draw: DrawCx) {
            draw.with_clip_region(self.rect(), self.offset, |draw| self.label.draw(draw));
            draw.with_pass(|draw| self.bar.draw(draw));
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll_offset(),
                max_offset: self.max_scroll_offset(),
            }
        }

        fn probe(&self, coord: Coord) -> Id {
            self.bar.try_probe(coord).unwrap_or_else(|| self.label.id())
        }
    }

    impl Self {
        /// Construct an `ScrollLabel` with the given inital `text`
        #[inline]
        pub fn new(text: T) -> Self {
            ScrollLabel {
                core: Default::default(),
                offset: Offset::ZERO,
                max_offset: Offset::ZERO,
                label: SelectableLabel::new(text),
                bar: ScrollBar::new().with_invisible(true),
            }
        }

        /// Replace text
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, cx: &mut EventState, text: T) {
            if !self.label.set_text(text) {
                return;
            }

            let inner_size = Offset::conv(self.label.rect().size);
            let self_size = Offset::conv(self.rect().size);
            self.max_offset = (inner_size - self_size).max(Offset::ZERO);
            self.offset = self.offset.min(self.max_offset);
            self.bar
                .set_limits(cx, self.max_offset.1, self.rect().size.1);

            cx.redraw(self);
        }

        // Pan by given delta.
        fn pan_delta(&mut self, cx: &mut EventCx, mut delta: Offset, kinetic: bool) {
            let new_offset = (self.offset - delta)
                .min(self.max_scroll_offset())
                .max(Offset::ZERO);
            if new_offset != self.offset {
                delta -= self.offset - new_offset;
                self.set_offset(cx, new_offset);
            }

            self.label
                .input_handler
                .set_scroll_residual(cx, delta, kinetic);
        }

        /// Set offset, updating the scroll bar
        fn set_offset(&mut self, cx: &mut EventState, offset: Offset) {
            self.offset = offset;
            // unnecessary: cx.redraw(self);
            self.bar.set_value(cx, offset.1);
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl ScrollLabel<String> {
        /// Set text contents from a string
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            self.label.set_string(cx, string);
        }
    }

    impl Events for Self {
        type Data = ();

        #[inline]
        fn hover_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Text)
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                // TODO: scroll by Event::Command(_)
                Event::Scroll(delta) => {
                    self.pan_delta(cx, delta.as_offset(cx), false);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(ScrollMsg(y)) = cx.try_pop() {
                let y = y.clamp(0, self.max_scroll_offset().1);
                self.offset.1 = y;
                cx.redraw(self);
            } else if let Some((delta, kinetic)) = cx.try_pop() {
                self.pan_delta(cx, delta, kinetic);
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, scroll: Scroll) {
            match scroll {
                Scroll::None | Scroll::Scrolled => (),
                Scroll::Offset(delta) => self.pan_delta(cx, delta, false),
                Scroll::Kinetic(start) => {
                    let delta = self.label.input_handler.kinetic_start(start);
                    self.pan_delta(cx, delta, true);
                }
                Scroll::Rect(rect) => {
                    self.label.input_handler.kinetic_stop();
                    let window_rect = self.rect();
                    let v = rect.pos - window_rect.pos;
                    let off = Offset::conv(rect.size) - Offset::conv(window_rect.size);
                    let offset = self.offset.max(v + off).min(v);
                    self.set_offset(cx, offset);

                    cx.set_scroll(Scroll::Rect(rect - self.offset));
                }
            }
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            let max = self.max_scroll_offset();
            (max.0 > size.0, max.1 > size.1)
        }

        fn max_scroll_offset(&self) -> Offset {
            self.max_offset
        }

        fn scroll_offset(&self) -> Offset {
            self.offset
        }

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.offset {
                self.set_offset(cx, new_offset);
                // No widget moves so do not need to report Action::REGION_MOVED
            }
            new_offset
        }
    }
}
