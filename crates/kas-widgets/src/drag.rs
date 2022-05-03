// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `DragHandle` control

use std::fmt::Debug;

use kas::event::{CursorIcon, MsgPressFocus, PressSource};
use kas::prelude::*;

impl_scope! {
    /// Draggable Handle
    ///
    /// A `DragHandle` is a draggable object with a given size which is restricted
    /// to a *track* and has an *offset* relative to the start of that track.
    ///
    /// This widget is unusual in several ways:
    ///
    /// 1.  [`Layout::size_rules`] does not request any size; the parent is expected
    ///     to do this.
    /// 2.  [`Layout::set_rect`] sets the *track* within which this handle may move;
    ///     the parent should always call [`DragHandle::set_size_and_offset`]
    ///     afterwards.
    /// 3.  [`Layout::draw`] does nothing. The parent should handle all drawing.
    /// 4.  Optionally, this widget can handle clicks on the track area via
    ///     [`DragHandle::handle_press_on_track`].
    ///
    /// # Messages
    ///
    /// On [`Event::PressStart`], pushes [`MsgPressFocus`].
    ///
    /// On input to change the position, pushes `offset: Offset`. This is a raw
    /// offset relative to the track calculated from input (usually this is
    /// between `Offset::ZERO` and [`Self::max_offset`], but it is not clamped).
    /// The position is not updated by this widget; call [`Self::set_offset`]
    /// to clamp the offset and update the position.
    #[derive(Clone, Debug, Default)]
    #[widget{
        hover_highlight = true;
        cursor_icon = CursorIcon::Grab;
    }]
    pub struct DragHandle {
        #[widget_core]
        core: CoreData,
        // The track is the area within which this DragHandle may move
        track: Rect,
        press_coord: Coord,
    }

    /// This implementation is unusual in that:
    ///
    /// 1.  `size_rules` always returns [`SizeRules::EMPTY`]
    /// 2.  `set_rect` sets the *track* within which this handle may move; the
    ///     parent should call [`DragHandle::set_size_and_offset`] after
    ///     `set_rect` (otherwise the handle's offset will not be updated)
    /// 3.  `draw` does nothing: the parent is expected to do all drawing
    impl Layout for DragHandle {
        fn size_rules(&mut self, _: SizeMgr, _: AxisInfo) -> SizeRules {
            SizeRules::EMPTY
        }

        fn set_rect(&mut self, _: &mut SetRectMgr, rect: Rect, _: AlignHints) {
            self.track = rect;
        }

        fn draw(&mut self, _: DrawMgr) {}
    }

    impl Widget for DragHandle {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::PressStart { source, coord, .. } => {
                    mgr.push_msg(MsgPressFocus);
                    mgr.grab_press_unique(self.id(), source, coord, Some(CursorIcon::Grabbing));

                    // Event delivery implies coord is over the handle.
                    self.press_coord = coord - self.offset();
                    Response::Used
                }
                Event::PressMove { coord, .. } => {
                    mgr.push_msg(coord - self.press_coord);
                    Response::Used
                }
                Event::PressEnd { .. } => Response::Used,
                _ => Response::Unused,
            }
        }
    }
}

impl DragHandle {
    /// Construct
    pub fn new() -> Self {
        DragHandle {
            core: Default::default(),
            track: Default::default(),
            press_coord: Coord::ZERO,
        }
    }

    /// Set a new handle size and offset
    ///
    /// Returns [`TkAction::REDRAW`] if a redraw is required.
    pub fn set_size_and_offset(&mut self, size: Size, offset: Offset) -> TkAction {
        self.core.rect.size = size;
        self.set_offset(offset).1
    }

    /// Get the current track `Rect`
    #[inline]
    pub fn track(&self) -> Rect {
        self.track
    }

    /// Get the current handle offset
    #[inline]
    pub fn offset(&self) -> Offset {
        self.core.rect.pos - self.track.pos
    }

    /// Get the maximum allowed offset
    ///
    /// This depends on size of the handle and the track.
    #[inline]
    pub fn max_offset(&self) -> Offset {
        Offset::conv(self.track.size) - Offset::conv(self.core.rect.size)
    }

    /// Set a new handle offset
    ///
    /// Returns the new offset (after clamping input) and an action: empty if
    /// the handle hasn't moved; `REDRAW` if it has (though this widget is
    /// not directly responsible for drawing, so this may not be accurate).
    pub fn set_offset(&mut self, offset: Offset) -> (Offset, TkAction) {
        let offset = offset.min(self.max_offset()).max(Offset::ZERO);
        let handle_pos = self.track.pos + offset;
        if handle_pos != self.core.rect.pos {
            self.core.rect.pos = handle_pos;
            (offset, TkAction::REDRAW)
        } else {
            (offset, TkAction::empty())
        }
    }

    /// Handle an event on the track itself
    ///
    /// If it is desired to make the handle move when the track area is clicked,
    /// then the parent widget should call this method when receiving
    /// [`Event::PressStart`].
    ///
    /// Returns a raw (unclamped) offset calculated from the press, but does
    /// not move the handle (maybe call [`Self::set_offset`] with the result).
    pub fn handle_press_on_track(
        &mut self,
        mgr: &mut EventMgr,
        source: PressSource,
        coord: Coord,
    ) -> Offset {
        mgr.grab_press_unique(self.id(), source, coord, Some(CursorIcon::Grabbing));

        self.press_coord = self.track.pos + self.core.rect.size / 2;
        coord - self.press_coord
    }
}
