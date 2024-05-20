// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{EventCx, IsUsed, Unused, Used};
#[allow(unused)] use super::{EventState, GrabMode};
use super::{Key, KeyEvent, NamedKey, PhysicalKey, Press};
use crate::geom::{DVec2, Offset};
use crate::{dir::Direction, Id, WindowId};
#[allow(unused)] use crate::{Events, Popup};

/// Events addressed to a widget
///
/// Note that a few events are received by disabled widgets; see
/// [`Event::pass_when_disabled`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Command input
    ///
    /// A generic "command". The source is often but not always a key press.
    /// In many cases (but not all) the target widget has navigation focus.
    ///
    /// A [`PhysicalKey`] is attached when the command is caused by a key press.
    /// The recipient may use this to call [`EventState::depress_with_key`].
    ///
    /// If a widget has keyboard input focus (see
    /// [`EventState::request_key_focus`]) it will instead receive
    /// [`Event::Key`] for key presses (but may still receive `Event::Command`
    /// from other sources).
    Command(Command, Option<PhysicalKey>),
    /// Keyboard input: `event, is_synthetic`
    ///
    /// This is only received by a widget with character focus (see
    /// [`EventState::request_key_focus`]).
    ///
    /// On some platforms, synthetic key events are generated when a window
    /// gains or loses focus with a key held (see documentation of
    /// [`winit::event::WindowEvent::KeyboardInput`]). This is indicated by the
    /// second parameter, `is_synthetic`. Unless you need to track key states
    /// it is advised only to match `Event::Key(event, false)`.
    ///
    /// Some key presses can be mapped to a [`Command`]. To do this (normally
    /// only when `event.state == ElementState::Pressed && !is_synthetic`), use
    /// `cx.config().shortcuts(|s| s.try_match(cx.modifiers(), &event.logical_key)`
    /// or (omitting shortcut matching) `Command::new(event.logical_key)`.
    /// Note that if the handler returns [`Unused`] the widget might
    /// then receive [`Event::Command`] for the same key press, but this is not
    /// guaranteed (behaviour may change in future versions).
    ///
    /// For standard text input, simply consume `event.text` when
    /// `event.state == ElementState::Pressed && !is_synthetic`.
    /// NOTE: unlike Winit, we force `text = None` for control chars and when
    /// <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Super</kbd> modifier keys are
    /// pressed. This is subject to change.
    Key(KeyEvent, bool),
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
    /// Movement of mouse cursor without press
    ///
    /// This event is sent only when:
    ///
    /// 1.  No [`Press::grab`] is active
    /// 2.  A [`Popup`] is open
    ///
    /// The parent (or an ancestor) of a [`Popup`] should handle or explicitly
    /// ignore this event.
    CursorMove { press: Press },
    /// A mouse button was pressed or touch event started
    ///
    /// Call [`Press::grab`] in order to "grab" corresponding motion
    /// and release events.
    ///
    /// This event is sent in exactly two cases, in this order:
    ///
    /// 1.  When a [`Popup`] is open. A [`Popup`] will close itself if
    ///     `press.id` is not a descendant of itself, but will still return
    ///     [`Unused`]. The parent (or an ancestor) of the
    ///     [`Popup`] should handle this event.
    /// 2.  If a widget is found under the mouse when pressed or where a touch
    ///     event starts, this event is sent to the widget.
    ///
    /// If `start_id` is `None`, then no widget was found at the coordinate and
    /// the event will only be delivered to pop-up layer owners.
    PressStart { press: Press },
    /// Movement of mouse or a touch press
    ///
    /// This event is only sent when a ([`Press::grab`]) is active.
    /// Motion events for the grabbed mouse pointer or touched finger are sent.
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
    /// This event is only sent when a ([`Press::grab`]) is active.
    /// Release/cancel events for the same mouse button or touched finger are
    /// sent.
    ///
    /// If `cur_id` is `None`, no widget was found at the coordinate (either
    /// outside the window or [`crate::Layout::find_id`] failed).
    PressEnd { press: Press, success: bool },
    /// Update from a timer
    ///
    /// This event is received after requesting timed wake-up(s)
    /// (see [`EventState::request_timer`]).
    ///
    /// The `u64` payload is copied from [`EventState::request_timer`].
    Timer(u64),
    /// Notification that a popup has been closed
    ///
    /// This is sent to the popup when closed.
    /// Since popups may be removed directly by the [`EventCx`], the parent should
    /// clean up any associated state here.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    PopupClosed(WindowId),
    /// Notification that a widget has gained navigation focus
    ///
    /// Navigation focus implies that the widget is highlighted and will be the
    /// primary target of [`Event::Command`], and is thus able to receive basic
    /// keyboard input (e.g. arrow keys). To receive full keyboard input
    /// ([`Event::Key`]), call [`EventState::request_key_focus`].
    ///
    /// With [`FocusSource::Pointer`] the widget should already have received
    /// [`Event::PressStart`].
    ///
    /// With [`FocusSource::Key`], [`EventCx::set_scroll`] is
    /// called automatically (to ensure that the widget is visible) and the
    /// response will be forced to [`Used`].
    NavFocus(FocusSource),
    /// Notification that a widget has lost navigation focus
    LostNavFocus,
    /// Notification that a widget has gained selection focus
    ///
    /// This focus must be requested by calling
    /// [`EventState::request_sel_focus`] or [`EventState::request_key_focus`].
    SelFocus(FocusSource),
    /// Notification that a widget has lost selection focus
    ///
    /// In the case the widget also had character focus, [`Event::LostKeyFocus`] is
    /// received first.
    LostSelFocus,
    /// Notification that a widget has gained keyboard input focus
    ///
    /// This focus must be requested by calling
    /// [`EventState::request_key_focus`].
    ///
    /// This is always preceeded by [`Event::SelFocus`] and is received prior to
    /// [`Event::Key`] events.
    KeyFocus,
    /// Notification that a widget has lost keyboard input focus
    LostKeyFocus,
    /// Notification that a widget gains or loses mouse hover
    ///
    /// The payload is `true` when focus is gained, `false` when lost.
    MouseHover(bool),
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
            Event::CursorMove { ref mut press } => {
                press.coord += offset;
            }
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
    ///
    /// The method calls [`EventState::depress_with_key`] on activation.
    pub fn on_activate<F: FnOnce(&mut EventCx) -> IsUsed>(
        self,
        cx: &mut EventCx,
        id: Id,
        f: F,
    ) -> IsUsed {
        match self {
            Event::Command(cmd, code) if cmd.is_activate() => {
                cx.depress_with_key(id, code);
                f(cx)
            }
            Event::PressStart { press, .. } if press.is_primary() => press.grab(id).with_cx(cx),
            Event::PressEnd { press, success } => {
                if success && id == press.id {
                    f(cx)
                } else {
                    Used
                }
            }
            _ => Unused,
        }
    }

    /// Pass to disabled widgets?
    ///
    /// Disabled status should disable input handling but not prevent other
    /// notifications.
    pub fn pass_when_disabled(&self) -> bool {
        use Event::*;
        match self {
            Command(_, _) => false,
            Key(_, _) | Scroll(_) | Pan { .. } => false,
            CursorMove { .. } | PressStart { .. } | PressMove { .. } | PressEnd { .. } => false,
            Timer(_) | PopupClosed(_) => true,
            NavFocus { .. } | SelFocus(_) | KeyFocus | MouseHover(_) => false,
            LostNavFocus | LostKeyFocus | LostSelFocus => true,
        }
    }

    /// Can the event be received by [`Events::handle_event`] during unwinding?
    ///
    /// Some events are sent to the widget with navigation focus (e.g.
    /// [`Event::Command`]). Others are sent to the widget under the mouse (e.g.
    /// [`Event::PressStart`]). All these events may be "reused" by an ancestor
    /// widget if not [`Used`] by the original target.
    ///
    /// Other events are sent to a specific widget as a result of a request
    /// (e.g. [`Event::Key`], [`Event::PressEnd`]), or as a notification of
    /// focus change (e.g. [`Event::LostKeyFocus`]). These events may never be
    /// "reused".
    ///
    /// Note: this could alternatively be seen as a property of the addressing
    /// mechanism, currently just an [`Id`].
    pub fn is_reusable(&self) -> bool {
        use Event::*;
        match self {
            // Events sent to navigation focus given some code,
            // otherwise sent to a specific target.
            Command(_, code) => code.is_some(),

            // Events sent to mouse focus
            Scroll(_) | Pan { .. } => true,
            CursorMove { .. } | PressStart { .. } => true,

            // Events sent to requester
            Key(_, _) => false,
            PressMove { .. } | PressEnd { .. } => false,
            Timer(_) => false,

            // Notifications of focus/status change
            PopupClosed(_) => false,
            NavFocus { .. } | LostNavFocus => false,
            SelFocus(_) | LostSelFocus => false,
            KeyFocus | LostKeyFocus => false,
            MouseHover(_) => false,
        }
    }
}

