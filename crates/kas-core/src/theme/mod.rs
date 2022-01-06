// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme APIs

mod draw;
mod size;

pub use draw::{DrawHandle, DrawMgr};
pub use size::{SizeHandle, SizeMgr};

#[allow(unused)]
use crate::event::EventMgr;
use crate::TkAction;
use std::ops::{Deref, DerefMut};

bitflags! {
    /// Input and highlighting state of a widget
    ///
    /// This struct is used to adjust the appearance of [`DrawMgr`]'s primitives.
    #[derive(Default)]
    pub struct InputState: u8 {
        /// Disabled widgets are not responsive to input and usually drawn in grey.
        ///
        /// All other states should be ignored when disabled.
        const DISABLED = 1 << 0;
        /// Some widgets, such as `EditBox`, use a red background on error
        const ERROR = 1 << 1;
        /// "Hover" is true if the mouse is over this element
        const HOVER = 1 << 2;
        /// Elements such as buttons, handles and menu entries may be depressed
        /// (visually pushed) by a click or touch event or an accelerator key.
        /// This is often visualised by a darker colour and/or by offsetting
        /// graphics. The `hover` state should be ignored when depressed.
        const DEPRESS = 1 << 3;
        /// Keyboard navigation of UIs moves a "focus" from widget to widget.
        const NAV_FOCUS = 1 << 4;
        /// "Character focus" implies this widget is ready to receive text input
        /// (e.g. typing into an input field).
        const CHAR_FOCUS = 1 << 5;
        /// "Selection focus" allows things such as text to be selected. Selection
        /// focus implies that the widget also has character focus.
        const SEL_FOCUS = 1 << 6;
    }
}

impl InputState {
    /// Extract `DISABLED` bit
    #[inline]
    pub fn disabled(self) -> bool {
        self.contains(InputState::DISABLED)
    }

    /// Extract `ERROR` bit
    #[inline]
    pub fn error(self) -> bool {
        self.contains(InputState::ERROR)
    }

    /// Extract `HOVER` bit
    #[inline]
    pub fn hover(self) -> bool {
        self.contains(InputState::HOVER)
    }

    /// Extract `DEPRESS` bit
    #[inline]
    pub fn depress(self) -> bool {
        self.contains(InputState::DEPRESS)
    }

    /// Extract `NAV_FOCUS` bit
    #[inline]
    pub fn nav_focus(self) -> bool {
        self.contains(InputState::NAV_FOCUS)
    }

    /// Extract `CHAR_FOCUS` bit
    #[inline]
    pub fn char_focus(self) -> bool {
        self.contains(InputState::CHAR_FOCUS)
    }

    /// Extract `SEL_FOCUS` bit
    #[inline]
    pub fn sel_focus(self) -> bool {
        self.contains(InputState::SEL_FOCUS)
    }
}

/// Class of text drawn
///
/// Themes choose font, font size, colour, and alignment based on this.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextClass {
    /// Label text is drawn over the background colour
    Label,
    /// Scrollable label (same as label except that min height is limited)
    LabelScroll,
    /// Button text is drawn over a button
    Button,
    /// Class of text drawn in a single-line edit box
    Edit,
    /// Class of text drawn in a multi-line edit box
    EditMulti,
    /// Menu label (single line, does not stretch)
    MenuLabel,
}

impl TextClass {
    /// True if text should be automatically line-wrapped
    pub fn line_wrap(self) -> bool {
        self == TextClass::Label || self == TextClass::EditMulti
    }
}

/// Default class: Label
impl Default for TextClass {
    fn default() -> Self {
        TextClass::Label
    }
}

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`TkAction`] to enable correct action when a theme
/// is updated via [`EventMgr::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
pub trait ThemeControl {
    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    fn set_font_size(&mut self, pt_size: f32) -> TkAction;

    /// Change the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    fn set_scheme(&mut self, scheme: &str) -> TkAction;

    /// List available colour schemes
    fn list_schemes(&self) -> Vec<&str>;

    /// Switch the theme
    ///
    /// Most themes do not react to this method; `kas_theme::MultiTheme` uses
    /// it to switch themes.
    fn set_theme(&mut self, _theme: &str) -> TkAction {
        TkAction::empty()
    }
}

impl<T: ThemeControl> ThemeControl for Box<T> {
    fn set_font_size(&mut self, size: f32) -> TkAction {
        self.deref_mut().set_font_size(size)
    }
    fn set_scheme(&mut self, scheme: &str) -> TkAction {
        self.deref_mut().set_scheme(scheme)
    }
    fn list_schemes(&self) -> Vec<&str> {
        self.deref().list_schemes()
    }
    fn set_theme(&mut self, theme: &str) -> TkAction {
        self.deref_mut().set_theme(theme)
    }
}
