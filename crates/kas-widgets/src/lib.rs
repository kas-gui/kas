// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS widget library
//!
//! ## Sub-modules
//!
//! -   [`adapter::AdaptWidget`]: provides `map_msg`, `with_reserve` and `with_label` methods
//! -   [`dialog`]: pre-made dialog widgets
//! -   [`menu`]: support for pop-up menus ([`ComboBox`], [`menu::MenuBar`])
//! -   [`view`]: data models
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
//! -   [`EditBox`], [`EditField`]: text editing with/without a frame
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
//! -   [`DragHandle`]: a handle (e.g. for a slider, splitter or scroll_bar)

// Use ``never_loop`` until: https://github.com/rust-lang/rust-clippy/issues/7397 is fixed
#![allow(
    clippy::or_fun_call,
    clippy::never_loop,
    clippy::comparison_chain,
    clippy::needless_late_init,
    clippy::collapsible_else_if,
    clippy::len_zero
)]
#![allow(clippy::type_complexity)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "min_spec", feature(min_specialization))]

mod button;
mod check_box;
mod combobox;
pub mod dialog;
mod drag;
mod edit_field;
mod filler;
mod frame;
mod grid;
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

pub mod adapter;
pub mod view;

pub use crate::image::Image;
pub use button::{Button, TextButton};
pub use check_box::{CheckBox, CheckButton};
pub use combobox::ComboBox;
pub use drag::DragHandle;
pub use edit_field::{EditBox, EditField, EditGuard};
pub use filler::Filler;
pub use frame::{Frame, PopupFrame};
pub use grid::{BoxGrid, Grid};
pub use label::{AccelLabel, Label, StrLabel, StringLabel};
pub use list::*;
pub use mark::{Mark, MarkButton};
pub use nav_frame::{NavFrame, SelectMsg};
pub use progress::ProgressBar;
pub use radio_box::{RadioBox, RadioButton, RadioGroup};
pub use scroll::ScrollRegion;
pub use scroll_bar::{ScrollBar, ScrollBarRegion, ScrollBars};
pub use scroll_label::ScrollLabel;
pub use separator::Separator;
pub use slider::{Slider, SliderType};
pub use spinner::{Spinner, SpinnerType};
pub use splitter::*;
pub use stack::{BoxStack, RefStack, Stack};
pub use tab_stack::{BoxTabStack, Tab, TabStack};
