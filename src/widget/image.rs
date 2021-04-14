// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Image widget

use kas::{event, prelude::*};
use std::path::PathBuf;

/// An image with margins
///
/// TODO: `BareImage` variant without margins
#[derive(Clone, Debug, Default, Widget)]
#[widget(config = noauto)]
pub struct Image {
    #[widget_core]
    core: CoreData,
    path: PathBuf,
    do_load: bool,
}

impl Image {
    /// Construct with a path
    ///
    /// TODO: low level variant allowing use of an existing image resource?
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Image {
            core: Default::default(),
            path: path.into(),
            do_load: true,
        }
    }
}

impl WidgetConfig for Image {
    fn configure(&mut self, mgr: &mut Manager) {
        if self.do_load {
            mgr.size_handle(|sh| sh.load_image(&self.path));
            self.do_load = false;
        }
    }
}

impl Layout for Image {
    fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = sh.image().unwrap_or(Size::ZERO);
        let margins = sh.outer_margins();
        SizeRules::extract_fixed(axis, size, margins)
    }

    fn draw(&self, draw: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        draw.image(self.rect());
    }
}
