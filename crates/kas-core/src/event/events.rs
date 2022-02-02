// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[allow(unused)]
use super::{EventMgr, EventState, GrabMode, Response, SendEvent}; // for doc-links
use super::{MouseButton, UpdateHandle, VirtualKeyCode};

use crate::geom::{Coord, DVec2, Offset};
use crate::{dir::Direction, WidgetId, WindowId};

/// Events addressed to a widget
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// No event
    None,
    /// Widget activation
    ///
    /// "Activate" is triggered e.g. by clicking on a control or using keyboard
    /// navigation to trigger a control. It should then perform the widget's
    /// primary function, e.g. a button or menu action, toggle a checkbox, etc.
    Activate,
    /// (Keyboard) command input
    ///
    /// This represents a control or navigation action, usually from the
    /// keyboard. It is sent to whichever widget is "most appropriate", then
    /// potentially to the "next most appropriate" target if the first returns
    /// [`Response::Unused`], until handled or no more appropriate targets
    /// are available (the exact logic is encoded in `EventMgr::start_key_event`).
    ///
    /// In some cases keys are remapped, e.g. a widget with selection focus but
    /// not character or navigation focus may receive [`Command::Deselect`]
    /// when the <kbd>Esc</kbd> key is pressed.
    ///
    /// The state of the shift key is included (true = pressed).
    Command(Command, bool),
    /// Widget lost keyboard input focus
    LostCharFocus,
    /// Widget lost selection focus
    ///
    /// Selection focus implies character focus, so this event implies that the
    /// widget has already received [`Event::LostCharFocus`].
    LostSelFocus,
    /// Widget receives a character of text input
    ///
    /// This is only received by a widget with character focus (see
    /// [`EventState::request_char_focus`]). There is no overlap with
    /// [`Event::Command`]: key presses result in at most one of these events
    /// being sent to a widget.
    ReceivedCharacter(char),
    /// A mouse or touchpad scroll event
    Scroll(ScrollDelta),
    /// A mouse or touch-screen move/zoom/rotate event
    ///
    /// Mouse-grabs generate translation (`delta` component) only. Touch grabs
    /// optionally also generate rotation and scaling components, depending on
    /// the [`GrabMode`].
    ///
    /// In general, a point `p` on the screen should be transformed as follows:
    /// ```
    /// # use kas_core::geom::{Coord, DVec2};
    /// # let (alpha, delta) = (DVec2::ZERO, DVec2::ZERO);
    /// # let mut p = Coord::ZERO;
    /// // Works for Coord type; for DVec2 type-conversions are unnecessary:
    /// p = (alpha.complex_mul(p.into()) + delta).into();
    /// ```
    ///
    /// When it is known that there is no rotational component, one can use a
    /// simpler transformation: `alpha.0 * p + delta`. When there is also no
    /// scaling component, we just have a translation: `p + delta`.
    /// Note however that if events are generated with rotation and/or scaling
    /// components, these simplifications are invalid.
    ///
    /// Two such transforms may be combined as follows:
    /// ```
    /// # use kas_core::geom::DVec2;
    /// # let (alpha1, delta1) = (DVec2::ZERO, DVec2::ZERO);
    /// # let (alpha2, delta2) = (DVec2::ZERO, DVec2::ZERO);
    /// let alpha = alpha2.complex_mul(alpha1);
    /// let delta = alpha2.complex_mul(delta1) + delta2;
    /// ```
    /// If instead one uses a transform to map screen-space to world-space,
    /// this transform should be adjusted as follows:
    /// ```
    /// # use kas_core::geom::DVec2;
    /// # let (alpha, delta) = (DVec2::ZERO, DVec2::ZERO);
    /// # let (mut world_alpha, mut world_delta) = (DVec2::ZERO, DVec2::ZERO);
    /// world_alpha = world_alpha.complex_div(alpha.into());
    /// world_delta = world_delta - world_alpha.complex_mul(delta.into());
    /// ```
    ///
    /// Those familiar with complex numbers may recognise that
    /// `alpha = a * e^{i*t}` where `a` is the scale component and `t` is the
    /// angle of rotation. Calculate these components as follows:
    /// ```
    /// # use kas_core::geom::DVec2;
    /// # let alpha = DVec2::ZERO;
    /// let a = (alpha.0 * alpha.0 + alpha.1 * alpha.1).sqrt();
    /// let t = (alpha.1).atan2(alpha.0);
    /// ```
    Pan {
        /// Rotation and scale component
        alpha: DVec2,
        /// Translation component
        delta: DVec2,
    },
    /// A mouse button was pressed or touch event started
    ///
    /// This event is sent in exactly two cases, in this order:
    ///
    /// 1.  When a pop-up layer is active ([`EventMgr::add_popup`]), the owner
    ///     of the top-most layer will receive this event. If the event is not
    ///     used, then the pop-up will be closed and the event sent again.
    /// 2.  If a widget is found under the mouse when pressed or where a touch
    ///     event starts, this event is sent to the widget.
    ///
    /// If `start_id` is `None`, then no widget was found at the coordinate and
    /// the event will only be delivered to pop-up layer owners.
    PressStart {
        source: PressSource,
        start_id: Option<WidgetId>,
        coord: Coord,
    },
    /// Movement of mouse or a touch press
    ///
    /// This event is sent in exactly two cases, in this order:
    ///
    /// 1.  Given a grab ([`EventMgr::grab_press`]), motion events for the
    ///     grabbed mouse pointer or touched finger will be sent.
    /// 2.  When a pop-up layer is active ([`EventMgr::add_popup`]), the owner
    ///     of the top-most layer will receive this event. If the event is not
    ///     used, then the pop-up will be closed and the event sent again.
    ///
    /// If `cur_id` is `None`, no widget was found at the coordinate (either
    /// outside the window or [`crate::Layout::find_id`] failed).
    PressMove {
        source: PressSource,
        cur_id: Option<WidgetId>,
        coord: Coord,
        delta: Offset,
    },
    /// End of a click/touch press
    ///
    /// If `success`, this is a button-release or touch finish; otherwise this
    /// is a cancelled/interrupted grab. "Activation events" (e.g. clicking of a
    /// button or menu item) should only happen on `success`. "Movement events"
    /// such as panning, moving a slider or opening a menu should not be undone
    /// when cancelling: the panned item or slider should be released as is, or
    /// the menu should remain open.
    ///
    /// This event is sent in exactly one case:
    ///
    /// 1.  Given a grab ([`EventMgr::grab_press`]), release/cancel events
    ///     for the same mouse button or touched finger will be sent.
    ///
    /// If `cur_id` is `None`, no widget was found at the coordinate (either
    /// outside the window or [`crate::Layout::find_id`] failed).
    PressEnd {
        source: PressSource,
        end_id: Option<WidgetId>,
        coord: Coord,
        success: bool,
    },
    /// Update from a timer
    ///
    /// This event is received after requesting timed wake-up(s)
    /// (see [`EventState::update_on_timer`]).
    ///
    /// The `u64` payload may be used to identify the corresponding
    /// [`EventState::update_on_timer`] call.
    TimerUpdate(u64),
    /// Update triggerred via an [`UpdateHandle`]
    ///
    /// This event may be received after registering an [`UpdateHandle`] via
    /// [`EventState::update_on_handle`].
    ///
    /// A user-defined payload is passed. Interpretation of this payload is
    /// user-defined and unfortunately not type safe.
    HandleUpdate { handle: UpdateHandle, payload: u64 },
    /// Notification that a popup has been destroyed
    ///
    /// This is sent to the popup's parent after a popup has been removed.
    /// Since popups may be removed directly by the EventMgr, the parent should
    /// clean up any associated state here.
    PopupRemoved(WindowId),
    /// Sent when a widget receives keyboard navigation focus
    ///
    /// This event may be used to react (e.g. by requesting char focus) or to
    /// steal focus from a child.
    ///
    /// When the payload, `key_focus`, is true when the focus was triggered by
    /// the keyboard, not the mouse or a touch event. When true, the widget
    /// should reply with [`Response::Focus`] in order to ensure visibility.
    /// This should not be done when `key_focus` is false to avoid moving
    /// widgets during interaction via mouse or finger.
    ///
    /// Widgets using [`EventMgr::handle_generic`] should note that this event
    /// is trapped and responded to (in line with the above recommendation).
    /// If the widget needs to react to `NavFocus`, the event must be matched
    /// *before* calling `handle_generic`, which might require a custom
    /// implementation of [`SendEvent`].
    NavFocus(bool),
}

