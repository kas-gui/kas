// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scrollable and selectable label

use super::{ScrollBar, ScrollMsg};
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{Command, CursorIcon, FocusSource, Scroll};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::text::SelectionHelper;
use kas::theme::{Text, TextClass};

impl_scope! {
    /// A static text label supporting scrolling and selection
    ///
    /// Line-wrapping is enabled; default alignment is derived from the script
    /// (usually top-left).
    #[derive(Clone, Default, Debug)]
    #[widget{
        cursor_icon = CursorIcon::Text;
    }]
    pub struct ScrollLabel<T: FormattableText + 'static> {
        core: widget_core!(),
        view_offset: Offset,
        text: Text<T>,
        text_size: Size,
        selection: SelectionHelper,
        has_sel_focus: bool,
        input_handler: TextInput,
        #[widget]
        bar: ScrollBar<kas::dir::Down>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = sizer.text_rules(&mut self.text, axis);
            let _ = self.bar.size_rules(sizer.re(), axis);
            if axis.is_vertical() {
                rules.reduce_min_to(sizer.text_line_height(&self.text) * 4);
            }
            rules.with_stretch(Stretch::Low)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, mut rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.text.set_rect(cx, rect, hints);
            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();

            let max_offset = self.max_scroll_offset();
            self.view_offset = self.view_offset.min(max_offset);

            let w = cx.size_cx().scroll_bar_width().min(rect.size.0);
            rect.pos.0 += rect.size.0 - w;
            rect.size.0 = w;
            self.bar.set_rect(cx, rect, AlignHints::NONE);
            self.bar.set_limits(cx, max_offset.1, rect.size.1);
            self.bar.set_value(cx, self.view_offset.1);
        }

        fn draw(&self, mut draw: DrawCx) {
            let rect = Rect::new(self.rect().pos, self.text_size);
            draw.with_clip_region(self.rect(), self.view_offset, |mut draw| {
                if self.selection.is_empty() {
                    draw.text(rect, &self.text);
                } else {
                    // TODO(opt): we could cache the selection rectangles here to make
                    // drawing more efficient (self.text.highlight_lines(range) output).
                    // The same applies to the edit marker below.
                    draw.text_selected(rect, &self.text, self.selection.range());
                }
            });
            draw.with_pass(|mut draw| {
                self.bar.draw(draw.re());
            });
        }
    }

    impl Tile for Self {
        fn probe(&self, coord: Coord) -> Id {
            self.bar.try_probe(coord).unwrap_or_else(|| self.id())
        }
    }

    impl Self {
        /// Construct an `ScrollLabel` with the given inital `text`
        #[inline]
        pub fn new(text: T) -> Self {
            ScrollLabel {
                core: Default::default(),
                view_offset: Default::default(),
                text: Text::new(text, TextClass::LabelScroll),
                text_size: Size::ZERO,
                selection: SelectionHelper::new(0, 0),
                has_sel_focus: false,
                input_handler: Default::default(),
                bar: ScrollBar::new().with_invisible(true),
            }
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, cx: &mut EventState, text: T) {
            self.text.set_text(text);
            if self.text.prepare() != Ok(true) {
                return;
            }

            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();
            let max_offset = self.max_scroll_offset();
            self.bar.set_limits(cx, max_offset.1, self.rect().size.1);
            self.view_offset = self.view_offset.min(max_offset);

            self.selection.set_max_len(self.text.str_len());

            cx.redraw(self);
        }

        fn set_edit_pos_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
            let rel_pos = (coord - self.rect().pos + self.view_offset).cast();
            if let Ok(pos) = self.text.text_index_nearest(rel_pos) {
                if pos != self.selection.edit_pos() {
                    self.selection.set_edit_pos(pos);
                    self.set_view_offset_from_edit_pos(cx, pos);
                    self.bar.set_value(cx, self.view_offset.1);
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

        // Pan by given delta.
        fn pan_delta(&mut self, cx: &mut EventCx, mut delta: Offset, kinetic: bool) -> IsUsed {
            let new_offset = (self.view_offset - delta)
                .min(self.max_scroll_offset())
                .max(Offset::ZERO);
            if new_offset != self.view_offset {
                delta -= self.view_offset - new_offset;
                self.set_offset(cx, new_offset);
            }

            self.input_handler.set_scroll_residual(cx, delta, kinetic);
            Used
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
                let bounds = Vec2::conv(self.text.size());
                let min_x = marker.pos.0 - bounds.0;
                let min_y = marker.pos.1 - marker.descent - bounds.1;
                let max_x = marker.pos.0;
                let max_y = marker.pos.1 - marker.ascent;
                let min = Offset(min_x.cast_ceil(), min_y.cast_ceil());
                let max = Offset(max_x.cast_floor(), max_y.cast_floor());

                let max = max.min(self.max_scroll_offset());

                let new_offset = self.view_offset.max(min).min(max);
                if new_offset != self.view_offset {
                    self.view_offset = new_offset;
                    cx.set_scroll(Scroll::Scrolled);
                }
            }
        }

        /// Set offset, updating the scroll bar
        fn set_offset(&mut self, cx: &mut EventState, offset: Offset) {
            self.view_offset = offset;
            // unnecessary: cx.redraw(self);
            self.bar.set_value(cx, offset.1);
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl ScrollLabel<String> {
        /// Set text contents from a string
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            if self.text.set_string(string) && self.text.prepare().is_ok() {
                cx.action(self, Action::SET_RECT);
            }
        }
    }

    impl Events for Self {
        type Data = ();

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
                    // TODO: scroll by command
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
                Event::Scroll(delta) => {
                    self.pan_delta(cx, delta.as_offset(cx), false)
                }
                event => match self.input_handler.handle(cx, self.id(), event) {
                    TextInputAction::Used | TextInputAction::Finish => Used,
                    TextInputAction::Unused => Unused,
                    TextInputAction::Pan(delta, kinetic) => self.pan_delta(cx, delta, kinetic),
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

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(ScrollMsg(y)) = cx.try_pop() {
                let y = y.clamp(0, self.max_scroll_offset().1);
                self.view_offset.1 = y;
                cx.redraw(self);
            }
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            let max = self.max_scroll_offset();
            (max.0 > size.0, max.1 > size.1)
        }

        fn max_scroll_offset(&self) -> Offset {
            let text_size = Offset::conv(self.text_size);
            let self_size = Offset::conv(self.rect().size);
            (text_size - self_size).max(Offset::ZERO)
        }

        fn scroll_offset(&self) -> Offset {
            self.view_offset
        }

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                self.set_offset(cx, new_offset);
                // No widget moves so do not need to report Action::REGION_MOVED
            }
            new_offset
        }
    }
}
