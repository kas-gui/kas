// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration items and utilities

mod config;
pub use config::{Config, WindowConfig};

pub mod event;

mod format;
pub use format::{Error, Format};

mod options;
pub use options::{ConfigMode, Options};

mod shortcuts;
pub use shortcuts::Shortcuts;

pub mod theme;