/// Command input ([`Event::Command`])
///
/// The exact command sent depends on the type of focus a widget has.
///
/// Handling may depend on the state of the Shift key.
///
/// The exact mapping between the keyboard and these commands is OS-specific.
/// In the future it should be customisable (see `shortcuts` module).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Command {
    /// Escape key
    ///
    /// Each press of this key should somehow relax control. It is expected that
    /// widgets receiving this key repeatedly eventually (soon) have no more
    /// use for this themselves and return it via [`Response::Unused`].
    ///
    /// This is in some cases remapped to [`Command::Deselect`].
    Escape,
    /// Return / enter key
    ///
    /// This may insert a line-break or may activate something.
    ///
    /// This is only sent to widgets with char focus.
    /// In other cases a widget may receive [`Event::Activate`].
    Return,
    /// Tab key
    ///
    /// This key is used to insert (horizontal) tabulators as well as to
    /// navigate focus (in reverse when combined with Shift).
    ///
    /// This is usually not sent to widgets but instead used for navigation.
    Tab,

    /// Move view up without affecting selection
    ViewUp,
    /// Move view down without affecting selection
    ViewDown,

    /// Move left
    Left,
    /// Move right
    Right,
    /// Move up
    Up,
    /// Move down
    Down,
    /// Move left one word
    WordLeft,
    /// Move right one word
    WordRight,
    /// Move to start (of the line)
    Home,
    /// Move to end (of the line)
    End,
    /// Move to start of the document
    DocHome,
    /// Move to end of the document
    DocEnd,
    /// Move up a page
    PageUp,
    /// Move down a page
    PageDown,

    /// Capture a screenshot
    Snapshot,
    /// Lock output of screen
    ScrollLock,
    /// Pause key
    Pause,
    /// Insert key
    Insert,

    /// Delete forwards
    Delete,
    /// Delete backwards (Backspace key)
    DelBack,
    /// Delete forwards one word
    DelWord,
    /// Delete backwards one word
    DelWordBack,

    /// Clear any selections
    Deselect,
    /// Select all contents
    SelectAll,

    /// Find (start)
    Find,
    /// Find and replace (start)
    FindReplace,
    /// Find next
    FindNext,
    /// Find previous
    FindPrev,

    /// Make text bold
    Bold,
    /// Make text italic
    Italic,
    /// Underline text
    Underline,
    /// Insert a link
    Link,

    /// Copy to clipboard and clear
    Cut,
    /// Copy to clipboard
    Copy,
    /// Copy from clipboard
    Paste,
    /// Undo the last action
    Undo,
    /// Redo the last undone action
    Redo,

    /// New document
    New,
    /// Open document
    Open,
    /// Save document
    Save,
    /// Print document
    Print,

    /// Navigate forwards one page/item
    NavNext,
    /// Navigate backwards one page/item
    NavPrev,
    /// Navigate to the parent item
    ///
    /// May be used to browse "up" to a parent directory.
    NavParent,
    /// Navigate "down"
    ///
    /// This is an opposite to `NavParent`, and will mostly not be used.
    NavDown,

    /// Open a new tab
    TabNew,
    /// Navigate to next tab
    TabNext,
    /// Navigate to previous tab
    TabPrev,

    /// Show help
    Help,
    /// Rename
    Rename,
    /// Refresh
    Refresh,
    /// Spell-check tool
    Spelling,
    /// Open the menu / activate the menubar
    Menu,
    /// Make view fullscreen
    Fullscreen,

    /// Close window/tab/popup
    Close,
    /// Exit program (e.g. Ctrl+Q)
    Exit,
}

