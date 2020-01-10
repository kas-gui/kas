// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widgets
//!
//! KAS provides these common widget types for convenience.
//! All these widgets can be implemented in user-code.

mod button;
mod checkbox;
mod dialog;
mod list;
mod scroll;
mod scrollbar;
mod text;
mod window;

pub use button::TextButton;
pub use checkbox::CheckBox;
pub use dialog::MessageBox;
pub use list::{BoxColumn, BoxList, BoxRow, Column, List, Row};
pub use scroll::ScrollRegion;
pub use scrollbar::{ScrollBar, ScrollBarRegion};
pub use text::{EditBox, Label};
pub use window::Window;
