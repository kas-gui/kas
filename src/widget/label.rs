// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::class::{CloneText, SetAccel, SetText};
use kas::draw::TextClass;
use kas::event::VirtualKeyCodes;
use kas::prelude::*;
use std::ops::Deref;

/// A simple text label
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[widget_core]
    core: CoreData,
    reserve: Option<&'static str>,
    label: PreparedText,
}

impl Layout for Label {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut prepared;
        let text = if let Some(s) = self.reserve {
            prepared = PreparedText::new(s.into(), true);
            &mut prepared
        } else {
            &mut self.label
        };
        let rules = size_handle.text_bound(text, TextClass::Label, axis);
        if axis.is_horizontal() {
            self.core.rect.size.0 = rules.ideal_size();
        } else {
            self.core.rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.label.set_size(rect.size);
        self.label.set_alignment(
            align.horiz.unwrap_or(Align::Default),
            align.vert.unwrap_or(Align::Centre),
        );
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        draw_handle.text(self.core.rect.pos, &self.label, TextClass::Label);
    }
}

impl Label {
    /// Construct a new, empty instance
    pub fn new<T: Into<LabelString>>(label: T) -> Self {
        Label {
            core: Default::default(),
            reserve: None,
            label: PreparedText::new(label.into().deref().into(), true),
        }
    }

    /// Reserve sufficient room for the given text
    ///
    /// If this option is used, the label will be sized to fit this text, not
    /// the actual text.
    // TODO: use rich-text model
    pub fn reserve(mut self, text: &'static str) -> Self {
        self.reserve = Some(text);
        self
    }
}

impl CloneText for Label {
    fn clone_text(&self) -> kas::text::RichText {
        self.label.clone_text()
    }
}

impl SetText for Label {
    fn set_rich_text(&mut self, text: kas::text::RichText) -> TkAction {
        self.label.set_text(text)
    }
}

/// A label supporting an accelerator key
///
/// Accelerator keys are not useful on plain labels, but this widget may be
/// embedded within a parent (e.g. `CheckBox` uses this).
#[derive(Clone, Default, Debug, Widget)]
pub struct AccelLabel {
    #[widget_core]
    core: CoreData,
    keys: VirtualKeyCodes,
    label: PreparedText,
}

impl Layout for AccelLabel {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let rules = size_handle.text_bound(&mut self.label, TextClass::Label, axis);
        if axis.is_horizontal() {
            self.core.rect.size.0 = rules.ideal_size();
        } else {
            self.core.rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.label.set_size(rect.size);
        self.label.set_alignment(
            align.horiz.unwrap_or(Align::Default),
            align.vert.unwrap_or(Align::Centre),
        );
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _mgr: &ManagerState, _: bool) {
        // TODO: mgr.show_accel_labels();
        draw_handle.text(self.core.rect.pos, &self.label, TextClass::Label);
    }
}

impl AccelLabel {
    /// Construct a new, empty instance
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        let label = label.into();
        let text = PreparedText::new(label.get(false).into(), false);
        let keys = label.take_keys();
        AccelLabel {
            core: Default::default(),
            keys,
            label: text,
        }
    }

    /// Get the accelerator keys
    pub fn keys(&self) -> &[event::VirtualKeyCode] {
        &self.keys
    }
}

impl CloneText for AccelLabel {
    fn clone_text(&self) -> kas::text::RichText {
        self.label.clone_text()
    }
}

impl SetAccel for AccelLabel {
    fn set_accel_string(&mut self, label: AccelString) -> TkAction {
        let text = label.get(false).to_string();
        self.keys = label.take_keys();
        self.label.set_text(text)
    }
}
