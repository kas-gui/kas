// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scrollable and selectable label

use super::{ScrollBar, ScrollMsg};
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{Command, CursorIcon, Scroll, ScrollDelta};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::format::{EditableText, FormattableText};
use kas::text::{SelectionHelper, Text};
use kas::theme::TextClass;

impl_scope! {
    /// A text label supporting scrolling and selection
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
        input_handler: TextInput,
        #[widget]
        bar: ScrollBar<kas::dir::Down>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let class = TextClass::LabelScroll;
            let mut rules = size_mgr.text_rules(&mut self.text, class, axis);
            if axis.is_vertical() {
                rules.reduce_min_to(size_mgr.line_height(class) * 4);
            }
            rules
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, mut rect: Rect) {
            self.core.rect = rect;
            mgr.text_set_size(&mut self.text, TextClass::LabelScroll, rect.size, None);
            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();

            let max_offset = self.max_scroll_offset();
            self.view_offset = self.view_offset.min(max_offset);

            let w = mgr.size_mgr().scroll_bar_width().min(rect.size.0);
            rect.pos.0 += rect.size.0 - w;
            rect.size.0 = w;
            self.bar.set_rect(mgr, rect);
            let _ = self.bar.set_limits(max_offset.1, rect.size.1);
            self.bar.set_value(mgr, self.view_offset.1);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            self.bar.find_id(coord).or_else(|| Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let class = TextClass::LabelScroll;
            let rect = Rect::new(self.rect().pos, self.text_size);
            draw.with_clip_region(self.rect(), self.view_offset, |mut draw| {
                if self.selection.is_empty() {
                    draw.text(rect, &self.text, class);
                } else {
                    // TODO(opt): we could cache the selection rectangles here to make
                    // drawing more efficient (self.text.highlight_lines(range) output).
                    // The same applies to the edit marker below.
                    draw.text_selected(rect, &self.text, self.selection.range(), class);
                }
            });
            draw.with_pass(|mut draw| {
                draw.recurse(&mut self.bar);
            });
        }
    }

    impl Self {
        /// Construct an `ScrollLabel` with the given inital `text`
        #[inline]
        pub fn new(text: T) -> Self {
            ScrollLabel {
                core: Default::default(),
                view_offset: Default::default(),
                text: Text::new(text),
                text_size: Size::ZERO,
                selection: SelectionHelper::new(0, 0),
                input_handler: Default::default(),
                bar: ScrollBar::new().with_invisible(true),
            }
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: T) -> Action {
            self.text
                .set_and_try_prepare(text)
                .expect("invalid font_id");

            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();
            let max_offset = self.max_scroll_offset();
            let _ = self.bar.set_limits(max_offset.1, self.rect().size.1);
            self.view_offset = self.view_offset.min(max_offset);

            self.selection.set_max_len(self.text.str_len());

            Action::REDRAW
        }

        fn set_edit_pos_from_coord(&mut self, mgr: &mut EventMgr, coord: Coord) {
            let rel_pos = (coord - self.rect().pos + self.view_offset).cast();
            if let Ok(pos) = self.text.text_index_nearest(rel_pos) {
                if pos != self.selection.edit_pos() {
                    self.selection.set_edit_pos(pos);
                    self.set_view_offset_from_edit_pos(mgr, pos);
                    self.bar.set_value(mgr, self.view_offset.1);
                    mgr.redraw(self.id());
                }
            }
        }

        fn set_primary(&self, mgr: &mut EventMgr) {
            if !self.selection.is_empty() {
                let range = self.selection.range();
                mgr.set_primary(String::from(&self.text.as_str()[range]));
            }
        }

        // Pan by given delta. Return `Response::Scrolled` or `Response::Pan(remaining)`.
        fn pan_delta(&mut self, mgr: &mut EventMgr, mut delta: Offset) -> Response {
            let new_offset = (self.view_offset - delta)
                .min(self.max_scroll_offset())
                .max(Offset::ZERO);
            if new_offset != self.view_offset {
                delta -= self.view_offset - new_offset;
                self.set_offset(mgr, new_offset);
            }

            mgr.set_scroll(if delta == Offset::ZERO {
                Scroll::Scrolled
            } else {
                Scroll::Offset(delta)
            });
            Response::Used
        }

        /// Update view_offset from edit_pos
        ///
        /// This method is mostly identical to its counterpart in `EditField`.
        fn set_view_offset_from_edit_pos(&mut self, mgr: &mut EventMgr, edit_pos: usize) {
            if let Some(marker) = self
                .text
                .text_glyph_pos(edit_pos)
                .ok()
                .and_then(|mut m| m.next_back())
            {
                let bounds = Vec2::from(self.text.env().bounds);
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
                    mgr.set_scroll(Scroll::Scrolled);
                }
            }
        }

        /// Set offset, updating the scroll bar
        fn set_offset(&mut self, mgr: &mut EventState, offset: Offset) {
            self.view_offset = offset;
            // unnecessary: mgr.redraw(self.id());
            self.bar.set_value(mgr, offset.1);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl HasString for Self
    where
        T: EditableText,
    {
        fn set_string(&mut self, string: String) -> Action {
            self.text.set_string(string);
            let _ = self.text.try_prepare();
            Action::REDRAW
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, _: &Self::Data, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(cmd) => match cmd {
                    Command::Escape | Command::Deselect if !self.selection.is_empty() => {
                        self.selection.set_empty();
                        mgr.redraw(self.id());
                        Response::Used
                    }
                    Command::SelectAll => {
                        self.selection.set_sel_pos(0);
                        self.selection.set_edit_pos(self.text.str_len());
                        self.set_primary(mgr);
                        mgr.redraw(self.id());
                        Response::Used
                    }
                    Command::Cut | Command::Copy => {
                        let range = self.selection.range();
                        mgr.set_clipboard((self.text.as_str()[range]).to_string());
                        Response::Used
                    }
                    // TODO: scroll by command
                    _ => Response::Unused,
                },
                Event::LostSelFocus => {
                    self.selection.set_empty();
                    mgr.redraw(self.id());
                    Response::Used
                }
                Event::Scroll(delta) => {
                    let delta2 = match delta {
                        ScrollDelta::LineDelta(x, y) => mgr.config().scroll_distance((x, y)),
                        ScrollDelta::PixelDelta(coord) => coord,
                    };
                    self.pan_delta(mgr, delta2)
                }
                event => match self.input_handler.handle(mgr, self.id(), event) {
                    TextInputAction::None | TextInputAction::Focus => Response::Used,
                    TextInputAction::Unused => Response::Unused,
                    TextInputAction::Pan(delta) => self.pan_delta(mgr, delta),
                    TextInputAction::Cursor(coord, anchor, clear, repeats) => {
                        if (clear && repeats <= 1) || mgr.request_sel_focus(self.id()) {
                            self.set_edit_pos_from_coord(mgr, coord);
                            if anchor {
                                self.selection.set_anchor();
                            }
                            if clear {
                                self.selection.set_empty();
                            }
                            if repeats > 1 {
                                self.selection.expand(&self.text, repeats);
                            }
                            self.set_primary(mgr);
                        }
                        Response::Used
                    }
                },
            }
        }

        fn handle_message(&mut self, _: &Self::Data, mgr: &mut EventMgr) {
            if let Some(ScrollMsg(y)) = mgr.try_pop() {
                let y = y.clamp(0, self.max_scroll_offset().1);
                self.view_offset.1 = y;
                mgr.redraw(self.id());
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

        fn set_scroll_offset(&mut self, _: &Self::Data, mgr: &mut EventMgr, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                self.set_offset(mgr, new_offset);
                // No widget moves so do not need to report Action::REGION_MOVED
            }
            new_offset
        }
    }
}