/// Command input ([`Event::Command`])
///
/// `Command` events are mostly produced as a result of OS-specific keyboard
/// bindings; for example,  [`Command::Copy`] is produced by pressing
/// <kbd>Command+C</kbd> on MacOS or <kbd>Ctrl+C</kbd> on other platforms.
/// See [`crate::config::Shortcuts`] for more on these bindings.
///
/// A `Command` event does not necessarily come from keyboard input; for example
/// some menu widgets send [`Command::Activate`] to trigger an entry as a result
/// of mouse input.
///
/// *Most* `Command` entries represent an action (such as `Copy` or `FindNext`)
/// but some represent an important key whose action may be context-dependent
/// (e.g. `Escape`, `Space`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Command {
    /// Escape key
    ///
    /// Each press of this key should somehow relax control. It is expected that
    /// widgets receiving this key repeatedly eventually (soon) have no more
    /// use for this themselves and return it via [`Unused`].
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
    FindPrevious,

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
    NavPrevious,
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
    TabPrevious,

    /// Show help
    Help,
    /// Rename
    Rename,
    /// Refresh
    Refresh,
    /// Debug
    Debug,
    /// Spell-check tool
    SpellCheck,
    /// Open the context menu
    ContextMenu,
    /// Open or activate the application menu / menubar
    Menu,
    /// Make view fullscreen
    Fullscreen,

    /// Close window/tab/popup
    Close,
    /// Exit program (e.g. Ctrl+Q)
    Exit,
}