impl Command {
    /// Try constructing from a [`VirtualKeyCode`]
    pub fn new(vkey: VirtualKeyCode) -> Option<Self> {
        use VirtualKeyCode::*;
        Some(match vkey {
            Escape => Command::Escape,
            Snapshot => Command::Snapshot,
            Scroll => Command::ScrollLock,
            Pause => Command::Pause,
            Insert => Command::Insert,
            Home => Command::Home,
            Delete => Command::Delete,
            End => Command::End,
            PageDown => Command::PageDown,
            PageUp => Command::PageUp,
            Left => Command::Left,
            Up => Command::Up,
            Right => Command::Right,
            Down => Command::Down,
            Back => Command::DelBack,
            Return => Command::Return,
            NavigateForward => Command::NavNext,
            NavigateBackward => Command::NavPrev,
            NumpadEnter => Command::Return,
            Tab => Command::Tab,
            Cut => Command::Cut,
            Copy => Command::Copy,
            Paste => Command::Paste,
            _ => return None,
        })
    }

    /// Convert to selection-focus command
    ///
    /// Certain limited commands may be sent to widgets with selection focus but
    /// not character or navigation focus.
    pub fn suitable_for_sel_focus(self) -> bool {
        use Command::*;
        matches!(self, Escape | Cut | Copy | Deselect)
    }

