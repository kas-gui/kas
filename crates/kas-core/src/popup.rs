// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Popup root

use crate::dir::Direction;
use crate::event::{ConfigCx, Event, EventCx, IsUsed, Scroll, Unused, Used};
use crate::layout::Align;
use crate::{ChildIndices, Events, Id, Tile, Widget, WindowId};
use kas_macros::{impl_self, widget_index};

#[allow(unused)] use crate::event::EventState;

#[derive(Clone, Debug)]
pub(crate) struct PopupDescriptor {
    /// Widget of type [`Popup`]
    pub id: Id,
    /// Visual parent: what the popup is placed next to
    pub parent: Id,
    /// Direction to place relative to [`Self::parent`]
    pub direction: Direction,
    /// Alignment relative to [`Self::parent`]
    ///
    /// This applies to the opposite axis to the direction; for example if
    /// [`Self::direction`] is [`Direction::Right`] then this is vertical
    /// alignment. In this case, [`Align::TL`] would imply that the top of the
    /// popup would be aligned to the top of the parent.
    pub align: Align,
}

pub(crate) const POPUP_INNER_INDEX: usize = 0;

#[impl_self]
mod Popup {
    /// A popup (e.g. menu or tooltip)
    ///
    /// A pop-up is a box used for things like tool-tips and menus which escapes
    /// the parent's rect. This widget is the root of any popup UI.
    ///
    /// This widget must be excluded from the parent's layout.
    ///
    /// Depending on the platform, the pop-up may be a special window or emulate
    /// this with a layer drawn in an existing window. Both approaches should
    /// exhibit similar behaviour except that the former approach allows the
    /// popup to escape the bounds of the parent window.
    /// NOTE: currently only the emulated approach is implemented.
    ///
    /// A popup receives input data from its parent like any other widget.
    #[widget]
    #[layout(frame!(self.inner).with_style(kas::theme::FrameStyle::Popup))]
    pub struct Popup<W: Widget> {
        core: widget_core!(),
        direction: Direction,
        align: Align,
        /// The inner widget
        #[widget]
        pub inner: W,
        win_id: Option<WindowId>,
    }

    impl Tile for Self {
        fn child_indices(&self) -> ChildIndices {
            // Child is not visible. We handle configuration and updates directly.
            (0..0).into()
        }

        fn find_child_index(&self, id: &Id) -> Option<usize> {
            let index = Some(widget_index!(self.inner));
            if self.win_id.is_none() || id.next_key_after(self.id_ref()) != index {
                return None;
            }
            index
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.new_access_layer(self.id(), true);
        }

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            if self.win_id.is_some() {
                let id = self.make_child_id(widget_index!(self.inner));
                if id.is_valid() {
                    cx.configure(self.inner.as_node(data), id);
                }
            }
        }

        fn update_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            if self.win_id.is_some() {
                cx.update(self.inner.as_node(data));
            }
        }

        fn handle_event(&mut self, _: &mut EventCx, _: &W::Data, event: Event) -> IsUsed {
            match event {
                Event::PopupClosed(_) => {
                    self.win_id = None;
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, _: Scroll) {
            // Scroll of the popup does not affect ancestor nodes
            cx.set_scroll(Scroll::None);
        }
    }

    impl Self {
        /// Construct a popup over a `W: Widget`
        pub fn new(inner: W, direction: Direction, align: Align) -> Self {
            Popup {
                core: Default::default(),
                direction,
                align,
                inner,
                win_id: None,
            }
        }

        /// Get direction
        pub fn direction(&self) -> Direction {
            self.direction
        }

        /// Set direction
        pub fn set_direction(&mut self, direction: Direction) {
            self.direction = direction;
        }

        /// Get alignment
        pub fn alignment(&self) -> Align {
            self.align
        }

        /// Set alignment
        pub fn set_alignment(&mut self, align: Align) {
            self.align = align;
        }

        /// Query whether the popup is open
        pub fn is_open(&self) -> bool {
            self.win_id.is_some()
        }

        /// Open or reposition the popup
        ///
        /// The popup is positioned next to the `parent`'s rect in the specified
        /// direction (if this is not possible, the direction may be reversed).
        ///
        /// The `parent` is marked as depressed (pushed down) while the popup is
        /// open.
        ///
        /// Returns `true` when the popup is newly opened. In this case, the
        /// caller may wish to call [`EventState::next_nav_focus`] next.
        pub fn open(
            &mut self,
            cx: &mut EventCx,
            data: &W::Data,
            parent: Id,
            set_focus: bool,
        ) -> bool {
            let desc = kas::PopupDescriptor {
                id: self.id(),
                parent,
                direction: self.direction,
                align: self.align,
            };

            if let Some(id) = self.win_id {
                cx.reposition_popup(id, desc);
                return false;
            }

            let index = widget_index!(self.inner);
            assert_eq!(index, POPUP_INNER_INDEX);
            let id = self.make_child_id(index);
            cx.configure(self.inner.as_node(data), id);

            self.win_id = Some(cx.add_popup(desc, set_focus));

            true
        }

        /// Close the popup
        ///
        /// Navigation focus will return to whichever widget had focus before
        /// the popup was open.
        pub fn close(&mut self, cx: &mut EventCx) {
            if let Some(id) = self.win_id.take() {
                cx.close_window(id);
            }
        }
    }
}
