// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS widget library
//!
//! ## Dialogs / pre-made windows
//!
//! See [`dialog`] module.
//!
//! ## Container widgets
//!
//! -   [`Frame`]: a simple frame around a single child
//! -   [`ScrollRegion`]: may be larger on the inside than the outside
//! -   [`Stack`]: a stack of widgets in the same rect (TODO: `TabbedStack`)
//! -   [`List`]: a dynamic row / column of children
//! -   [`Splitter`]: similar to [`List`] but with resizing handles
//!
//! ## Menus
//!
//! See the [`menu`] module.
//!
//! ## Controls
//!
//! -   [`TextButton`]: a simple button
//! -   [`CheckBox`]: a checkable box
//! -   [`RadioBox`]: a checkable box bound to a group
//! -   [`EditBox`]: a text-editing box
//! -   [`ScrollBar`]: a scrollbar
//! -   [`Slider`]: a slider
//!
//! ## Static widgets
//!
//! -   [`Filler`]: an empty widget, sometimes used to fill space
//! -   [`Separator`]: a visible bar to separate things
//! -   [`Label`]: a simple text label
//!
//! ## Components
//!
//! -   [`AccelLabel`]: a label which parses accelerator keys
//! -   [`CheckBoxBare`]: `CheckBox` without its label
//! -   [`RadioBoxBare`]: `RadioBox` without its label
//! -   [`DragHandle`]: a handle (e.g. for a slider, splitter or scrollbar)

// Use ``never_loop`` until: https://github.com/rust-lang/rust-clippy/issues/7397 is fixed
#![allow(
    clippy::or_fun_call,
    clippy::never_loop,
    clippy::comparison_chain,
    clippy::needless_late_init,
    clippy::collapsible_else_if,
    clippy::len_zero
)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "min_spec", feature(min_specialization))]

mod button;
mod checkbox;
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
#[macro_use]
mod macros;
mod mark;
pub mod menu;
mod nav_frame;
mod progress;
mod radiobox;
mod scroll;
mod scroll_label;
mod scrollbar;
mod separator;
mod slider;
mod splitter;
mod stack;

pub mod adapter;
pub mod view;

pub use crate::image::Image;
pub use button::{Button, TextButton};
pub use checkbox::{CheckBox, CheckBoxBare};
pub use combobox::ComboBox;
pub use drag::DragHandle;
pub use edit_field::{EditBox, EditField, EditGuard};
pub use filler::Filler;
pub use frame::{Frame, PopupFrame};
pub use grid::{BoxGrid, Grid};
pub use label::{AccelLabel, Label, StrLabel, StringLabel};
pub use list::*;
pub use mark::Mark;
pub use nav_frame::{NavFrame, SelectMsg};
pub use progress::ProgressBar;
pub use radiobox::{RadioBox, RadioBoxBare, RadioBoxGroup};
pub use scroll::ScrollRegion;
pub use scroll_label::ScrollLabel;
pub use scrollbar::{ScrollBar, ScrollBarRegion, ScrollBars, Scrollable};
pub use separator::Separator;
pub use slider::{Slider, SliderType};
pub use splitter::*;
pub use stack::{BoxStack, RefStack, Stack};
