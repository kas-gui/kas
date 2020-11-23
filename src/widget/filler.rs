// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filler widget

use kas::{event, prelude::*};

/// A space filler
///
/// This widget has zero minimum size but can expand according to the given
/// stretch policy.
#[derive(Clone, Debug, Default, Widget)]
pub struct Filler {
    #[widget_core]
    core: CoreData,
    policy: StretchPolicy,
}

impl Layout for Filler {
    fn size_rules(&mut self, _: &mut dyn SizeHandle, _: AxisInfo) -> SizeRules {
        SizeRules::empty(self.policy)
    }

    fn draw(&self, _: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {}
}

impl Filler {
    /// Construct a filler with policy [`StretchPolicy::Filler`]
    pub fn new() -> Self {
        Filler::with_policy(StretchPolicy::Filler)
    }

    /// Construct a filler with policy [`StretchPolicy::Maximize`]
    pub fn maximize() -> Self {
        Filler::with_policy(StretchPolicy::Maximize)
    }

    /// Construct with a custom stretch policy
    pub fn with_policy(policy: StretchPolicy) -> Self {
        Filler {
            core: Default::default(),
            policy,
        }
    }
}
