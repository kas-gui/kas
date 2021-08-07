// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[allow(unused)]
use super::{GrabMode, Manager, Response}; // for doc-links
use super::{MouseButton, UpdateHandle, VirtualKeyCode};

use crate::geom::{Coord, DVec2, Offset};
use crate::{WidgetId, WindowId};

/// Events addressed to a widget
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// No event
    None,
    /// Widget activation
    ///
    /// For example, clicking a button, toggling a check-box, opening a menu or
    /// requesting char focus (keyboard input).
    ///
    /// This event is triggered by keyboard navigation and accelerator key
    /// bindings, and may be used by a parent widget to activate a child.
    Activate,
    /// (Keyboard) command input
    ///
    /// This represents a control or navigation action, usually from the
    /// keyboard. It is sent to the widget with char focus (if any; see
    /// [`Manager::request_char_focus`]), otherwise to the widget with nav focus
    /// ([`Manager::nav_focus`]), otherwise to a widget registered as a nav
    /// fallback ([`Manager::register_nav_fallback`]).
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
    PressStart {
        source: PressSource,
        start_id: WidgetId,
        coord: Coord,
    },
    /// Movement of mouse or a touch press
    ///
    /// Received only given a [press grab](Manager::request_grab).
    PressMove {
        source: PressSource,
        cur_id: Option<WidgetId>,
        coord: Coord,
        delta: Offset,
    },
    /// End of a click/touch press
    ///
    /// Received only given a [press grab](Manager::request_grab).
    ///
    /// When `end_id == None`, this is a "cancelled press": the end of the press
    /// is outside the application window.
    PressEnd {
        source: PressSource,
        end_id: Option<WidgetId>,
        coord: Coord,
    },
    /// Update from a timer
    ///
    /// This event is received after requesting timed wake-up(s)
    /// (see [`Manager::update_on_timer`]).
    ///
    /// The `u64` payload may be used to identify the corresponding
    /// [`Manager::update_on_timer`] call.
    TimerUpdate(u64),
    /// Update triggerred via an [`UpdateHandle`]
    ///
    /// This event may be received after registering an [`UpdateHandle`] via
    /// [`Manager::update_on_handle`].
    ///
    /// A user-defined payload is passed. Interpretation of this payload is
    /// user-defined and unfortunately not type safe.
    HandleUpdate { handle: UpdateHandle, payload: u64 },
    /// Notification that a new popup has been created
    ///
    /// This is sent to the parent of each open popup when a new popup is
    /// created. This enables parents to close their popups when the new popup
    /// is not a descendant of itself. The `WidgetId` is that of the popup.
    NewPopup(WidgetId),
    /// Notification that a popup has been destroyed
    ///
    /// This is sent to the popup's parent after a popup has been removed.
    /// Since popups may be removed directly by the Manager, the parent should
    /// clean up any associated state here.
    PopupRemoved(WindowId),
    /// Sent when a widget receives keyboard navigation focus
    ///
    /// The widget should reply with [`Response::Focus`] (this is done by
    /// [`Manager::handle_generic`]). It may also be used as an opportunity to
    /// request char focus.
    NavFocus,
}

/// Command input ([`Event::Command`])
///
/// Behaviour differs slightly between char and nav focus. When a widget
/// has char focus, the Space key sends a space character via
/// [`Event::ReceivedCharacter`] while the Return key sends
/// [`Command::Return`]. Without char focus, both Space and Return keys
/// send [`Event::Activate`] to the widget with nav focus (or the fallback).
/// Also, [`Command::Escape`] and [`Command::Tab`] are only sent to widgets
/// with char focus.
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
    /// use for this themselves and return it via [`Response::Unhandled`].
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
    /// This is only sent to widgets with char focus.
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
    /// not character or navigation focus. This is limited to: `Deselect`,
    /// `Copy`.
    pub fn as_select(self) -> Option<Self> {
        use Command::*;
        Some(match self {
            Deselect | Escape => Deselect,
            Cut | Copy => Copy,
            _ => return None,
        })
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
