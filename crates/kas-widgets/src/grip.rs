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

#[impl_self]
mod GripPart {
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
    /// [`Layout::size_rules`] does not request any size; the parent is expected
    /// to determine the grip's size.
    /// (Calling `size_rules` is still required to comply with widget model.)
    ///
    /// [`Layout::set_rect`] sets the grip's rect directly.
    /// [`Self::set_track`] must be called first.
    ///
    /// Often it is preferable to use [`Self::set_size`] to set the grip's size
    /// then [`Self::set_offset`] to set the position.
    /// (Calling `set_rect` is still required to comply with widget model.)
    ///
    /// [`Layout::draw`] does nothing. The parent should handle all drawing.
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
    #[widget]
    pub struct GripPart {
        core: widget_core!(),
        // The track is the area within which this GripPart may move
        track: Rect,
        // The position of the grip handle
        rect: Rect,
        press_coord: Coord,
    }

    /// This implementation is unusual (see [`GripPart`] documentation).
    impl Layout for GripPart {
        fn rect(&self) -> Rect {
            self.rect
        }

        fn size_rules(&mut self, _: SizeCx, _axis: AxisInfo) -> SizeRules {
            SizeRules::EMPTY
        }

        fn set_rect(&mut self, _: &mut ConfigCx, rect: Rect, _: AlignHints) {
            self.rect = rect;
        }

        fn draw(&self, _: DrawCx) {}
    }

    impl Tile for Self {
        #[cfg(feature = "accesskit")]
        fn accesskit_node(&self) -> Option<accesskit::Node> {
            // There is no Role::Grip. There are roles for scrollbars, splitters
            // and sliders (the widgets which use a grip).
            None
        }
    }

    impl Events for GripPart {
        const REDRAW_ON_HOVER: bool = true;

        type Data = ();

        #[inline]
        fn hover_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Grab)
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::PressStart { press, .. } => {
                    cx.push(GripMsg::PressStart);
                    press
                        .grab(self.id(), kas::event::GrabMode::Grab)
                        .with_icon(CursorIcon::Grabbing)
                        .complete(cx);

                    // Event delivery implies coord is over the grip.
                    self.press_coord = press.coord - self.offset();
                    Used
                }
                Event::PressMove { press, .. } => {
                    let offset = press.coord - self.press_coord;
                    let offset = offset.clamp(Offset::ZERO, self.max_offset());
                    cx.push(GripMsg::PressMove(offset));
                    Used
                }
                Event::PressEnd { success, .. } => {
                    cx.push(GripMsg::PressEnd(success));
                    Used
                }
                _ => Unused,
            }
        }
    }

    impl GripPart {
        /// Construct
        pub fn new() -> Self {
            GripPart {
                core: Default::default(),
                track: Default::default(),
                rect: Default::default(),
                press_coord: Coord::ZERO,
            }
        }

        /// Set the track
        ///
        /// The `track` is the region within which the grip may be moved.
        ///
        /// This method must be called to set the `track`, presumably from the
        /// parent widget's [`Layout::set_rect`] method.
        /// It is expected that [`GripPart::set_offset`] is called after this.
        pub fn set_track(&mut self, track: Rect) {
            self.track = track;
        }

        /// Get the current track `Rect`
        #[inline]
        pub fn track(&self) -> Rect {
            self.track
        }

        /// Set the grip's size
        ///
        /// It is expected that for each axis the `size` is no larger than the size
        /// of the `track` (see [`GripPart::set_track`]). If equal, then the grip
        /// may not be moved on this axis.
        ///
        /// This method must be called at least once.
        /// It is expected that [`GripPart::set_offset`] is called after this.
        ///
        /// This size may be read via `self.rect().size`.
        pub fn set_size(&mut self, size: Size) {
            self.rect.size = size;
        }

        /// Get the current grip position
        ///
        /// The position returned is relative to `self.track().pos` and is always
        /// between [`Offset::ZERO`] and [`Self::max_offset`].
        #[inline]
        pub fn offset(&self) -> Offset {
            self.rect.pos - self.track.pos
        }

        /// Get the maximum allowed offset
        ///
        /// This is the maximum allowed [`Self::offset`], equal to the size of the
        /// track minus the size of the grip.
        #[inline]
        pub fn max_offset(&self) -> Offset {
            Offset::conv(self.track.size) - Offset::conv(self.rect.size)
        }

        /// Set a new grip position
        ///
        /// The input `offset` is clamped between [`Offset::ZERO`] and
        /// [`Self::max_offset`].
        ///
        /// The return value is a tuple of the new offest.
        ///
        /// It is expected that [`Self::set_track`] and [`Self::set_size`] are
        /// called before this method.
        pub fn set_offset(&mut self, cx: &mut EventState, offset: Offset) -> Offset {
            let offset = offset.min(self.max_offset()).max(Offset::ZERO);
            let grip_pos = self.track.pos + offset;
            if grip_pos != self.rect.pos {
                self.rect.pos = grip_pos;
                cx.redraw(self);
            }
            offset
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
                .grab(self.id(), kas::event::GrabMode::Grab)
                .with_icon(CursorIcon::Grabbing)
                .complete(cx);

            let offset = press.coord - self.track.pos - Offset::conv(self.rect.size / 2);
            let offset = offset.clamp(Offset::ZERO, self.max_offset());
            self.press_coord = press.coord - offset;
            offset
        }
    }
}
