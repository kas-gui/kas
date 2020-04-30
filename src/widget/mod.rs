// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widgets
//!
//! KAS provides these common widgets for convenience, although there is no
//! reason they cannot be implemented in user code.

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
pub use label::{AccelLabel, Label};
pub use list::*;
pub use menu::*;
pub use radiobox::{RadioBox, RadioBoxBare};
pub use scroll::ScrollRegion;
pub use scrollbar::ScrollBar;
pub use separator::Separator;
pub use slider::Slider;
pub use splitter::*;
pub use stack::{BoxStack, RefStack, Stack};
pub use window::Window;
