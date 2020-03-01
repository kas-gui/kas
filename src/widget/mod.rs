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
mod dialog;
mod editbox;
mod filler;
mod label;
mod list;
mod radiobox;
mod scroll;
mod scrollbar;
mod window;

pub use button::TextButton;
pub use checkbox::{CheckBox, CheckBoxBare};
pub use dialog::MessageBox;
pub use editbox::EditBox;
pub use filler::Filler;
pub use label::Label;
pub use list::{BoxColumn, BoxList, BoxRow, Column, List, Row};
pub use radiobox::{RadioBox, RadioBoxBare};
pub use scroll::ScrollRegion;
pub use scrollbar::ScrollBar;
pub use window::Window;
