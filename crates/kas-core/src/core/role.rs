// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget roles

use crate::Id;
use crate::dir::Direction;
#[allow(unused)] use crate::event::EventState;
use crate::event::Key;
use crate::geom::Offset;
#[allow(unused)]
use crate::messages::{DecrementStep, IncrementStep, SetValueF64};
#[allow(unused)] use crate::{Layout, Tile};

/// Describes a widget's purpose and capabilities
///
/// This `enum` does not describe children; use [`Tile::child_indices`] for
/// that. This `enum` does not describe associated properties such as a label
/// or labelled-by relationship.
///
/// ### Messages
///
/// Some roles of widget are expected to accept specific messages, as outlined
/// below. See also [`EventState::send`] and related functions.
#[non_exhaustive]
pub enum Role<'a> {
    /// Role is unspecified or no listed role is applicable
    Unknown,
    /// A text label with the given contents, usually (but not necessarily) short and fixed
    Label(&'a str),
    /// A text label with an access key
    AccessLabel(&'a str, Key),
    /// A push button
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to trigger the button.
    Button,
    /// A checkable box
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to toggle the state.
    CheckBox(bool),
    /// A radio button
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to toggle the state.
    RadioButton(bool),
    /// A tab handle
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to activate the tab.
    Tab,
    /// A visible border surrounding or between other items
    Border,
    /// A scrollable region
    ScrollRegion {
        /// The current scroll offset (from zero to `max_offset`)
        offset: Offset,
        /// The maximum offset (non-negative)
        max_offset: Offset,
    },
    /// A scroll bar
    ScrollBar {
        /// Orientation (usually either `Down` or `Right`)
        direction: Direction,
        /// The current position (from zero to `max_value`)
        value: i32,
        /// The maximum position (non-negative)
        max_value: i32,
    },
    /// A small visual element
    Indicator,
    /// An image
    Image,
    /// A canvas
    Canvas,
    /// A text label supporting selection
    TextLabel {
        /// Text contents
        ///
        /// NOTE: it is likely that the representation here changes to
        /// accomodate more complex texts and potentially other details.
        text: &'a str,
        /// The cursor index within `contents`
        edit_pos: usize,
        /// The selection index. Equals `cursor` if the selection is empty.
        /// May be less than or greater than `cursor`. (Aside: some toolkits
        /// call this the selection anchor but Kas does not; see
        /// [`kas::text::SelectionHelper`].)
        sel_pos: usize,
    },
    /// Editable text
    ///
    /// ### Messages
    ///
    /// [`kas::messages::SetValueText`] may be used to replace the entire
    /// text. [`kas::messages::ReplaceSelectedText`] may be used to insert text
    /// at `edit_pos`, replacing all text between `edit_pos` and `sel_pos`.
    TextInput {
        /// Text contents
        ///
        /// NOTE: it is likely that the representation here changes to
        /// accomodate more complex texts and potentially other details.
        text: &'a str,
        /// Whether the text input supports multi-line text
        multi_line: bool,
        /// The cursor index within `contents`
        edit_pos: usize,
        /// The selection index. Equals `cursor` if the selection is empty.
        /// May be less than or greater than `cursor`. (Aside: some toolkits
        /// call this the selection anchor but Kas does not; see
        /// [`kas::text::SelectionHelper`].)
        sel_pos: usize,
    },
    /// A gripable handle
    ///
    /// This is a part of a slider, scroll-bar, splitter or similar widget which
    /// can be dragged by the mouse. Its [`Layout::rect`] may be queried.
    Grip,
    /// A slider input
    ///
    /// Note that values may not be finite; for example `max: f64::INFINITY`.
    ///
    /// ### Messages
    ///
    /// [`SetValueF64`] may be used to set the input value.
    ///
    /// [`IncrementStep`] and [`DecrementStep`] change the value by one step.
    Slider {
        /// Minimum value
        min: f64,
        /// Maximum value
        max: f64,
        /// Step
        step: f64,
        /// Current value
        value: f64,
    },
    /// A spinner: numeric edit box with up and down buttons
    ///
    /// Note that values may not be finite; for example `max: f64::INFINITY`.
    ///
    /// ### Messages
    ///
    /// [`SetValueF64`] may be used to set the input value.
    ///
    /// [`IncrementStep`] and [`DecrementStep`] change the value by one step.
    SpinButton {
        /// Minimum value
        min: f64,
        /// Maximum value
        max: f64,
        /// Step
        step: f64,
        /// Current value
        value: f64,
    },
    /// A progress bar
    ///
    /// The reported value should be between `0.0` and `1.0`.
    ProgressBar(f32),
    /// A menu bar
    MenuBar,
    /// An openable menu
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to open the menu.
    ///
    /// [`kas::messages::Expand`] and [`kas::messages::Collapse`] may be used to
    /// open and close the menu.
    Menu {
        /// True if the menu is open
        expanded: bool,
    },
    /// A drop-down combination box
    ///
    /// Includes the index and text of the active entry
    ///
    /// # Messages
    ///
    /// [`kas::messages::SetIndex`] may be used to set the selected entry.
    ///
    /// [`kas::messages::Expand`] and [`kas::messages::Collapse`] may be used to
    /// open and close the menu.
    ComboBox {
        /// Index of the current choice
        active: usize,
        /// Text of the current choice
        text: &'a str,
        /// True if the menu is open
        expanded: bool,
    },
    /// A window
    Window,
    /// The special bar at the top of a window titling contents and usually embedding window controls
    TitleBar,
}

/// A copy-on-write text value or a reference to another source
pub enum TextOrSource<'a> {
    /// Borrowed text
    Borrowed(&'a str),
    /// Owned text
    Owned(String),
    /// A reference to another widget able to a text value
    ///
    /// It is expected that the given [`Id`] refers to a widget with role
    /// [`Role::Label`] or [`Role::TextLabel`].
    Source(Id),
}

impl<'a> From<&'a str> for TextOrSource<'a> {
    #[inline]
    fn from(text: &'a str) -> Self {
        Self::Borrowed(text)
    }
}

