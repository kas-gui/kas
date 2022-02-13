// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme style components

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

/// Style of a frame
///
/// A "frame" is an element surrounding another element.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum FrameStyle {
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
    /// Box used to contain editable text
    EditBox,
}
