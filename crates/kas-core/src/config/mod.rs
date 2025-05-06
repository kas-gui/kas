// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration items and utilities

mod config;
pub use config::{Config, ConfigMsg, WindowConfig};

mod event;
pub use event::{EventConfig, EventConfigMsg, EventWindowConfig, MousePan};

mod font;
pub use font::{FontConfig, FontConfigMsg, RasterConfig};

mod format;
pub use format::{Error, Format};

mod factory;
pub use factory::{ConfigFactory, ConfigMode};

mod shortcuts;
pub use shortcuts::Shortcuts;

mod theme;
pub use theme::{ThemeConfig, ThemeConfigMsg};
