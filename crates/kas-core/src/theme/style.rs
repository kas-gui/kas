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
    /// An arrowhead/angle-bracket/triangle pointing in the given direction
    Point(Direction),
    /// A cross rotated 45°
    X,
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
    /// No frame, just draw the background
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
    /// Border of a tab
    Tab,
    /// Box used to contain editable text
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