    /// Convert arrow keys to a direction
    pub fn as_direction(self) -> Option<Direction> {
        match self {
            Command::Left => Some(Direction::Left),
            Command::Right => Some(Direction::Right),
            Command::Up => Some(Direction::Up),
            Command::Down => Some(Direction::Down),
            _ => None,
        }
    }
}

/// Source of `EventChild::Press`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PressSource {
    /// A mouse click
    ///
    /// Arguments: `button, repeats`.
    ///
    /// The `repeats` argument is used for double-clicks and similar. For a
    /// single-click, `repeats == 1`; for a double-click it is 2, for a
    /// triple-click it is 3, and so on (without upper limit).
    ///
    /// For `PressMove` and `PressEnd` events delivered with a mouse-grab,
    /// both arguments are copied from the initiating `PressStart` event.
    /// For a `PressMove` delivered without a grab (only possible with pop-ups)
    /// a fake `button` value is used and `repeats == 0`.
    Mouse(MouseButton, u32),
    /// A touch event (with given `id`)
    Touch(u64),
}

impl PressSource {
    /// Returns true if this represents the left mouse button or a touch event
    #[inline]
    pub fn is_primary(self) -> bool {
        match self {
            PressSource::Mouse(button, _) => button == MouseButton::Left,
            PressSource::Touch(_) => true,
        }
    }

    /// Returns true if this represents a touch event
    #[inline]
    pub fn is_touch(self) -> bool {
        matches!(self, PressSource::Touch(_))
    }

    /// The `repetitions` value
    ///
    /// This is 1 for a single-click and all touch events, 2 for a double-click,
    /// 3 for a triple-click, etc. For `PressMove` without a grab this is 0.
    #[inline]
    pub fn repetitions(self) -> u32 {
        match self {
            PressSource::Mouse(_, repetitions) => repetitions,
            PressSource::Touch(_) => 1,
        }
    }
}

/// Type used by [`Event::Scroll`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Scroll a given number of lines
    LineDelta(f32, f32),
    /// Scroll a given number of pixels
    PixelDelta(Offset),
}
