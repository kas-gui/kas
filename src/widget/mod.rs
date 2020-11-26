// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget library
//!
//! Unlike the rest of the `kas` crate, this module is not infrastructure but
//! merely a library of useful widgets. It may be moved to a new crate in the
//! future. Any implementation can be directly copied into user code if desired.
//!
//! ## Dialogs
//!
//! -   [`MessageBox`]: a simple window with a message and an "Ok" button
//!
//! ## Container widgets
//!
//! -   [`Frame`]: a simple frame around a single child
//! -   [`ScrollRegion`]: may be larger on the inside than the outside
//! -   [`Stack`]: a stack of widgets in the same rect (TODO: `TabbedStack`)
//! -   [`List`]: a dynamic row / column of children
//! -   [`Splitter`]: similar to [`List`] but with resizing handles
//! -   [`Window`] is usually the root widget and has special handling for
//!     pop-ups and callbacks
//!
//! ## Menus
//!
//! -   [`ComboBox`]: a simple pop-up selector
//! -   [`MenuBar`], [`SubMenu`]: menu parent widgets
//! -   [`MenuEntry`], [`MenuToggle`], [`Separator`]: menu entries
//! -   [`MenuFrame`]: edges of a pop-up menu
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

mod button;
mod checkbox;
mod combobox;
mod dialog;
mod drag;
mod editbox;
mod filler;
mod frame;
mod label;
mod list;
mod menu;
mod progress;
mod radiobox;
mod scroll;
mod scrollbar;
mod separator;
mod slider;
mod splitter;
mod stack;
mod window;

pub use button::TextButton;
pub use checkbox::{CheckBox, CheckBoxBare};
pub use combobox::ComboBox;
pub use dialog::MessageBox;
pub use drag::DragHandle;
pub use editbox::{EditBox, EditBoxVoid, EditGuard};
pub use filler::Filler;
pub use frame::Frame;
pub use label::{AccelLabel, Label, StrLabel, StringLabel};
pub use list::*;
pub use menu::*;
pub use progress::ProgressBar;
pub use radiobox::{RadioBox, RadioBoxBare};
pub use scroll::ScrollRegion;
pub use scrollbar::ScrollBar;
pub use separator::Separator;
pub use slider::{Slider, SliderType};
pub use splitter::*;
pub use stack::{BoxStack, RefStack, Stack};
pub use window::Window;