impl From<String> for TextOrSource<'static> {
    #[inline]
    fn from(text: String) -> Self {
        Self::Owned(text)
    }
}

impl<'a> From<&'a String> for TextOrSource<'a> {
    #[inline]
    fn from(text: &'a String) -> Self {
        Self::Borrowed(text)
    }
}

impl From<Id> for TextOrSource<'static> {
    #[inline]
    fn from(id: Id) -> Self {
        Self::Source(id)
    }
}

#[cfg(feature = "accesskit")]
impl<'a> Role<'a> {
    /// Construct an AccessKit [`Role`] from self
    pub(crate) fn as_accesskit_role(&self) -> accesskit::Role {
        use accesskit::Role as R;

        match self {
            // TODO: do we want to automatically use role GenericContainer?
            // Role::Unknown if has_children => R::GenericContainer,
            Role::Unknown | Role::Grip => R::Unknown,
            Role::Label(_) | Role::AccessLabel(_, _) | Role::TextLabel { .. } => R::Label,
            Role::Button => R::Button,
            Role::CheckBox(_) => R::CheckBox,
            Role::RadioButton(_) => R::RadioButton,
            Role::Tab => R::Tab,
            Role::Border => R::Unknown,
            Role::ScrollRegion { .. } => R::ScrollView,
            Role::ScrollBar { .. } => R::ScrollBar,
            Role::Indicator => R::Unknown,
            Role::Image => R::Image,
            Role::Canvas => R::Canvas,
            Role::TextInput {
                multi_line: false, ..
            } => R::TextInput,
            Role::TextInput {
                multi_line: true, ..
            } => R::MultilineTextInput,
            Role::Slider { .. } => R::Slider,
            Role::SpinButton { .. } => R::SpinButton,
            Role::ProgressBar(_) => R::ProgressIndicator,
            Role::MenuBar => R::MenuBar,
            Role::Menu { .. } => R::Menu,
            Role::ComboBox { .. } => R::ComboBox,
            Role::Window => R::Window,
            Role::TitleBar => R::TitleBar,
        }
    }

