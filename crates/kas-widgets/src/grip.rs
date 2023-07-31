// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `GripPart` control

use std::fmt::Debug;

use kas::event::{CursorIcon, Press};
use kas::prelude::*;

/// A message from a [`GripPart`]
#[derive(Clone, Debug)]
pub enum GripMsg {
    /// Widget received [`Event::PressStart`]
    ///
    /// Some parents will call [`EventState::set_nav_focus`] on this event.
    PressStart,
    /// Widget received [`Event::PressMove`]
    ///
    /// Parameter: the new position of the grip relative to the track.
    ///
    /// The grip position is not adjusted; the caller should also call
    /// [`GripPart::set_offset`] to do so. This is separate to allow adjustment of
    /// the posision; e.g. `Slider` pins the position to the nearest detent.
    PressMove(Offset),
    /// Widget received [`Event::PressEnd`]
    ///
    /// Parameter: `success` (see [`Event::PressEnd`]).
    PressEnd(bool),
}

impl_scope! {
    /// A draggable grip part
    ///
    /// [`Slider`](crate::Slider), [`ScrollBar`](crate::ScrollBar) and
    /// [`Splitter`](crate::Splitter) all require a component which supports
    /// click+drag behaviour. The appearance differs but event handling is the
    /// same: this widget is its implementation.
    ///
    /// # Layout
    ///
    /// This widget is unusual in several ways:
    ///
    /// 1.  [`Layout::size_rules`] does not request any size; the parent is expected
    ///     to do this.
    /// 2.  [`Layout::set_rect`] sets the *track* within which this grip may move;
    ///     the parent should always call [`GripPart::set_size_and_offset`]
    ///     afterwards to set the grip position.
    /// 3.  [`Layout::draw`] does nothing. The parent should handle all drawing.
    ///
    /// # Event handling
    ///
    /// This widget handles click/touch events on the widget, pushing a
    /// [`GripMsg`] to allow the parent to implement further handling.
    ///
    /// Optionally, the parent may call [`GripPart::handle_press_on_track`]
    /// when a [`Event::PressStart`] occurs on the track area (which identifies
    /// as being the parent widget).
    #[derive(Clone, Debug, Default)]
    #[widget{
        hover_highlight = true;
        cursor_icon = CursorIcon::Grab;
    }]
    pub struct GripPart {
        core: widget_core!(),
        // The track is the area within which this GripPart may move
        track: Rect,
        press_coord: Coord,
    }

    /// This implementation is unusual in that:
    ///
    /// 1.  `size_rules` always returns [`SizeRules::EMPTY`]
    /// 2.  `set_rect` sets the *track* within which this grip may move; the
    ///     parent should call [`GripPart::set_size_and_offset`] after
    ///     `set_rect` (otherwise the grip's position will not be updated)
    /// 3.  `draw` does nothing: the parent is expected to do all drawing
    impl Layout for GripPart {
        fn size_rules(&mut self, _: SizeMgr, _: AxisInfo) -> SizeRules {
            SizeRules::EMPTY
        }

        fn set_rect(&mut self, _: &mut ConfigCx, rect: Rect) {
            self.track = rect;
        }

        fn draw(&mut self, _: DrawCx) {}
    }

    impl Events for GripPart {
        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> Response {
            match event {
                Event::PressStart { press, .. } => {
                    cx.push(GripMsg::PressStart);
                    press.grab(self.id())
                        .with_icon(CursorIcon::Grabbing)
                        .with_cx(cx);

                    // Event delivery implies coord is over the grip.
                    self.press_coord = press.coord - self.offset();
                    Response::Used
                }
                Event::PressMove { press, .. } => {
                    let offset = press.coord - self.press_coord;
                    let offset = offset.clamp(Offset::ZERO, self.max_offset());
                    cx.push(GripMsg::PressMove(offset));
                    Response::Used
                }
                Event::PressEnd { success, .. } => {
                    cx.push(GripMsg::PressEnd(success));
                    Response::Used
                }
                _ => Response::Unused,
            }
        }
    }
}

impl GripPart {
    /// Construct
    pub fn new() -> Self {
        GripPart {
            core: Default::default(),
            track: Default::default(),
            press_coord: Coord::ZERO,
        }
    }

    /// Set a new grip size and position
    ///
    /// Returns [`Action::REDRAW`] if a redraw is required.
    pub fn set_size_and_offset(&mut self, size: Size, offset: Offset) -> Action {
        self.core.rect.size = size;
        self.set_offset(offset).1
    }

    /// Get the current track `Rect`
    #[inline]
    pub fn track(&self) -> Rect {
        self.track
    }

    /// Get the current grip position
    #[inline]
    pub fn offset(&self) -> Offset {
        self.core.rect.pos - self.track.pos
    }

    /// Get the maximum allowed offset
    ///
    /// The grip position is clamped between `ZERO` and this offset relative to
    /// the track. This value depends on size of the grip and the track.
    #[inline]
    pub fn max_offset(&self) -> Offset {
        Offset::conv(self.track.size) - Offset::conv(self.core.rect.size)
    }

    /// Set a new grip position
    ///
    /// Returns the new position (after clamping input) and an action: empty if
    /// the grip hasn't moved; `REDRAW` if it has (though this widget is
    /// not directly responsible for drawing, so this may not be accurate).
    pub fn set_offset(&mut self, offset: Offset) -> (Offset, Action) {
        let offset = offset.min(self.max_offset()).max(Offset::ZERO);
        let handle_pos = self.track.pos + offset;
        if handle_pos != self.core.rect.pos {
            self.core.rect.pos = handle_pos;
            (offset, Action::REDRAW)
        } else {
            (offset, Action::empty())
        }
    }

    /// Handle an event on the track itself
    ///
    /// If it is desired to make the grip move when the track area is clicked,
    /// then the parent widget should call this method when receiving
    /// [`Event::PressStart`].
    ///
    /// Returns the new grip position relative to the track.
    ///
    /// The grip position is not adjusted; the caller should also call
    /// [`Self::set_offset`] to do so. This is separate to allow adjustment of
    /// the posision; e.g. `Slider` pins the position to the nearest detent.
    pub fn handle_press_on_track(&mut self, cx: &mut EventCx, press: &Press) -> Offset {
        press
            .grab(self.id())
            .with_icon(CursorIcon::Grabbing)
            .with_cx(cx);

        let offset = press.coord - self.track.pos - Offset::conv(self.core.rect.size / 2);
        let offset = offset.clamp(Offset::ZERO, self.max_offset());
        self.press_coord = press.coord - offset;
        offset
    }
}
