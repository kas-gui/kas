// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour schemes

use kas::draw::color::{Rgba, Rgba8Srgb};
use kas::draw::InputState;

const MULT_DEPRESS: f32 = 0.75;
const MULT_HIGHLIGHT: f32 = 1.25;
const MIN_HIGHLIGHT: f32 = 0.2;

/// Provides standard theme colours
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct Colors<C> {
    /// Background colour
    pub background: C,
    /// Colour for frames (not always used)
    pub frame: C,
    /// Background colour of `EditBox`
    pub edit_bg: C,
    /// Background colour of `EditBox` (error state)
    pub edit_bg_error: C,
    /// Normal text colour (over background)
    pub text: C,
    /// Opposing text colour (e.g. white if `text` is black)
    pub text_invert: C,
    /// Disabled text colour
    pub text_disabled: C,
    /// Selected text background colour
    pub text_sel_bg: C,
    /// Highlight colour for keyboard navigation
    pub nav_focus: C,
    /// Colour of a `TextButton`
    pub button: C,
    /// Colour of mark within a `CheckBox` or `RadioBox`
    pub checkbox: C,
}

/// [`Colors`] parameterised for reading and writing using sRGB
pub type ColorsSrgb = Colors<Rgba8Srgb>;

/// [`Colors`] parameterised for graphics usage
pub type ColorsLinear = Colors<Rgba>;

impl From<ColorsSrgb> for ColorsLinear {
    fn from(col: ColorsSrgb) -> Self {
        Colors {
            background: col.background.into(),
            frame: col.frame.into(),
            edit_bg: col.edit_bg.into(),
            edit_bg_error: col.edit_bg_error.into(),
            text: col.text.into(),
            text_invert: col.text_invert.into(),
            text_disabled: col.text_disabled.into(),
            text_sel_bg: col.text_sel_bg.into(),
            nav_focus: col.nav_focus.into(),
            button: col.button.into(),
            checkbox: col.checkbox.into(),
        }
    }
}

impl From<ColorsLinear> for ColorsSrgb {
    fn from(col: ColorsLinear) -> Self {
        Colors {
            background: col.background.into(),
            frame: col.frame.into(),
            edit_bg: col.edit_bg.into(),
            edit_bg_error: col.edit_bg_error.into(),
            text: col.text.into(),
            text_invert: col.text_invert.into(),
            text_disabled: col.text_disabled.into(),
            text_sel_bg: col.text_sel_bg.into(),
            nav_focus: col.nav_focus.into(),
            button: col.button.into(),
            checkbox: col.checkbox.into(),
        }
    }
}

impl Default for ColorsLinear {
    #[inline]
    fn default() -> Self {
        Colors::white_blue()
    }
}

impl Default for ColorsSrgb {
    #[inline]
    fn default() -> Self {
        ColorsLinear::default().into()
    }
}

// NOTE: these colour schemes are defined using linear (Rgba) colours instead of
// sRGB (Rgba8Srgb) colours for historical reasons. Either should be fine.
impl ColorsLinear {
    /// White background with blue activable items
    pub fn white_blue() -> Self {
        Colors {
            background: Rgba::grey(1.0),
            frame: Rgba::grey(0.7),
            edit_bg: Rgba::grey(1.0),
            edit_bg_error: Rgba::rgb(1.0, 0.5, 0.5),
            text: Rgba::BLACK,
            text_invert: Rgba::WHITE,
            text_disabled: Rgba::grey(0.4),
            text_sel_bg: Rgba::rgb(0.15, 0.525, 0.75),
            nav_focus: Rgba::rgb(0.9, 0.65, 0.4),
            button: Rgba::rgb(0.2, 0.7, 1.0),
            checkbox: Rgba::rgb(0.2, 0.7, 1.0),
        }
    }

    /// Light scheme
    pub fn light() -> Self {
        Colors {
            background: Rgba::grey(0.9),
            frame: Rgba::grey(0.5),
            edit_bg: Rgba::grey(1.0),
            edit_bg_error: Rgba::rgb(1.0, 0.5, 0.5),
            text: Rgba::BLACK,
            text_invert: Rgba::WHITE,
            text_disabled: Rgba::grey(0.4),
            text_sel_bg: Rgba::rgb(0.8, 0.72, 0.24),
            nav_focus: Rgba::rgb(0.9, 0.65, 0.4),
            button: Rgba::rgb(1.0, 0.9, 0.3),
            checkbox: Rgba::grey(0.4),
        }
    }

    /// Dark scheme
    pub fn dark() -> Self {
        Colors {
            background: Rgba::grey(0.2),
            frame: Rgba::grey(0.4),
            edit_bg: Rgba::grey(0.1),
            edit_bg_error: Rgba::rgb(1.0, 0.5, 0.5),
            text: Rgba::WHITE,
            text_invert: Rgba::BLACK,
            text_disabled: Rgba::grey(0.6),
            text_sel_bg: Rgba::rgb(0.6, 0.3, 0.1),
            nav_focus: Rgba::rgb(1.0, 0.7, 0.5),
            button: Rgba::rgb(0.5, 0.1, 0.1),
            checkbox: Rgba::rgb(0.5, 0.1, 0.1),
        }
    }

    /// Adjust a colour depending on state
    pub fn adjust_for_state(col: Rgba, state: InputState) -> Rgba {
        if state.disabled {
            col.average()
        } else if state.depress {
            col.multiply(MULT_DEPRESS)
        } else if state.hover {
            col.multiply(MULT_HIGHLIGHT).max(MIN_HIGHLIGHT)
        } else {
            col
        }
    }

    /// Get colour of a text area, depending on state
    pub fn bg_col(&self, state: InputState) -> Rgba {
        if state.disabled {
            self.edit_bg.average()
        } else if state.error {
            self.edit_bg_error
        } else {
            self.edit_bg
        }
    }

    /// Get colour for navigation highlight region, if any
    pub fn nav_region(&self, state: InputState) -> Option<Rgba> {
        if state.nav_focus && !state.disabled {
            Some(self.nav_focus)
        } else {
            None
        }
    }

    /// Get colour for a button, depending on state
    #[inline]
    pub fn button_state(&self, state: InputState) -> Rgba {
        Self::adjust_for_state(self.button, state)
    }

    /// Get colour for a checkbox mark, depending on state
    pub fn check_mark_state(&self, state: InputState, checked: bool) -> Option<Rgba> {
        if checked {
            Some(Self::adjust_for_state(self.checkbox, state))
        } else if state.depress {
            Some(self.checkbox.multiply(MULT_DEPRESS))
        } else {
            None
        }
    }

    /// Get background highlight colour of a menu entry, if any
    pub fn menu_entry(&self, state: InputState) -> Option<Rgba> {
        if state.depress || state.nav_focus {
            Some(self.button.multiply(MULT_DEPRESS))
        } else {
            None
        }
    }

    /// Get colour of a scrollbar, depending on state
    #[inline]
    pub fn scrollbar_state(&self, state: InputState) -> Rgba {
        self.button_state(state)
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