    /// Construct an AccessKit [`Node`] from self
    ///
    /// This will set node properties as provided by self, but not those provided by the parent.
    pub(crate) fn as_accesskit_node(&self, tile: &dyn Tile) -> accesskit::Node {
        use crate::cast::Cast;
        use accesskit::Action;

        let mut node = accesskit::Node::new(self.as_accesskit_role());
        node.set_bounds(tile.rect().cast());
        if tile.navigable() {
            node.add_action(Action::Focus);
        }

        match *self {
            Role::Unknown | Role::Border | Role::Grip => (),
            Role::Button | Role::Tab => {
                node.add_action(Action::Click);
            }
            Role::Indicator | Role::Image | Role::Canvas => (),
            Role::MenuBar | Role::Window | Role::TitleBar => (),
            Role::Label(text) | Role::TextLabel { text, .. } => node.set_value(text),
            Role::TextInput { text, .. } => {
                node.add_action(Action::SetValue);
                node.add_action(Action::ReplaceSelectedText);
                node.set_value(text)
            }
            Role::AccessLabel(text, ref key) => {
                node.set_value(text);
                if let Some(text) = key.to_text() {
                    node.set_access_key(text);
                }
            }
            Role::CheckBox(state) | Role::RadioButton(state) => {
                node.add_action(Action::Click);
                node.set_toggled(state.into());
            }
            Role::ScrollRegion { offset, max_offset } => {
                node.add_action(Action::ScrollDown);
                node.add_action(Action::ScrollLeft);
                node.add_action(Action::ScrollRight);
                node.add_action(Action::ScrollUp);
                node.set_scroll_x(offset.0.cast());
                node.set_scroll_y(offset.1.cast());
                node.set_scroll_x_min(0.0);
                node.set_scroll_y_min(0.0);
                node.set_scroll_x_max(max_offset.0.cast());
                node.set_scroll_y_max(max_offset.1.cast());
            }
            Role::ScrollBar {
                direction,
                value,
                max_value,
            } => {
                node.set_orientation(direction.into());
                node.set_numeric_value(value.cast());
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(max_value.cast());
            }
            Role::Slider {
                min,
                max,
                step,
                value,
            }
            | Role::SpinButton {
                min,
                max,
                step,
                value,
            } => {
                node.add_action(Action::SetValue);
                node.add_action(Action::Increment);
                node.add_action(Action::Decrement);
                if min.is_finite() {
                    node.set_min_numeric_value(min);
                }
                if max.is_finite() {
                    node.set_max_numeric_value(max);
                }
                if step.is_finite() {
                    node.set_numeric_value_step(step);
                }
                node.set_numeric_value(value);
            }
            Role::ProgressBar(value) => {
                node.set_max_numeric_value(1.0);
                node.set_numeric_value(value.cast());
            }
            Role::ComboBox { expanded, .. } | Role::Menu { expanded } => {
                node.add_action(Action::Expand);
                node.add_action(Action::Collapse);
                node.set_expanded(expanded);
            }
        }

        node
    }
}

/// Context through which additional role properties may be specified
///
/// Unlike other widget method contexts, this is a trait; the caller provides an
/// implementation.
pub trait RoleCx {
    /// Attach a label
    ///
    /// Do not use this for [`Role::Label`] and similar items where the label is
    /// the widget's primary value. Do use this where a label exists which is
    /// not the primary value, for example an image's alternate text or a label
    /// next to a control.
    fn set_label_impl(&mut self, label: TextOrSource<'_>);
}

/// Convenience methods over a [`RoleCx`]
pub trait RoleCxExt: RoleCx {
    /// Attach a label
    ///
    /// Do not use this for [`Role::Label`] and similar items where the label is
    /// the widget's primary value. Do use this where a label exists which is
    /// not the primary value, for example an image's alternate text or a label
    /// next to a control.
    fn set_label<'a>(&mut self, label: impl Into<TextOrSource<'a>>) {
        self.set_label_impl(label.into());
    }
}

impl<C: RoleCx + ?Sized> RoleCxExt for C {}
