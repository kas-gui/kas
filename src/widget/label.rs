// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Manager, ManagerState};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;

/// A simple text label
#[widget_config]
#[handler]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[widget_core]
    core: CoreData,
    align: (Align, Align),
    reserve: Option<&'static str>,
    text: CowString,
}

impl Layout for Label {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let text = self.reserve.unwrap_or(&self.text);
        let rules = size_handle.text_bound(text, TextClass::Label, axis);
        if axis.is_horizontal() {
            self.core.rect.size.0 = rules.ideal_size();
        } else {
            self.core.rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.align = (
            align.horiz.unwrap_or(Align::Begin),
            align.vert.unwrap_or(Align::Centre),
        );
        self.core.rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState) {
        draw_handle.text(self.core.rect, &self.text, TextClass::Label, self.align);
    }
}

impl Label {
    /// Construct a new, empty instance
    pub fn new<T: Into<CowString>>(text: T) -> Self {
        Label {
            core: Default::default(),
            align: Default::default(),
            reserve: None,
            text: text.into(),
        }
    }

    /// Reserve sufficient room for the given text
    ///
    /// If this option is used, the label will be sized to fit this text, not
    /// the actual text.
    pub fn reserve(mut self, text: &'static str) -> Self {
        self.reserve = Some(text);
        self
    }
}

impl HasText for Label {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_cow_string(&mut self, mgr: &mut Manager, text: CowString) {
        self.text = text.into();
        mgr.redraw(self.id());
    }
}