impl Command {
    /// Try constructing from a [`winit::keyboard::Key`]
    pub fn new(key: &Key) -> Option<Self> {
        match key {
            Key::Named(named) => Some(match named {
                NamedKey::ScrollLock => Command::ScrollLock,
                NamedKey::Enter => Command::Enter,
                NamedKey::Tab => Command::Tab,
                NamedKey::Space => Command::Space,
                NamedKey::ArrowDown => Command::Down,
                NamedKey::ArrowLeft => Command::Left,
                NamedKey::ArrowRight => Command::Right,
                NamedKey::ArrowUp => Command::Up,
                NamedKey::End => Command::End,
                NamedKey::Home => Command::Home,
                NamedKey::PageDown => Command::PageDown,
                NamedKey::PageUp => Command::PageUp,
                NamedKey::Backspace => Command::DelBack,
                NamedKey::Clear => Command::Deselect,
                NamedKey::Copy => Command::Copy,
                NamedKey::Cut => Command::Cut,
                NamedKey::Delete => Command::Delete,
                NamedKey::Insert => Command::Insert,
                NamedKey::Paste => Command::Paste,
                NamedKey::Redo | NamedKey::Again => Command::Redo,
                NamedKey::Undo => Command::Undo,
                NamedKey::ContextMenu => Command::ContextMenu,
                NamedKey::Escape => Command::Escape,
                NamedKey::Execute => Command::Activate,
                NamedKey::Find => Command::Find,
                NamedKey::Help => Command::Help,
                NamedKey::Pause => Command::Pause,
                NamedKey::Select => Command::SelectAll,
                NamedKey::PrintScreen => Command::Snapshot,
                // NamedKey::Close => CloseDocument ?
                NamedKey::New => Command::New,
                NamedKey::Open => Command::Open,
                NamedKey::Print => Command::Print,
                NamedKey::Save => Command::Save,
                NamedKey::SpellCheck => Command::SpellCheck,
                NamedKey::BrowserBack | NamedKey::GoBack => Command::NavPrevious,
                NamedKey::BrowserForward => Command::NavNext,
                NamedKey::BrowserRefresh => Command::Refresh,
                NamedKey::Exit => Command::Exit,
                _ => return None,
            }),
            _ => None,
        }
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

/// Reason that navigation focus is received
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FocusSource {
    /// Focus is received as a result of a mouse or touch event
    Pointer,
    /// Focus is received as a result of keyboard navigation (usually
    /// <kbd>Tab</kbd>) or a command ([`Event::Command`])
    Key,
    /// Focus is received from a programmatic event
    Synthetic,
}

impl FocusSource {
    pub fn key_or_synthetic(self) -> bool {
        match self {
            FocusSource::Pointer => false,
            FocusSource::Key | FocusSource::Synthetic => true,
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
