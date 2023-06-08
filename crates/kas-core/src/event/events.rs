// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[allow(unused)]
use super::{EventMgr, EventState, GrabMode, Response}; // for doc-links
use super::{Press, UpdateId, VirtualKeyCode};
use crate::geom::{DVec2, Offset};
#[allow(unused)] use crate::Widget;
use crate::{dir::Direction, WidgetId, WindowId};

/// Events addressed to a widget
///
/// Note regarding disabled widgets: [`Event::Update`], [`Event::PopupRemoved`]
/// and `Lost..` events are received regardless of status; other events are not
/// received by disabled widgets. See [`Event::pass_when_disabled`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// No event
    None,
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
    Command(Command),
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
    /// This event is sent for certain types of grab ([`Press::grab`]),
    /// enabling two-finger scale/rotate gestures as well as translation.
    ///
    /// Mouse-grabs generate translation (`delta` component) only. Touch grabs
    /// optionally also generate rotation and scaling components, depending on
    /// the [`GrabMode`].
    ///
    /// In general, a point `p` on the screen should be transformed as follows:
    /// ```
    /// # use kas_core::cast::{Cast, CastFloat};
    /// # use kas_core::geom::{Coord, DVec2};
    /// # let (alpha, delta) = (DVec2::ZERO, DVec2::ZERO);
    /// let mut p = Coord::ZERO; // or whatever
    /// p = (alpha.complex_mul(p.cast()) + delta).cast_nearest();
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
    /// Call [`Press::grab`] in order to "grab" corresponding motion
    /// and release events.
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
    PressStart { press: Press },
    /// Movement of mouse or a touch press
    ///
    /// This event is sent in exactly two cases, in this order:
    ///
    /// 1.  Given a grab ([`Press::grab`]), motion events for the
    ///     grabbed mouse pointer or touched finger will be sent.
    /// 2.  When a pop-up layer is active ([`EventMgr::add_popup`]), the owner
    ///     of the top-most layer will receive this event. If the event is not
    ///     used, then the pop-up will be closed and the event sent again.
    ///
    /// If `cur_id` is `None`, no widget was found at the coordinate (either
    /// outside the window or [`crate::Layout::find_id`] failed).
    PressMove { press: Press, delta: Offset },
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
    /// 1.  Given a grab ([`Press::grab`]), release/cancel events
    ///     for the same mouse button or touched finger will be sent.
    ///
    /// If `cur_id` is `None`, no widget was found at the coordinate (either
    /// outside the window or [`crate::Layout::find_id`] failed).
    PressEnd { press: Press, success: bool },
    /// Update from a timer
    ///
    /// This event is received after requesting timed wake-up(s)
    /// (see [`EventState::request_timer_update`]).
    ///
    /// The `u64` payload is copied from [`EventState::request_timer_update`].
    TimerUpdate(u64),
    /// Update triggerred via an [`UpdateId`]
    ///
    /// When [`EventMgr::update_all`] is called, this event is broadcast to all
    /// widgets via depth-first traversal of the widget tree. As such,
    /// [`Widget::steal_event`] and [`Widget::handle_unused`] are not called
    /// with this `Event`, nor are [`Widget::handle_message`] or
    /// [`Widget::handle_scroll`] called after a widget receives this `Event`.
    Update { id: UpdateId, payload: u64 },
    /// Notification that a popup has been destroyed
    ///
    /// This is sent to the popup's parent after a popup has been removed.
    /// Since popups may be removed directly by the EventMgr, the parent should
    /// clean up any associated state here.
    PopupRemoved(WindowId),
    /// Sent when a widget receives (keyboard) navigation focus
    ///
    /// When the payload, `key_focus`, is true when the focus was triggered by
    /// the keyboard, not the mouse or a touch event.
    /// This event may be used e.g. to request char focus or to
    /// steal focus from a child.
    ///
    /// Note: when `NavFocus(true)` is sent to a widget, the sender
    /// automatically sets `Scroll::Rect(widget.rect())` to
    /// [`EventMgr::set_scroll`] and considers the event used.
    NavFocus(bool),
    /// Sent when a widget becomes the mouse hover target
    MouseHover,
    /// Sent when a widget loses navigation focus
    LostNavFocus,
    /// Sent when a widget is no longer the mouse hover target
    LostMouseHover,
    /// Widget lost keyboard input focus
    ///
    /// This focus is gained through the widget calling [`EventState::request_char_focus`].
    LostCharFocus,
    /// Widget lost selection focus
    ///
    /// This focus is gained through the widget calling [`EventState::request_sel_focus`]
    /// or [`EventState::request_char_focus`].
    ///
    /// In the case the widget also had character focus, [`Event::LostCharFocus`] is
    /// received first.
    LostSelFocus,
}

impl std::ops::Add<Offset> for Event {
    type Output = Self;

    #[inline]
    fn add(mut self, offset: Offset) -> Event {
        self += offset;
        self
    }
}

