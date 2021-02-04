// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

use super::shortcuts::Shortcuts;

/// Event handling configuration
#[derive(Debug)]
pub struct Config {
    pub shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        let mut shortcuts = Shortcuts::new();
        shortcuts.load_platform_defaults();
        Config { shortcuts }
    }
}
