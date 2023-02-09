// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS widget library
//!
//! ## Sub-modules
//!
//! -   [`adapter`] provides the [`AdaptWidget`](adapter::AdaptWidget) trait with `map_msg`, `with_reserve` and `with_label` methods
//! -   [`dialog`] provides [`MessageBox`](dialog::MessageBox), a simple [`Window`](dialog::Window), ...
//! -   [`edit`] provides [`EditBox`], [`EditField`] widgets, [`EditGuard`] trait and some impls
//! -   [`menu`] provides a [`MenuBar`](menu::MenuBar), [`SubMenu`](menu::SubMenu), ...
//!
//! ## Container widgets
//!
//! -   [`Frame`], [`NavFrame`], [`PopupFrame`]: frames around content
//! -   [`ScrollRegion`], [`ScrollBarRegion`]: larger on the inside
//! -   [`Stack`], [`TabStack`]: a stack of widgets in the same rect
//! -   [`List`]: a row / column of children
//! -   [`Splitter`]: like [`List`] but with resizing handles
//!
//! ## Controls
//!
//! -   [`TextButton`], [`Button`], [`MarkButton`]: button widgets
//! -   [`CheckBox`], [`CheckButton`]: checkable boxes
//! -   [`RadioBox`], [`RadioButton`]: linked checkable boxes
//! -   [`ScrollBar`]: a scroll bar
//! -   [`Slider`]: a slider
//! -   [`Spinner`]: numeric entry
//!
//! ## Displays
//!
//! -   [`Filler`]: an empty widget, sometimes used to fill space
//! -   [`Separator`]: a visible bar to separate things
//! -   [`Mark`]: a small mark
//! -   [`Label`]: a simple text label
//! -   [`ScrollLabel`]: text label supporting scrolling and selection
//! -   [`Image`]: a pixmap image
//! -   [`ProgressBar`]: show completion level
//!
//! ## Components
//!
//! -   [`AccelLabel`]: a label which parses accelerator keys
//! -   [`GripPart`]: a handle (e.g. for a slider, splitter or scroll_bar)

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "min_spec", feature(min_specialization))]

mod button;
mod check_box;
mod combobox;
pub mod dialog;
pub mod edit;
mod filler;
mod frame;
mod grid;
mod grip;
mod image;
mod label;
mod list;
mod mark;
pub mod menu;
mod nav_frame;
mod progress;
mod radio_box;
mod scroll;
mod scroll_bar;
mod scroll_label;
mod separator;
mod slider;
mod spinner;
mod splitter;
mod stack;
mod tab_stack;
mod title_bar;

pub mod adapter;

pub use crate::image::Image;
pub use button::{Button, TextButton};
pub use check_box::{CheckBox, CheckButton};
pub use combobox::ComboBox;
pub use edit::{EditBox, EditField, EditGuard};
pub use filler::Filler;
pub use frame::{Frame, PopupFrame};
pub use grid::{BoxGrid, Grid};
pub use grip::{GripMsg, GripPart};
pub use label::{AccelLabel, Label, StrLabel, StringLabel};
pub use list::*;
pub use mark::{Mark, MarkButton};
pub use nav_frame::{NavFrame, SelectMsg};
pub use progress::ProgressBar;
pub use radio_box::{RadioBox, RadioButton, RadioGroup};
pub use scroll::ScrollRegion;
pub use scroll_bar::{ScrollBar, ScrollBarRegion, ScrollBars, ScrollMsg};
pub use scroll_label::ScrollLabel;
pub use separator::Separator;
pub use slider::{Slider, SliderValue};
pub use spinner::{Spinner, SpinnerValue};
pub use splitter::*;
pub use stack::{BoxStack, RefStack, Stack};
pub use tab_stack::{BoxTabStack, Tab, TabStack};
