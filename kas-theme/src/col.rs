// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour schemes

use log::warn;

use kas::draw::{Colour, InputState, TextClass};

/// Provides standard theme colours
#[derive(Clone, Debug)]
pub struct ThemeColours {
    /// Background colour
    pub background: Colour,
    /// Colour for frames (not always used)
    pub frame: Colour,
    /// Background colour of `EditBox`
    pub bg: Colour,
    /// Background colour of `EditBox` (disabled state)
    pub bg_disabled: Colour,
    /// Background colour of `EditBox` (error state)
    pub bg_error: Colour,
    /// Text colour in an `EditBox`
    pub text: Colour,
    /// Selected tect colour
    pub text_sel: Colour,
    /// Selected text background colour
    pub text_sel_bg: Colour,
    /// Text colour in a `Label`
    pub label_text: Colour,
    /// Text colour on a `TextButton`
    pub button_text: Colour,
    /// Highlight colour for keyboard navigation
    pub nav_focus: Colour,
    /// Colour of a `TextButton`
    pub button: Colour,
    /// Colour of a `TextButton` (disabled state)
    pub button_disabled: Colour,
    /// Colour of a `TextButton` when hovered by the mouse
    pub button_highlighted: Colour,
    /// Colour of a `TextButton` when depressed
    pub button_depressed: Colour,
    /// Colour of mark within a `CheckBox` or `RadioBox`
    pub checkbox: Colour,
}

impl ThemeColours {
    /// Open the given scheme, if found
    ///
    /// TODO: the intention is that this method can read and cache data from
    /// external resources. For now, we simply hard-code a few instances.
    pub fn open(scheme: &str) -> Option<Self> {
        Some(match scheme {
            "default" | "white" => Self::new(),
            "grey" => Self::grey(),
            "light" => Self::light(),
            "dark" => Self::dark(),
            other => {
                warn!("ThemeColours::open: scheme \"{}\" not found", other);
                return None;
            }
        })
    }

    /// Default theme: white with blue activable items
    pub fn new() -> Self {
        ThemeColours {
            background: Colour::grey(1.0),
            frame: Colour::grey(0.7),
            bg: Colour::grey(1.0),
            bg_disabled: Colour::grey(0.85),
            bg_error: Colour::new(1.0, 0.5, 0.5),
            text: Colour::grey(0.0),
            text_sel: Colour::grey(1.0),
            text_sel_bg: Colour::new(0.15, 0.525, 0.75),
            label_text: Colour::grey(0.0),
            button_text: Colour::grey(1.0),
            nav_focus: Colour::new(1.0, 0.7, 0.5),
            button: Colour::new(0.2, 0.7, 1.0),
            button_disabled: Colour::grey(0.5),
            button_highlighted: Colour::new(0.25, 0.8, 1.0),
            button_depressed: Colour::new(0.15, 0.525, 0.75),
            checkbox: Colour::new(0.2, 0.7, 1.0),
        }
    }

    /// Grey with blue activable items
    pub fn grey() -> Self {
        let mut col = ThemeColours::new();
        col.background = Colour::grey(0.8);
        col
    }

    /// Light scheme
    pub fn light() -> Self {
        ThemeColours {
            background: Colour::grey(0.9),
            frame: Colour::new(0.8, 0.8, 0.9),
            bg: Colour::grey(1.0),
            bg_disabled: Colour::grey(0.85),
            bg_error: Colour::new(1.0, 0.5, 0.5),
            text: Colour::grey(0.0),
            text_sel: Colour::grey(0.0),
            text_sel_bg: Colour::new(0.8, 0.72, 0.24),
            label_text: Colour::grey(0.0),
            button_text: Colour::grey(0.0),
            nav_focus: Colour::new(1.0, 0.7, 0.5),
            button: Colour::new(1.0, 0.9, 0.3),
            button_disabled: Colour::grey(0.6),
            button_highlighted: Colour::new(1.0, 0.95, 0.6),
            button_depressed: Colour::new(0.8, 0.72, 0.24),
            checkbox: Colour::grey(0.4),
        }
    }

    /// Dark scheme
    pub fn dark() -> Self {
        ThemeColours {
            background: Colour::grey(0.2),
            frame: Colour::grey(0.4),
            bg: Colour::grey(0.1),
            bg_disabled: Colour::grey(0.3),
            bg_error: Colour::new(1.0, 0.5, 0.5),
            text: Colour::grey(1.0),
            text_sel: Colour::grey(1.0),
            text_sel_bg: Colour::new(0.6, 0.3, 0.1),
            label_text: Colour::grey(1.0),
            button_text: Colour::grey(1.0),
            nav_focus: Colour::new(1.0, 0.7, 0.5),
            button: Colour::new(0.5, 0.1, 0.1),
            button_disabled: Colour::grey(0.7),
            button_highlighted: Colour::new(0.6, 0.3, 0.1),
            button_depressed: Colour::new(0.3, 0.1, 0.1),
            checkbox: Colour::new(0.5, 0.1, 0.1),
        }
    }

    /// Get colour of a text area, depending on state
    pub fn bg_col(&self, state: InputState) -> Colour {
        if state.disabled {
            self.bg_disabled
        } else if state.error {
            self.bg_error
        } else {
            self.bg
        }
    }

    /// Get colour for navigation highlight region, if any
    pub fn nav_region(&self, state: InputState) -> Option<Colour> {
        if state.nav_focus && !state.disabled {
            Some(self.nav_focus)
        } else {
            None
        }
    }

    /// Get colour for a button, depending on state
    pub fn button_state(&self, state: InputState) -> Colour {
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
    pub fn check_mark_state(&self, state: InputState, checked: bool) -> Option<Colour> {
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
    pub fn menu_entry(&self, state: InputState) -> Option<Colour> {
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
    pub fn scrollbar_state(&self, state: InputState) -> Colour {
        self.button_state(state)
    }

    /// Get text colour from class
    pub fn text_class(&self, class: TextClass) -> Colour {
        match class {
            TextClass::Label => self.label_text,
            TextClass::Button => self.button_text,
            TextClass::Edit | TextClass::EditMulti => self.text,
        }
    }
}
