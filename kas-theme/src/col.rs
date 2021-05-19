// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour schemes

use kas::draw::{color::Rgba, InputState, TextClass};

/// Provides standard theme colours
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct ThemeColours {
    /// Background colour
    pub background: Rgba,
    /// Colour for frames (not always used)
    pub frame: Rgba,
    /// Background colour of `EditBox`
    pub bg: Rgba,
    /// Background colour of `EditBox` (disabled state)
    pub bg_disabled: Rgba,
    /// Background colour of `EditBox` (error state)
    pub bg_error: Rgba,
    /// Text colour in an `EditBox`
    pub text: Rgba,
    /// Selected tect colour
    pub text_sel: Rgba,
    /// Selected text background colour
    pub text_sel_bg: Rgba,
    /// Text colour in a `Label`
    pub label_text: Rgba,
    /// Text colour on a `TextButton`
    pub button_text: Rgba,
    /// Highlight colour for keyboard navigation
    pub nav_focus: Rgba,
    /// Colour of a `TextButton`
    pub button: Rgba,
    /// Colour of a `TextButton` (disabled state)
    pub button_disabled: Rgba,
    /// Colour of a `TextButton` when hovered by the mouse
    pub button_highlighted: Rgba,
    /// Colour of a `TextButton` when depressed
    pub button_depressed: Rgba,
    /// Colour of mark within a `CheckBox` or `RadioBox`
    pub checkbox: Rgba,
}

impl Default for ThemeColours {
    #[inline]
    fn default() -> Self {
        ThemeColours::white_blue()
    }
}

impl ThemeColours {
    /// White background with blue activable items
    pub fn white_blue() -> Self {
        ThemeColours {
            background: Rgba::grey(1.0),
            frame: Rgba::grey(0.7),
            bg: Rgba::grey(1.0),
            bg_disabled: Rgba::grey(0.85),
            bg_error: Rgba::rgb(1.0, 0.5, 0.5),
            text: Rgba::grey(0.0),
            text_sel: Rgba::grey(1.0),
            text_sel_bg: Rgba::rgb(0.15, 0.525, 0.75),
            label_text: Rgba::grey(0.0),
            button_text: Rgba::grey(1.0),
            nav_focus: Rgba::rgb(0.9, 0.65, 0.4),
            button: Rgba::rgb(0.2, 0.7, 1.0),
            button_disabled: Rgba::grey(0.5),
            button_highlighted: Rgba::rgb(0.25, 0.8, 1.0),
            button_depressed: Rgba::rgb(0.15, 0.525, 0.75),
            checkbox: Rgba::rgb(0.2, 0.7, 1.0),
        }
    }

    /// Light scheme
    pub fn light() -> Self {
        ThemeColours {
            background: Rgba::grey(0.9),
            frame: Rgba::rgb(0.8, 0.8, 0.9),
            bg: Rgba::grey(1.0),
            bg_disabled: Rgba::grey(0.85),
            bg_error: Rgba::rgb(1.0, 0.5, 0.5),
            text: Rgba::grey(0.0),
            text_sel: Rgba::grey(0.0),
            text_sel_bg: Rgba::rgb(0.8, 0.72, 0.24),
            label_text: Rgba::grey(0.0),
            button_text: Rgba::grey(0.0),
            nav_focus: Rgba::rgb(0.9, 0.65, 0.4),
            button: Rgba::rgb(1.0, 0.9, 0.3),
            button_disabled: Rgba::grey(0.6),
            button_highlighted: Rgba::rgb(1.0, 0.95, 0.6),
            button_depressed: Rgba::rgb(0.8, 0.72, 0.24),
            checkbox: Rgba::grey(0.4),
        }
    }

    /// Dark scheme
    pub fn dark() -> Self {
        ThemeColours {
            background: Rgba::grey(0.2),
            frame: Rgba::grey(0.4),
            bg: Rgba::grey(0.1),
            bg_disabled: Rgba::grey(0.3),
            bg_error: Rgba::rgb(1.0, 0.5, 0.5),
            text: Rgba::grey(1.0),
            text_sel: Rgba::grey(1.0),
            text_sel_bg: Rgba::rgb(0.6, 0.3, 0.1),
            label_text: Rgba::grey(1.0),
            button_text: Rgba::grey(1.0),
            nav_focus: Rgba::rgb(1.0, 0.7, 0.5),
            button: Rgba::rgb(0.5, 0.1, 0.1),
            button_disabled: Rgba::grey(0.7),
            button_highlighted: Rgba::rgb(0.6, 0.3, 0.1),
            button_depressed: Rgba::rgb(0.3, 0.1, 0.1),
            checkbox: Rgba::rgb(0.5, 0.1, 0.1),
        }
    }

    /// Get colour of a text area, depending on state
    pub fn bg_col(&self, state: InputState) -> Rgba {
        if state.disabled {
            self.bg_disabled
        } else if state.error {
            self.bg_error
        } else {
            self.bg
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
    pub fn button_state(&self, state: InputState) -> Rgba {
        if state.disabled {
            self.button_disabled
        } else if state.depress {
            self.button_depressed
        } else if state.hover {
            self.button_highlighted
        } else {
            self.button
        }
    }

    /// Get colour for a checkbox mark, depending on state
    pub fn check_mark_state(&self, state: InputState, checked: bool) -> Option<Rgba> {
        Some(if checked {
            if state.disabled {
                self.button_disabled
            } else if state.depress {
                self.button_depressed
            } else if state.hover {
                self.button_highlighted
            } else {
                self.checkbox
            }
        } else if state.depress {
            self.button_depressed
        } else {
            return None;
        })
    }

    /// Get background highlight colour of a menu entry, if any
    pub fn menu_entry(&self, state: InputState) -> Option<Rgba> {
        if state.depress || state.nav_focus {
            Some(self.button_depressed)
        } else if state.hover {
            Some(self.button_highlighted)
        } else {
            None
        }
    }

    /// Get colour of a scrollbar, depending on state
    #[inline]
    pub fn scrollbar_state(&self, state: InputState) -> Rgba {
        self.button_state(state)
    }

    /// Get text colour from class
    pub fn text_class(&self, class: TextClass) -> Rgba {
        match class {
            TextClass::Label | TextClass::LabelFixed | TextClass::LabelScroll => self.label_text,
            TextClass::Button => self.button_text,
            TextClass::Edit | TextClass::EditMulti => self.text,
        }
    }
}
