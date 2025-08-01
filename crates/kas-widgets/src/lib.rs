// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS widget library
//!
//! ## Complex widgets
//!
//! -   [`EventConfig`] provides an editor for event configuration
//! -   [`TitleBar`] is a window title-bar (including buttons)
//! -   [`TitleBarButtons`] is the standard minimize/maximize/close button cluster on a title-bar
//!
//! ## Sub-modules
//!
//! -   [`adapt`] provides [`Adapt`], [`AdaptWidget`], [`AdaptWidgetAny`] and supporting items
//!     (the items mentioned are re-export here).
//! -   [`dialog`] provides [`MessageBox`](dialog::MessageBox), ...
//! -   [`edit`] provides [`EditBox`], [`EditField`] widgets, [`EditGuard`] trait and some impls
//! -   [`menu`] provides a [`MenuBar`](menu::MenuBar), [`SubMenu`](menu::SubMenu), ...
//!
//! ## Container widgets
//!
//! -   [`Frame`]: a frame around content
//! -   [`ScrollRegion`], [`ScrollBarRegion`]: larger on the inside
//! -   [`Stack`], [`TabStack`]: a stack of widgets in the same rect
//! -   [`List`]: a row / column of children
//! -   [`Splitter`]: like [`List`] but with resizing handles
//! -   [`Grid`]: a container using grid layout
//!
//! ## Controls
//!
//! -   [`Button`], [`MarkButton`]: button widgets
//! -   [`CheckBox`], [`CheckButton`]: checkable boxes
//! -   [`RadioBox`], [`RadioButton`]: linked checkable boxes
//! -   [`ComboBox`]: a drop-down menu over a list
//! -   [`ScrollBar`]: a scroll bar; [`ScrollBars`]: a wrapper adding scroll
//!     bars around an inner widget
//! -   [`Slider`]: a slider
//! -   [`SpinBox`]: numeric entry
//!
//! ## Displays
//!
//! -   [`Filler`]: an empty widget, sometimes used to fill space
//! -   [`Image`]: a pixmap image
//! -   [`Label`], [`AccessLabel`]: are static text labels
//! -   [`Text`]: a dynamic (input-data derived) text label
//! -   [`Mark`]: a small mark
//! -   [`ScrollLabel`]: static text label supporting scrolling and selection
//! -   [`ScrollText`]: dynamic text label supporting scrolling and selection
//! -   [`Separator`]: a visible bar to separate things
//! -   [`format_value`] and [`format_data`] are constructors for [`Text`],
//!     displaying a text label derived from input data
//! -   [`ProgressBar`]: show completion level
//!
//! ## Components
//!
//! -   [`AccessLabel`]: a label which parses access keys
//! -   [`GripPart`]: a handle (e.g. for a slider, splitter or scroll_bar)

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod adapt;
#[doc(no_inline)]
pub use adapt::{Adapt, AdaptWidget, AdaptWidgetAny};

mod button;
mod check_box;
mod combobox;
pub mod dialog;
pub mod edit;
mod event_config;
mod filler;
mod float;
mod frame;
mod grid;
mod grip;
mod image;
mod label;
mod list;
mod mark;
pub mod menu;
mod progress;
mod radio_box;
mod scroll;
mod scroll_bar;
mod scroll_label;
mod separator;
mod slider;
mod spin_box;
mod splitter;
mod stack;
mod tab_stack;
mod text;

#[doc(inline)] pub use kas::widgets::*;

pub use crate::image::Image;
#[cfg(feature = "image")] pub use crate::image::ImageError;
pub use button::Button;
pub use check_box::{CheckBox, CheckButton};
pub use combobox::ComboBox;
pub use edit::{EditBox, EditField, EditGuard};
pub use event_config::EventConfig;
pub use filler::Filler;
pub use float::Float;
pub use frame::Frame;
pub use grid::Grid;
pub use grip::{GripMsg, GripPart};
pub use label::{AccessLabel, Label};
pub use list::*;
pub use mark::{Mark, MarkButton};
pub use progress::ProgressBar;
pub use radio_box::{RadioBox, RadioButton};
pub use scroll::ScrollRegion;
pub use scroll_bar::{ScrollBar, ScrollBarMode, ScrollBarRegion, ScrollBars, ScrollMsg};
pub use scroll_label::{ScrollLabel, ScrollText, SelectableLabel, SelectableText};
pub use separator::Separator;
pub use slider::{Slider, SliderValue};
pub use spin_box::{SpinBox, SpinValue};
pub use splitter::Splitter;
pub use stack::{BoxStack, Stack};
pub use tab_stack::{BoxTabStack, Tab, TabStack};
pub use text::Text;
