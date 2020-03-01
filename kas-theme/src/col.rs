// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour schemes

use log::warn;

use kas::draw::Colour;
use kas::event::HighlightState;

/// Provides standard theme colours
#[derive(Clone, Debug)]
pub struct ThemeColours {
    /// Background colour
    pub background: Colour,
    /// Colour for frames (not always used)
    pub frame: Colour,
    /// Background colour of `EditBox`
    pub text_area: Colour,
    /// Text colour in an `EditBox`
    pub text: Colour,
    /// Text colour in a `Label`
    pub label_text: Colour,
    /// Text colour on a `TextButton`
    pub button_text: Colour,
    /// Highlight colour for keyboard navigation
    pub nav_focus: Colour,
    /// Colour of a `TextButton`
    pub button: Colour,
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
            "default" => Self::new(),
            "light" => Self::light(),
            "dark" => Self::dark(),
            other => {
                warn!("ThemeColours::open: scheme \"{}\" not found", other);
                return None;
            }
        })
    }

    /// Default theme: grey with blue activable items
    pub fn new() -> Self {
        ThemeColours {
            background: Colour::grey(0.8),
            frame: Colour::grey(0.7),
            text_area: Colour::grey(1.0),
            text: Colour::grey(0.0),
            label_text: Colour::grey(0.0),
            button_text: Colour::grey(1.0),
            nav_focus: Colour::new(1.0, 0.7, 0.5),
            button: Colour::new(0.2, 0.7, 1.0),
            button_highlighted: Colour::new(0.25, 0.8, 1.0),
            button_depressed: Colour::new(0.15, 0.525, 0.75),
            checkbox: Colour::new(0.2, 0.7, 1.0),
        }
    }

    /// Light scheme
    pub fn light() -> Self {
        ThemeColours {
            background: Colour::grey(0.9),
            frame: Colour::new(0.8, 0.8, 0.9),
            text_area: Colour::grey(1.0),
            text: Colour::grey(0.0),
            label_text: Colour::grey(0.0),
            button_text: Colour::grey(0.0),
            nav_focus: Colour::new(1.0, 0.7, 0.5),
            button: Colour::new(1.0, 1.0, 0.8),
            button_highlighted: Colour::new(1.0, 1.0, 0.6),
            button_depressed: Colour::new(0.8, 0.8, 0.6),
            checkbox: Colour::grey(0.4),
        }
    }

    /// Dark scheme
    pub fn dark() -> Self {
        ThemeColours {
            background: Colour::grey(0.2),
            frame: Colour::grey(0.4),
            text_area: Colour::grey(0.1),
            text: Colour::grey(1.0),
            label_text: Colour::grey(1.0),
            button_text: Colour::grey(1.0),
            nav_focus: Colour::new(1.0, 0.7, 0.5),
            button: Colour::new(0.5, 0.1, 0.1),
            button_highlighted: Colour::new(0.6, 0.3, 0.1),
            button_depressed: Colour::new(0.3, 0.1, 0.1),
            checkbox: Colour::new(0.5, 0.1, 0.1),
        }
    }

    /// Get colour for navigation highlight region, if any
    pub fn nav_region(&self, highlights: HighlightState) -> Option<Colour> {
        if highlights.nav_focus {
            Some(self.nav_focus)
        } else {
            None
        }
    }

    /// Get colour for a button, depending on state
    pub fn button_state(&self, highlights: HighlightState) -> Colour {
        if highlights.depress {
            self.button_depressed
        } else if highlights.hover {
            self.button_highlighted
        } else {
            self.button
        }
    }

    /// Get colour for a checkbox mark, depending on state
    pub fn check_mark_state(&self, highlights: HighlightState, checked: bool) -> Option<Colour> {
        if highlights.depress {
            Some(self.button_depressed)
        } else if checked && highlights.hover {
            Some(self.button_highlighted)
        } else if checked {
            Some(self.checkbox)
        } else {
            None
        }
    }

    /// Get colour of a scrollbar, depending on state
    #[inline]
    pub fn scrollbar_state(&self, highlights: HighlightState) -> Colour {
        self.button_state(highlights)
    }
}