impl std::ops::AddAssign<Offset> for Event {
    fn add_assign(&mut self, offset: Offset) {
        match self {
            Event::PressStart { ref mut press, .. } => {
                press.coord += offset;
            }
            Event::PressMove { ref mut press, .. } => {
                press.coord += offset;
            }
            Event::PressEnd { ref mut press, .. } => {
                press.coord += offset;
            }
            _ => (),
        }
    }
}

impl Event {
    /// Call `f` on any "activation" event
    ///
    /// Activation is considered:
    ///
    /// -   Mouse click and release on the same widget
    /// -   Touchscreen press and release on the same widget
    /// -   `Event::Command(cmd, _)` where [`cmd.is_activate()`](Command::is_activate)
    pub fn on_activate<F: FnOnce(&mut EventMgr) -> Response>(
        self,
        mgr: &mut EventMgr,
        id: WidgetId,
        f: F,
    ) -> Response {
        match self {
            Event::Command(cmd) if cmd.is_activate() => f(mgr),
            Event::PressStart { press, .. } if press.is_primary() => press.grab(id).with_mgr(mgr),
            Event::PressEnd { press, success } => {
                if success && id == press.id {
                    f(mgr)
                } else {
                    Response::Used
                }
            }
            _ => Response::Unused,
        }
    }

    /// Pass to disabled widgets?
    ///
    /// Disabled status should disable input handling but not prevent other
    /// notifications.
    pub fn pass_when_disabled(&self) -> bool {
        use Event::*;
        match self {
            None | Command(_) => false,
            ReceivedCharacter(_) | Scroll(_) | Pan { .. } => false,
            PressStart { .. } | PressMove { .. } | PressEnd { .. } => false,
            TimerUpdate(_) | Update { .. } | PopupRemoved(_) => true,
            NavFocus(_) | MouseHover => false,
            LostNavFocus | LostMouseHover | LostCharFocus | LostSelFocus => true,
        }
    }

    /// Can the event be received by [`Widget::handle_unused`]?
    ///
    /// Events which may be sent to the widget under the mouse or to the
    /// keyboard navigation target may be acted on by an ancestor if unused.
    /// Other events may not be; e.g. [`Event::PressMove`] and
    /// [`Event::PressEnd`] are only received by the widget requesting them
    /// while [`Event::LostCharFocus`] (and similar events) are only sent to a
    /// specific widget.
    pub fn is_reusable(&self) -> bool {
        use Event::*;
        match self {
            None => false,
            Command(_) | ReceivedCharacter(_) | Scroll(_) | Pan { .. } | PressStart { .. } => true,
            PressMove { .. } | PressEnd { .. } => false,
            TimerUpdate(_) | Update { .. } | PopupRemoved(_) => false,
            NavFocus(_) | MouseHover | LostNavFocus | LostMouseHover => false,
            LostCharFocus | LostSelFocus => false,
        }
    }
}

/// Command input ([`Event::Command`])
///
/// `Command` events are mostly produced as a result of OS-specific keyboard
/// bindings; for example,  [`Command::Copy`] is produced by pressing
/// <kbd>Command+C</kbd> on MacOS or <kbd>Ctrl+C</kbd> on other platforms.
/// See [`crate::event::config::Shortcuts`] for more on these bindings.
///
/// A `Command` event does not necessarily come from keyboard input; for example
/// some menu widgets send [`Command::Activate`] to trigger an entry as a result
/// of mouse input.
///
/// *Most* `Command` entries represent an action (such as `Copy` or `FindNext`)
/// but some represent an important key whose action may be context-dependent
/// (e.g. `Escape`, `Space`).
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
    /// Programmatic activation
    ///
    /// A synthetic event to activate widgets. Consider matching
    /// [`Command::is_activate`] or using using [`Event::on_activate`]
    /// instead for generally applicable activation.
    Activate,
    /// Return / enter key
    ///
    /// This may insert a line-break or may activate something.
    Enter,
    /// Space bar key
    Space,
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
    /// Debug
    Debug,
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
            Space => Command::Space,
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
            Return => Command::Enter,
            NavigateForward => Command::NavNext,
            NavigateBackward => Command::NavPrev,
            NumpadEnter => Command::Enter,
            Tab => Command::Tab,
            Cut => Command::Cut,
            Copy => Command::Copy,
            Paste => Command::Paste,
            _ => return None,
        })
    }

    /// True for "activation" commands
    ///
    /// This matches:
    ///
    /// -   [`Self::Activate`] — programmatic activation
    /// -   [`Self::Enter`] —  <kbd>Enter</kbd> and <kbd>Return</kbd> keys
    /// -   [`Self::Space`] — <kbd>Space</kbd> key
    pub fn is_activate(self) -> bool {
        use Command::*;
        matches!(self, Activate | Enter | Space)
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

/// Type used by [`Event::Scroll`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Scroll a given number of lines
    LineDelta(f32, f32),
    /// Scroll a given number of pixels
    PixelDelta(Offset),
}
