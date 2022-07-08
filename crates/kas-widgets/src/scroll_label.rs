// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scrollable and selectable label

use kas::event::components::{TextInput, TextInputAction};
use kas::event::{Command, CursorIcon, Scroll, ScrollDelta};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::format::{EditableText, FormattableText};
use kas::text::SelectionHelper;
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
        required: Vec2,
        selection: SelectionHelper,
        input_handler: TextInput,
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            size_mgr.text_bound(&mut self.text, TextClass::LabelScroll, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let align = align.unwrap_or(Align::Default, Align::Default);
            self.required = mgr.text_set_size(&mut self.text, TextClass::LabelScroll, rect.size, align);
            self.set_view_offset_from_edit_pos();
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let class = TextClass::LabelScroll;
            draw.with_clip_region(self.rect(), self.view_offset, |mut draw| {
                if self.selection.is_empty() {
                    draw.text(self.rect().pos, &self.text, class);
                } else {
                    // TODO(opt): we could cache the selection rectangles here to make
                    // drawing more efficient (self.text.highlight_lines(range) output).
                    // The same applies to the edit marker below.
                    draw.text_selected(
                        self.rect().pos,
                        &self.text,
                        self.selection.range(),
                        class,
                    );
                }
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
                text: Text::new_multi(text),
                required: Vec2::ZERO,
                selection: SelectionHelper::new(0, 0),
                input_handler: Default::default(),
            }
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: T) -> TkAction {
            kas::text::util::set_text_and_prepare(&mut self.text, text, self.core.rect.size)
        }

        fn set_edit_pos_from_coord(&mut self, mgr: &mut EventMgr, coord: Coord) {
            let rel_pos = (coord - self.rect().pos + self.view_offset).cast();
            if let Ok(pos) = self.text.text_index_nearest(rel_pos) {
                self.selection.set_edit_pos(pos);
            }
            self.set_view_offset_from_edit_pos();
            mgr.redraw(self.id());
        }

        // Pan by given delta. Return `Response::Scrolled` or `Response::Pan(remaining)`.
        fn pan_delta(&mut self, mgr: &mut EventMgr, mut delta: Offset) -> Response {
            let new_offset = (self.view_offset - delta).min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                delta -= self.view_offset - new_offset;
                self.view_offset = new_offset;
                mgr.redraw(self.id());
            }

            mgr.set_scroll(if delta == Offset::ZERO {
                    Scroll::Scrolled
                } else {
                    Scroll::Offset(delta)
                }
            );
            Response::Used
        }

        /// Update view_offset after edit_pos changes
        ///
        /// A redraw is assumed since edit_pos moved.
        fn set_view_offset_from_edit_pos(&mut self) {
            let edit_pos = self.selection.edit_pos();
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

                self.view_offset = self.view_offset.max(min).min(max);
            }
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
        fn set_string(&mut self, string: String) -> TkAction {
            let avail = self.core.rect.size;
            kas::text::util::set_string_and_prepare(&mut self.text, string, avail)
        }
    }

    impl Widget for Self {
        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
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
                        ScrollDelta::LineDelta(x, y) => {
                            // We arbitrarily scroll 3 Em:
                            let dist = 3.0 * self.text.env().dpem;
                            Offset((x * dist).cast_nearest(), (y * dist).cast_nearest())
                        }
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
                        }
                        Response::Used
                    }
                },
            }
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            let max = self.max_scroll_offset();
            (max.0 > size.0, max.1 > size.1)
        }

        fn max_scroll_offset(&self) -> Offset {
            let bounds = Vec2::from(self.text.env().bounds);
            let max_offset = Offset::conv_ceil(self.required - bounds);
            max_offset.max(Offset::ZERO)
        }

        fn scroll_offset(&self) -> Offset {
            self.view_offset
        }

        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                self.view_offset = new_offset;
                // No widget moves so do not need to report TkAction::REGION_MOVED
                mgr.redraw(self.id());
            }
            new_offset
        }
    }
}
