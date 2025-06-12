// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filler widget

use kas::prelude::*;

#[impl_self]
mod Filler {
    /// A space filler
    ///
    /// This widget has zero minimum size but can expand according to the given
    /// stretch priority.
    #[derive(Clone, Debug, Default)]
    #[widget(type Data = ())]
    pub struct Filler {
        core: widget_core!(),
        horiz: Stretch,
        vert: Stretch,
    }

    impl Layout for Filler {
        fn size_rules(&mut self, _: SizeCx, axis: AxisInfo) -> SizeRules {
            let stretch = if axis.is_horizontal() { self.horiz } else { self.vert };
            SizeRules::empty(stretch)
        }

        fn draw(&self, _: DrawCx) {}
    }
}

impl Filler {
    /// Construct a filler with priority [`Stretch::Filler`]
    pub fn new() -> Self {
        Filler::with(Stretch::Filler)
    }

    /// Construct a filler with priority [`Stretch::Low`]
    pub fn low() -> Self {
        Filler::with(Stretch::Low)
    }

    /// Construct a filler with priority [`Stretch::High`]
    pub fn high() -> Self {
        Filler::with(Stretch::High)
    }

    /// Construct a filler with priority [`Stretch::Maximize`]
    pub fn maximize() -> Self {
        Filler::with(Stretch::Maximize)
    }

    /// Construct with a custom stretch priority
    pub fn with(stretch: Stretch) -> Self {
        Filler::with_hv(stretch, stretch)
    }

    /// Construct with custom horizontal and vertical priorities
    pub fn with_hv(horiz: Stretch, vert: Stretch) -> Self {
        let core = Default::default();
        Filler { core, horiz, vert }
    }
}
