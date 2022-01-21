// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `DragHandle` control

use std::fmt::Debug;

use kas::event::{self, PressSource};
use kas::prelude::*;

widget! {
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
    #[derive(Clone, Debug, Default)]
    #[widget{
        hover_highlight = true;
        cursor_icon = event::CursorIcon::Grab;
    }]
    pub struct DragHandle {
        #[widget_core]
        core: CoreData,
        // The track is the area within which this DragHandle may move
        track: Rect,
        press_source: Option<event::PressSource>,
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

    impl event::Handler for DragHandle {
        type Msg = Offset;

        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response<Self::Msg> {
            match event {
                Event::PressStart { source, coord, .. } => {
                    if !self.grab_press(mgr, source, coord) {
                        return Response::Used;
                    }

                    // Event delivery implies coord is over the handle.
                    self.press_coord = coord - self.offset();
                    Response::Used
                }
                Event::PressMove { source, coord, .. } if Some(source) == self.press_source => {
                    let offset = coord - self.press_coord;
                    let (offset, action) = self.set_offset(offset);
                    if action.is_empty() {
                        Response::Used
                    } else {
                        mgr.send_action(action);
                        Response::Msg(offset)
                    }
                }
                Event::PressEnd { source, .. } if Some(source) == self.press_source => {
                    self.press_source = None;
                    Response::Used
                }
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
            press_source: None,
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
        Offset::from(self.track.size) - Offset::from(self.core.rect.size)
    }

    /// Set a new handle offset
    ///
    /// Returns the new offset (after clamping input) and an action: empty if
    /// the handle hasn't moved; `REDRAW` if it has (though this widget is
    /// not directly responsible for drawing, so this may not be accurate).
    pub fn set_offset(&mut self, offset: Offset) -> (Offset, TkAction) {
        let offset = offset.clamp(Offset::ZERO, self.max_offset());
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
    /// This method moves the handle immediately and returns the new offset.
    pub fn handle_press_on_track(
        &mut self,
        mgr: &mut EventMgr,
        source: PressSource,
        coord: Coord,
    ) -> Offset {
        if !self.grab_press(mgr, source, coord) {
            return self.offset();
        }

        self.press_coord = self.track.pos + self.core.rect.size / 2;

        // Since the press is not on the handle, we move the bar immediately.
        let (offset, action) = self.set_offset(coord - self.press_coord);
        debug_assert!(action == TkAction::REDRAW);
        mgr.send_action(action);
        offset
    }

    fn grab_press(&mut self, mgr: &mut EventMgr, source: PressSource, coord: Coord) -> bool {
        let cur = Some(event::CursorIcon::Grabbing);
        if mgr.request_grab(self.id(), source, coord, event::GrabMode::Grab, cur) {
            // Interacting with a scrollbar with multiple presses
            // does not make sense. Any other gets aborted.
            self.press_source = Some(source);
            true
        } else {
            false
        }
    }
}
