// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour schemes

use kas::draw::color::{Rgba, Rgba8Srgb};
use kas::event::EventState;
use kas::theme::Background;
use kas::WidgetId;
use std::str::FromStr;

const MULT_DEPRESS: f32 = 0.75;
const MULT_HIGHLIGHT: f32 = 1.25;
const MIN_HIGHLIGHT: f32 = 0.2;

bitflags::bitflags! {
    /// Input and highlighting state of a widget
    ///
    /// This struct is used to adjust the appearance of [`DrawMgr`]'s primitives.
    #[derive(Default)]
    pub struct InputState: u8 {
        /// Disabled widgets are not responsive to input and usually drawn in grey.
        ///
        /// All other states should be ignored when disabled.
        const DISABLED = 1 << 0;
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
    /// Construct, setting all components
    pub fn new_all(ev: &EventState, id: &WidgetId) -> Self {
        let mut state = Self::new_except_depress(ev, id);
        if ev.is_depressed(id) {
            state |= InputState::DEPRESS;
        }
        state
    }

    /// Construct, setting all but depress status
    pub fn new_except_depress(ev: &EventState, id: &WidgetId) -> Self {
        let (char_focus, sel_focus) = ev.has_char_focus(id);
        let mut state = InputState::empty();
        if ev.is_disabled(id) {
            state |= InputState::DISABLED;
        }
        if ev.is_hovered(id) {
            state |= InputState::HOVER;
        }
        if ev.has_nav_focus(id) {
            state |= InputState::NAV_FOCUS;
        }
        if char_focus {
            state |= InputState::CHAR_FOCUS;
        }
        if sel_focus {
            state |= InputState::SEL_FOCUS;
        }
        state
    }

    /// Construct, setting all components, also setting hover from `id2`
    pub fn new2(ev: &EventState, id: &WidgetId, id2: &WidgetId) -> Self {
        let mut state = Self::new_all(ev, id);
        if ev.is_hovered(id2) {
            state |= InputState::HOVER;
        }
        state
    }

    /// Extract `DISABLED` bit
    #[inline]
    pub fn disabled(self) -> bool {
        self.contains(InputState::DISABLED)
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

/// Provides standard theme colours
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct Colors<C> {
    /// True if this is a dark theme
    pub is_dark: bool,
    /// Background colour
    pub background: C,
    /// Colour for frames (not always used)
    pub frame: C,
    /// Background colour of `EditBox`
    pub edit_bg: C,
    /// Background colour of `EditBox` (disabled state)
    pub edit_bg_disabled: C,
    /// Background colour of `EditBox` (error state)
    pub edit_bg_error: C,
    /// Theme accent
    ///
    /// This should be a bold colour, used for small details.
    pub accent: C,
    /// Soft version of accent
    ///
    /// A softer version of the accent colour, used for block elements in some themes.
    pub accent_soft: C,
    /// Highlight colour for keyboard navigation
    ///
    /// This may be the same as `accent`. It should contrast well with
    /// `accent_soft`. Themes should use `nav_focus` over `accent` where a
    /// strong contrast is required.
    pub nav_focus: C,
    /// Normal text colour (over background)
    pub text: C,
    /// Opposing text colour (e.g. white if `text` is black)
    pub text_invert: C,
    /// Disabled text colour
    pub text_disabled: C,
    /// Selected text background colour
    ///
    /// This may be the same as `accent_soft`.
    pub text_sel_bg: C,
}

/// [`Colors`] parameterised for reading and writing using sRGB
pub type ColorsSrgb = Colors<Rgba8Srgb>;

/// [`Colors`] parameterised for graphics usage
pub type ColorsLinear = Colors<Rgba>;

impl From<ColorsSrgb> for ColorsLinear {
    fn from(col: ColorsSrgb) -> Self {
        Colors {
            is_dark: col.is_dark,
            background: col.background.into(),
            frame: col.frame.into(),
            accent: col.accent.into(),
            accent_soft: col.accent_soft.into(),
            nav_focus: col.nav_focus.into(),
            edit_bg: col.edit_bg.into(),
            edit_bg_disabled: col.edit_bg_disabled.into(),
            edit_bg_error: col.edit_bg_error.into(),
            text: col.text.into(),
            text_invert: col.text_invert.into(),
            text_disabled: col.text_disabled.into(),
            text_sel_bg: col.text_sel_bg.into(),
        }
    }
}

impl From<ColorsLinear> for ColorsSrgb {
    fn from(col: ColorsLinear) -> Self {
        Colors {
            is_dark: col.is_dark,
            background: col.background.into(),
            frame: col.frame.into(),
            accent: col.accent.into(),
            accent_soft: col.accent_soft.into(),
            nav_focus: col.nav_focus.into(),
            edit_bg: col.edit_bg.into(),
            edit_bg_disabled: col.edit_bg_disabled.into(),
            edit_bg_error: col.edit_bg_error.into(),
            text: col.text.into(),
            text_invert: col.text_invert.into(),
            text_disabled: col.text_disabled.into(),
            text_sel_bg: col.text_sel_bg.into(),
        }
    }
}

impl Default for ColorsLinear {
    #[inline]
    fn default() -> Self {
        ColorsSrgb::default().into()
    }
}

impl Default for ColorsSrgb {
    #[inline]
    fn default() -> Self {
        ColorsSrgb::light()
    }
}

impl ColorsSrgb {
    /// Default "light" scheme
    pub fn light() -> Self {
        Colors {
            is_dark: false,
            background: Rgba8Srgb::from_str("#FAFAFA").unwrap(),
            frame: Rgba8Srgb::from_str("#BCBCBC").unwrap(),
            accent: Rgba8Srgb::from_str("#8347f2").unwrap(),
            accent_soft: Rgba8Srgb::from_str("#B38DF9").unwrap(),
            nav_focus: Rgba8Srgb::from_str("#7E3FF2").unwrap(),
            edit_bg: Rgba8Srgb::from_str("#FAFAFA").unwrap(),
            edit_bg_disabled: Rgba8Srgb::from_str("#DCDCDC").unwrap(),
            edit_bg_error: Rgba8Srgb::from_str("#FFBCBC").unwrap(),
            text: Rgba8Srgb::from_str("#000000").unwrap(),
            text_invert: Rgba8Srgb::from_str("#FFFFFF").unwrap(),
            text_disabled: Rgba8Srgb::from_str("#AAAAAA").unwrap(),
            text_sel_bg: Rgba8Srgb::from_str("#A172FA").unwrap(),
        }
    }

    /// Dark scheme
    pub fn dark() -> Self {
        Colors {
            is_dark: true,
            background: Rgba8Srgb::from_str("#404040").unwrap(),
            frame: Rgba8Srgb::from_str("#AAAAAA").unwrap(),
            accent: Rgba8Srgb::from_str("#F74C00").unwrap(),
            accent_soft: Rgba8Srgb::from_str("#E77346").unwrap(),
            nav_focus: Rgba8Srgb::from_str("#D03E00").unwrap(),
            edit_bg: Rgba8Srgb::from_str("#303030").unwrap(),
            edit_bg_disabled: Rgba8Srgb::from_str("#606060").unwrap(),
            edit_bg_error: Rgba8Srgb::from_str("#FFBCBC").unwrap(),
            text: Rgba8Srgb::from_str("#FFFFFF").unwrap(),
            text_invert: Rgba8Srgb::from_str("#000000").unwrap(),
            text_disabled: Rgba8Srgb::from_str("#CBCBCB").unwrap(),
            text_sel_bg: Rgba8Srgb::from_str("#E77346").unwrap(),
        }
    }

    /// Blue scheme
    pub fn blue() -> Self {
        Colors {
            is_dark: false,
            background: Rgba8Srgb::from_str("#FFFFFF").unwrap(),
            frame: Rgba8Srgb::from_str("#DADADA").unwrap(),
            accent: Rgba8Srgb::from_str("#3fafd7").unwrap(),
            accent_soft: Rgba8Srgb::from_str("#7CDAFF").unwrap(),
            nav_focus: Rgba8Srgb::from_str("#3B697A").unwrap(),
            edit_bg: Rgba8Srgb::from_str("#FFFFFF").unwrap(),
            edit_bg_disabled: Rgba8Srgb::from_str("#DCDCDC").unwrap(),
            edit_bg_error: Rgba8Srgb::from_str("#FFBCBC").unwrap(),
            text: Rgba8Srgb::from_str("#000000").unwrap(),
            text_invert: Rgba8Srgb::from_str("#FFFFFF").unwrap(),
            text_disabled: Rgba8Srgb::from_str("#AAAAAA").unwrap(),
            text_sel_bg: Rgba8Srgb::from_str("#6CC0E1").unwrap(),
        }
    }
}

impl ColorsLinear {
    /// Adjust a colour depending on state
    pub fn adjust_for_state(col: Rgba, state: InputState) -> Rgba {
        if state.disabled() {
            col.average()
        } else if state.depress() {
            col.multiply(MULT_DEPRESS)
        } else if state.hover() || state.char_focus() {
            col.multiply(MULT_HIGHLIGHT).max(MIN_HIGHLIGHT)
        } else {
            col
        }
    }

    /// Extract from [`Background`]
    pub fn from_bg(&self, bg: Background, state: InputState, force_accent: bool) -> Rgba {
        let use_accent = force_accent || state.depress() || state.nav_focus();
        let col = match bg {
            _ if state.disabled() => self.edit_bg_disabled,
            Background::Default if use_accent => self.accent_soft,
            Background::Default => self.background,
            Background::Error => self.edit_bg_error,
            Background::Rgb(rgb) => rgb.into(),
        };
        Self::adjust_for_state(col, state)
    }

    /// Get colour of a text area, depending on state
    pub fn from_edit_bg(&self, bg: Background, state: InputState) -> Rgba {
        let mut col = match bg {
            _ if state.disabled() => self.edit_bg_disabled,
            Background::Default => self.edit_bg,
            Background::Error => self.edit_bg_error,
            Background::Rgb(rgb) => rgb.into(),
        };
        if state.depress() {
            col = col.multiply(MULT_DEPRESS);
        }
        col
    }

    /// Get colour for navigation highlight region, if any
    pub fn nav_region(&self, state: InputState) -> Option<Rgba> {
        if state.depress() || state.nav_focus() && !state.disabled() {
            Some(self.nav_focus)
        } else {
            None
        }
    }

    /// Get accent colour, adjusted for state
    #[inline]
    pub fn accent_state(&self, state: InputState) -> Rgba {
        Self::adjust_for_state(self.accent, state)
    }

    /// Get soft accent colour, adjusted for state
    #[inline]
    pub fn accent_soft_state(&self, state: InputState) -> Rgba {
        Self::adjust_for_state(self.accent_soft, state)
    }

    /// Get colour for a check box mark, depending on state
    #[inline]
    pub fn check_mark_state(&self, state: InputState) -> Rgba {
        Self::adjust_for_state(self.accent, state)
    }

    /// Get background highlight colour of a menu entry, if any
    pub fn menu_entry(&self, state: InputState) -> Option<Rgba> {
        if state.depress() || state.nav_focus() {
            Some(self.accent_soft.multiply(MULT_DEPRESS))
        } else {
            None
        }
    }

    /// Get appropriate text colour over the given background
    pub fn text_over(&self, bg: Rgba) -> Rgba {
        let bg_sum = bg.sum();
        if (bg_sum - self.text_invert.sum()).abs() > (bg_sum - self.text.sum()).abs() {
            self.text_invert
        } else {
            self.text
        }
    }
}
