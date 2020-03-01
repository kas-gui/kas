// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use crate::class::HasText;
use crate::draw::{DrawHandle, SizeHandle, TextClass};
use crate::event::{Manager, ManagerState};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{Align, AlignHints, CoreData, Layout, WidgetCore};
use kas::geom::Rect;

/// A simple text label
#[widget]
#[handler]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[core]
    core: CoreData,
    align: (Align, Align),
    text: String,
}

impl Layout for Label {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let rules = size_handle.text_bound(&self.text, TextClass::Label, axis);
        if axis.is_horizontal() {
            self.core_data_mut().rect.size.0 = rules.ideal_size();
        } else {
            self.core_data_mut().rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.align = (
            align.horiz.unwrap_or(Align::Begin),
            align.vert.unwrap_or(Align::Centre),
        );
        self.core_data_mut().rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState) {
        draw_handle.text(self.core.rect, &self.text, TextClass::Label, self.align);
    }
}

impl Label {
    /// Construct a new, empty instance
    pub fn new<T: ToString>(text: T) -> Self {
        Label {
            core: Default::default(),
            align: Default::default(),
            text: text.to_string(),
        }
    }
}

impl<T> From<T> for Label
where
    String: From<T>,
{
    fn from(text: T) -> Self {
        Label {
            core: Default::default(),
            align: Default::default(),
            text: String::from(text),
        }
    }
}

impl HasText for Label {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_string(&mut self, mgr: &mut Manager, text: String) {
        self.text = text;
        mgr.redraw(self.id());
    }
}
