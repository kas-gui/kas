// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

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
    /// Control / Navigation key input
    ///
    /// This represents a "control" actions, usually triggered by a key, and are
    /// received with char focus ([`Manager::request_char_focus`]), nav focus
    /// ([`Manager::nav_focus`]) or as a nav
    /// fallback ([`Manager::register_nav_fallback`]).
    ///
    /// Behaviour differs slightly between char and nav focus. When a widget
    /// has char focus, the Space key sends a space character via
    /// [`Event::ReceivedCharacter`] while the Return key sends
    /// [`ControlKey::Return`]. Without char focus, both Space and Return keys
    /// send [`Event::Activate`] to the widget with nav focus (if any, otherwise
    /// to the nav fallback, if any).
    Control(ControlKey),
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
    /// # use kas::geom::{Coord, DVec2};
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
    /// # use kas::geom::DVec2;
    /// # let (alpha1, delta1) = (DVec2::ZERO, DVec2::ZERO);
    /// # let (alpha2, delta2) = (DVec2::ZERO, DVec2::ZERO);
    /// let alpha = alpha2.complex_mul(alpha1);
    /// let delta = alpha2.complex_mul(delta1) + delta2;
    /// ```
    /// If instead one uses a transform to map screen-space to world-space,
    /// this transform should be adjusted as follows:
    /// ```
    /// # use kas::geom::DVec2;
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
    /// # use kas::geom::DVec2;
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
    TimerUpdate,
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
    /// The widget should reply with [`Response::Focus`].
    NavFocus,
}

/// Control / Navigation key ([`Event::Control`])
///
/// These codes are generated from keyboard events when a widget has char or
/// nav focus. The codes generated differ slightly depending on which focus
/// dominates; see notes on [`ControlKey::Return`] and [`ControlKey::Tab`].
///
/// In some cases, a widget's response will depend on the state of modifier
/// keys. This state can be read via the [`Manager::modifiers`] method.
///
/// The purpose of this enum (instead of sending the active widget a
/// [`VirtualKeyCode`]) is consistent behaviour: these "navigation keys" will
/// always be sent to the widget highlighted for keyboard navigation, if active,
/// while alpha-numeric keys will always be available for accelerator keys
/// (when a character input grab is not present). Additionally, this allows
/// uniform behaviour with regards to num-pad keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControlKey {
    /// Escape key
    ///
    /// Each press of this key should somehow relax control. It is expected that
    /// widgets receiving this key repeatedly eventually (soon) have no more
    /// use for this themselves and return it via [`Response::Unhandled`].
    Escape,
    /// Line break (return / enter key)
    ///
    /// Note: this is generated *only* when a widget has char focus (see
    /// [`Manager::request_char_focus`]), otherwise the Return key is mapped to
    /// [`Event::Activate`] and sent to the widget with nav focus.
    Return,
    /// (Horizontal) tabulation
    ///
    /// Note: this is generated *only* when a widget has char focus (see
    /// [`Manager::request_char_focus`]), otherwise the Tab key adjusts nav
    /// focus.
    Tab,

    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Home key
    Home,
    /// End key
    End,
    /// Page up
    PageUp,
    /// Page down
    PageDown,

    /// "Screenshot" key
    Snapshot,
    /// Scroll lock key
    ScrollLock,
    /// Pause key
    Pause,
    /// Insert key
    Insert,
    /// Delete forwards
    Delete,
    /// Delete backwards
    Backspace,

    /// Clear any selections
    Deselect,
    /// Select all contents
    SelectAll,
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

    /// Navigate backwards one page/item
    Backward,
    /// Navigate forwards one page/item
    Forward,
}

impl ControlKey {
    /// Try constructing from a [`VirtualKeyCode`]
    pub fn new(vkey: VirtualKeyCode) -> Option<Self> {
        use ControlKey as CK;
        use VirtualKeyCode::*;
        Some(match vkey {
            Escape => CK::Escape,
            Snapshot => CK::Snapshot,
            Scroll => CK::ScrollLock,
            Pause => CK::Pause,
            Insert => CK::Insert,
            Home => CK::Home,
            Delete => CK::Delete,
            End => CK::End,
            PageDown => CK::PageDown,
            PageUp => CK::PageUp,
            Left => CK::Left,
            Up => CK::Up,
            Right => CK::Right,
            Down => CK::Down,
            Back => CK::Backspace,
            Return => CK::Return,
            NavigateForward => CK::Forward,
            NavigateBackward => CK::Backward,
            NumpadEnter => CK::Return,
            Tab => CK::Tab,
            Cut => CK::Cut,
            Copy => CK::Copy,
            Paste => CK::Paste,
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
        match self {
            PressSource::Touch(_) => true,
            _ => false,
        }
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
