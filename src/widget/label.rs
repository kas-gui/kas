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
            prepared = PreparedText::new_multi(s.into());
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
        self.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Default, Align::Centre));
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        draw_handle.text(self.core.rect.pos, &self.label, TextClass::Label);
    }
}

impl Label {
    /// Construct from label text
    pub fn new<T: Into<LabelString>>(label: T) -> Self {
        Label {
            core: Default::default(),
            reserve: None,
            label: PreparedText::new_multi(label.into().deref().into()),
        }
    }

    /// Construct from Markdown
    #[cfg(feature = "markdown")]
    pub fn from_md(text: &str) -> Self {
        let text = kas::text::rich::Text::from_md(text);
        Label {
            core: Default::default(),
            reserve: None,
            label: PreparedText::new_multi(text),
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
        self.label.set_and_prepare(text)
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
    underline: usize,
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
        self.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Default, Align::Centre));
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, _: bool) {
        if mgr.show_accel_labels() {
            draw_handle.text_with_underline(
                self.core.rect.pos,
                Coord::ZERO,
                &self.label,
                TextClass::Label,
                self.underline,
            );
        } else {
            draw_handle.text(self.core.rect.pos, &self.label, TextClass::Label);
        }
    }
}

impl AccelLabel {
    /// Construct a new, empty instance
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        let label = label.into();
        let text = PreparedText::new_single(label.text().into());
        let underline = label.underline();
        let keys = label.take_keys();
        AccelLabel {
            core: Default::default(),
            keys,
            label: text,
            underline,
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
        let text = label.text().to_string();
        self.keys = label.take_keys();
        self.label.set_and_prepare(text)
    }
}
