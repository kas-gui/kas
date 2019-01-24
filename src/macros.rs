// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Library macros
//! 
//! KAS's widget specification is complex, and parts of it are for internal use
//! only. This means the only way users should implement the [`Widget`] trait
//! and base-traits is via the [`derive(Widget)`] macro.
//! 
//! Additionally, the following convenience macros are available:
//! 
//! -  [`make_widget`] creates a custom anonymous widget type and produces
//!     an implementation of that type
//! -  [`derive(NoResponse)`] implements `From<NoResponse>` for the deriving
//!     type
//! 
//! Note that these are re-exports from the `kas-macros` crate. Users should
//! consider the `kas-macros` crate an implementation detail and not use it
//! directly.
//! 
//! [`make_widget`]: kas_macros::make_widget
//! [`derive(Widget)`]: kas_macros::Widget
//! [`derive(NoResponse)`]: kas_macros::NoResponse

pub use kas_macros::{NoResponse, Widget, make_widget};
