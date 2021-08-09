// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scrollable and selectable label

use super::Scrollable;
use kas::draw::TextClass;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{self, Command, ScrollDelta};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::format::{EditableText, FormattableText};
use kas::text::SelectionHelper;

/// A text label supporting scrolling and selection
///
/// TODO: this should support copying to clipboard (like editbox), but it never
/// has keyboard focus and we don't yet have another way of capturing Ctrl+C.
#[derive(Clone, Default, Debug, Widget)]
#[widget(config(cursor_icon = event::CursorIcon::Text))]
#[handler(handle=noauto)]
pub struct ScrollLabel<T: FormattableText + 'static> {
    #[widget_core]
    core: CoreData,
    view_offset: Offset,
    text: Text<T>,
    required: Vec2,
    selection: SelectionHelper,
    input_handler: TextInput,
}

impl<T: FormattableText + 'static> Layout for ScrollLabel<T> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        size_handle.text_bound(&mut self.text, TextClass::LabelScroll, axis)
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let size = rect.size;
        self.required = self
            .text
            .update_env(|env| {
                env.set_align(align.unwrap_or(Align::Default, Align::Default));
                env.set_bounds(size.into());
            })
            .into();
        self.set_view_offset_from_edit_pos();
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        let class = TextClass::LabelScroll;
        draw_handle.with_clip_region(self.rect(), self.view_offset, &mut |draw_handle| {
            if self.selection.is_empty() {
                draw_handle.text(self.rect().pos, self.text.as_ref(), class);
            } else {
                // TODO(opt): we could cache the selection rectangles here to make
                // drawing more efficient (self.text.highlight_lines(range) output).
                // The same applies to the edit marker below.
                draw_handle.text_selected(
                    self.rect().pos,
                    &self.text,
                    self.selection.range(),
                    class,
                );
            }
        });
    }
}

impl<T: FormattableText + 'static> ScrollLabel<T> {
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

    fn set_edit_pos_from_coord(&mut self, mgr: &mut Manager, coord: Coord) {
        let rel_pos = (coord - self.rect().pos + self.view_offset).into();
        self.selection
            .set_edit_pos(self.text.text_index_nearest(rel_pos));
        self.set_view_offset_from_edit_pos();
        mgr.redraw(self.id());
    }

    // Pan by given delta. Return remaining (unused) delta.
    fn pan_delta(&mut self, mgr: &mut Manager, delta: Offset) -> Offset {
        let new_offset = (self.view_offset - delta).clamp(Offset::ZERO, self.max_scroll_offset());
        if new_offset != self.view_offset {
            let delta = delta - (self.view_offset - new_offset);
            self.view_offset = new_offset;
            mgr.redraw(self.id());
            delta
        } else {
            delta
        }
    }

    /// Update view_offset after edit_pos changes
    ///
    /// A redraw is assumed since edit_pos moved.
    fn set_view_offset_from_edit_pos(&mut self) {
        let edit_pos = self.selection.edit_pos();
        if let Some(marker) = self.text.text_glyph_pos(edit_pos).next_back() {
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

impl<T: FormattableText + 'static> HasStr for ScrollLabel<T> {
    fn get_str(&self) -> &str {
        self.text.as_str()
    }
}

impl<T: FormattableText + EditableText + 'static> HasString for ScrollLabel<T> {
    fn set_string(&mut self, string: String) -> TkAction {
        let avail = self.core.rect.size;
        kas::text::util::set_string_and_prepare(&mut self.text, string, avail)
    }
}

impl<T: FormattableText + 'static> event::Handler for ScrollLabel<T> {
    type Msg = VoidMsg;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::Command(cmd, _) => match cmd {
                Command::Escape | Command::Deselect if !self.selection.is_empty() => {
                    self.selection.set_empty();
                    mgr.redraw(self.id());
                    Response::None
                }
                Command::SelectAll => {
                    self.selection.set_sel_pos(0);
                    self.selection.set_edit_pos(self.text.str_len());
                    mgr.redraw(self.id());
                    Response::None
                }
                Command::Cut | Command::Copy => {
                    let range = self.selection.range();
                    mgr.set_clipboard((self.text.as_str()[range]).to_string());
                    Response::None
                }
                // TODO: scroll by command
                _ => Response::Unhandled,
            },
            Event::LostSelFocus => {
                self.selection.set_empty();
                mgr.redraw(self.id());
                Response::None
            }
            Event::Scroll(delta) => {
                let delta2 = match delta {
                    ScrollDelta::LineDelta(x, y) => {
                        // We arbitrarily scroll 3 lines:
                        let dist = 3.0 * self.text.env().height(Default::default());
                        Offset((x * dist).cast_nearest(), (y * dist).cast_nearest())
                    }
                    ScrollDelta::PixelDelta(coord) => coord,
                };
                match self.pan_delta(mgr, delta2) {
                    delta if delta == Offset::ZERO => Response::None,
                    delta => Response::Pan(delta),
                }
            }
            event => match self.input_handler.handle(mgr, self.id(), event) {
                TextInputAction::None | TextInputAction::Focus => Response::None,
                TextInputAction::Unhandled => Response::Unhandled,
                TextInputAction::Pan(delta) => match self.pan_delta(mgr, delta) {
                    delta if delta == Offset::ZERO => Response::None,
                    delta => Response::Pan(delta),
                },
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
                    Response::None
                }
            },
        }
    }
}

impl<T: FormattableText + 'static> Scrollable for ScrollLabel<T> {
    fn scroll_axes(&self, size: Size) -> (bool, bool) {
        let max = self.max_scroll_offset();
        (max.0 > size.0, max.1 > size.1)
    }

    fn max_scroll_offset(&self) -> Offset {
        let bounds = Vec2::from(self.text.env().bounds);
        let max_offset = (self.required - bounds).ceil();
        Offset::from(max_offset).max(Offset::ZERO)
    }

    fn scroll_offset(&self) -> Offset {
        self.view_offset
    }

    fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Offset) -> Offset {
        let new_offset = offset.clamp(Offset::ZERO, self.max_scroll_offset());
        if new_offset != self.view_offset {
            self.view_offset = new_offset;
            // No widget moves so do not need to report TkAction::REGION_MOVED
            mgr.redraw(self.id());
        }
        new_offset
    }
}
