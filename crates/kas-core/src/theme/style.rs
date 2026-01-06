// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme style components

use crate::dir::Direction;

/// Margin size
///
/// Default value: [`MarginStyle::Large`].
#[crate::impl_default(MarginStyle::Large)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MarginStyle {
    /// No margins
    None,
    /// Inner margin, used to draw highlight/selection boxes
    ///
    /// Guide size: 1px at 100%, 2px at 125%, 2px at 150%, 3px at 200%.
    ///
    /// This is the smallest of the fixed margin sizes, and only really
    /// useful to reserve space for drawing selection boxes.
    Inner,
    /// A small margin for inner layout
    ///
    /// Guide size: 2px at 100%, 3px at 125%, 4px at 150%, 5px at 200%.
    Tiny,
    /// Small external margin size
    ///
    /// Guide size: 4px at 100%, 5px at 125%, 7px at 150%, 9px at 200%.
    Small,
    /// Large margin, used between elements such as buttons
    ///
    /// Guide size: 7px at 100%, 9px at 125%, 11px at 150%, 15px at 200%.
    Large,
    /// Huge margin, used between things like file icons
    ///
    /// Guide size: 15px at 100%.
    Huge,
    /// Text margins
    ///
    /// Margins for use around standard text elements (may be asymmetric).
    Text,
    /// Specify in pixels (scaled)
    Px(f32),
    /// Specify in Em (font size)
    Em(f32),
}

/// Style of marks
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum MarkStyle {
    /// A chevron (i.e. arrow without stalk) pointing in the given direction
    Chevron(Direction),
    /// A cross rotated 45°
    X,
    /// Plus (+) symbol
    Plus,
    /// Minus (-) symbol
    Minus,
}

/// Various features which may be sized and drawn
///
/// Includes most types of features excepting text and frames.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Feature {
    Separator,
    Mark(MarkStyle),
    CheckBox,
    RadioBox,
    ScrollBar(Direction),
    Slider(Direction),
    ProgressBar(Direction),
}

impl From<MarkStyle> for Feature {
    fn from(style: MarkStyle) -> Self {
        Feature::Mark(style)
    }
}

/// Style of a frame
///
/// A "frame" is an element surrounding another element.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum FrameStyle {
    /// No frame, just draw the background and force margins to be internal
    #[default]
    None,
    /// A frame for grouping content
    Frame,
    /// A frame around pop-ups
    Popup,
    /// Border around a pop-up menu entry
    MenuEntry,
    /// Frame used to indicate navigation focus
    NavFocus,
    /// Border of a button
    Button,
    /// Border of a button which is visible only when under the mouse
    InvisibleButton,
    /// Border of a tab
    Tab,
    /// Frame with a background, often used for editable text
    EditBox,
    /// Window decoration (excludes top buttons)
    Window,
}

/// Selection style hint
///
/// How to draw selections
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum SelectionStyle {
    /// Adjust background color
    Highlight,
    /// Draw a frame around the selection
    Frame,
    /// Both
    Both,
}

impl SelectionStyle {
    /// True if an external margin is required
    ///
    /// Margin size is [`SizeCx::inner_margins`](super::SizeCx::inner_margins)
    pub fn is_external(self) -> bool {
        matches!(self, SelectionStyle::Frame | SelectionStyle::Both)
    }
}

/// Font "class" selector
///
/// Fonts are chosen from available (system) fonts depending on the `TextClass`
/// and [configuration](crate::config::FontConfig).
/// `TextClass` may affect other font properties, including size and weight.
///
/// Some classes by default appear identical to [`TextClass::Standard`] but may
/// be configured otherwise by the user.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, linearize::Linearize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextClass {
    /// The standard UI font
    ///
    /// By default this matches the system UI font.
    Standard,
    /// Label UI font
    ///
    /// This text class should be used by short labels such as those found on
    /// buttons, menus and other UI controls.
    ///
    /// Its appearance is normally identical to [`TextClass::Standard`].
    Label,
    /// Small UI font
    ///
    /// This class is usually similar to [`TextClass::Standard`] but smaller.
    Small,
    /// Editable text
    ///
    /// This text class should be preferred for editable text.
    ///
    /// Its appearance is normally identical to [`TextClass::Standard`].
    Editor,
    /// Serif font
    ///
    /// This class may be used where a Serif font is specifically preferred,
    /// for example in an editor where it is important to be able to distinguish
    /// all letters.
    Serif,
    /// Sans-serif Font
    ///
    /// Its appearance is normally identical to [`TextClass::Standard`].
    SansSerif,
    /// Monospace font
    Monospace,
}
