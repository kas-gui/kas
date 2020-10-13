// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::class::{HasFormatted, HasString, SetAccel};
use kas::draw::TextClass;
use kas::event::VirtualKeyCodes;
use kas::prelude::*;

/// A simple text label
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[widget_core]
    core: CoreData,
    reserve: Option<FormattedString>,
    label: Text,
}

impl Layout for Label {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut prepared;
        let text = if let Some(ref s) = self.reserve {
            prepared = Text::new_multi(s.clone());
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
    pub fn new<T: Into<FormattedString>>(label: T) -> Self {
        Label {
            core: Default::default(),
            reserve: None,
            label: Text::new_multi(label.into()),
        }
    }

    /// Construct from Markdown
    #[cfg(feature = "markdown")]
    pub fn from_md(text: &str) -> Self {
        let text = kas::text::parser::Markdown::new(text);
        Label::from(FormattedString::from(text))
    }
}

impl From<FormattedString> for Label {
    fn from(text: FormattedString) -> Self {
        Label {
            core: Default::default(),
            reserve: None,
            label: Text::new_multi(text),
        }
    }
}

impl Label {
    /// Reserve sufficient room for the given text
    ///
    /// If this option is used, the label will be sized to fit this text, not
    /// the actual text.
    pub fn reserve<A: Into<FormattedString>>(mut self, text: A) -> Self {
        self.reserve = Some(text.into());
        self
    }
}

impl HasString for Label {
    fn get_str(&self) -> &str {
        self.label.text()
    }

    fn set_string(&mut self, text: String) -> TkAction {
        self.label.set_and_prepare(text)
    }
}

impl HasFormatted for Label {
    fn get_formatted(&self) -> FormattedString {
        self.label.clone_text()
    }

    fn set_formatted_string(&mut self, text: FormattedString) -> TkAction {
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
    label: Text,
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
                self.label.as_ref(),
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
        let text = Text::new_single(label.text().into());
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

impl HasString for AccelLabel {
    fn get_str(&self) -> &str {
        self.label.text()
    }

    fn set_string(&mut self, text: String) -> TkAction {
        self.keys.clear();
        self.label.set_and_prepare(text)
    }
}

impl SetAccel for AccelLabel {
    fn set_accel_string(&mut self, label: AccelString) -> TkAction {
        let text = label.text().to_string();
        self.keys = label.take_keys();
        self.label.set_and_prepare(text)
    }
}
