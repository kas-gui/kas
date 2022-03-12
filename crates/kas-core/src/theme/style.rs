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
    ///
    /// This takes one parameter: `multi_line`. Text is wrapped only if true.
    Label(bool),
    /// Scrollable label
    ///
    /// This is similar to `Label(true)`, but may occupy less vertical space.
    /// Usually it also implies that the text is both scrollable and selectable,
    /// but these are characteristics of the widget, not the text object.
    LabelScroll,
    /// Label with accelerator keys
    ///
    /// This takes one parameter: `multi_line`. Text is wrapped only if true.
    ///
    /// This is identical to `Label` except that effects are only drawn if
    /// accelerator-key mode is activated (usually the `Alt` key).
    AccelLabel(bool),
    /// Button text is drawn over a button
    ///
    /// Same as `AccelLabel(false)`, though theme may differentiate.
    Button,
    /// Menu label (single line, does not stretch)
    ///
    /// Similar to `AccelLabel(false)`, but with horizontal stretching disabled.
    MenuLabel,
    /// Editable text, usually encapsulated in some type of box
    ///
    /// This takes one parameter: `multi_line`. Text is wrapped only if true.
    Edit(bool),
}

impl TextClass {
    /// True if text is single-line only
    #[inline]
    pub fn single_line(self) -> bool {
        !self.multi_line()
    }

    /// True if text is multi-line and should automatically line-wrap
    #[inline]
    pub fn multi_line(self) -> bool {
        use TextClass::*;
        matches!(
            self,
            Label(true) | LabelScroll | AccelLabel(true) | Edit(true)
        )
    }

    /// True if text effects should only be shown dependant on accelerator-key
    /// mode being active
    #[inline]
    pub fn is_accel(self) -> bool {
        use TextClass::*;
        matches!(self, AccelLabel(_) | Button | MenuLabel)
    }
}

/// Style of a frame
///
/// A "frame" is an element surrounding another element.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum FrameStyle {
    /// An invisible frame which forces all margins to be interior
    InnerMargin,
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
