// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use std::fmt::Debug;
use std::iter::FromIterator;

use super::{Column, TextButton};
use crate::class::HasText;
use crate::draw::{DrawHandle, SizeHandle, TextClass};
use crate::event::{self, Action, Manager, Response};
use crate::geom::Rect;
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{Align, AlignHints, CoreData, CowString, Layout, WidgetCore};

/// A pop-up multiple choice menu
#[widget_config(key_nav = true)]
#[handler(event)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ComboBox<M: Clone + Debug> {
    #[widget_core]
    core: CoreData,
    // text_rect: Rect,
    choices: Column<TextButton<M>>,
    active: usize,
}

impl<M: Clone + Debug> Layout for ComboBox<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.dir(), sides.0 + sides.1, margins);

        // TODO: should we calculate a bound over all choices or assume some default?
        let content_rules = size_handle.text_bound(self.text(), TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, _: &mut dyn SizeHandle, rect: Rect, _align: AlignHints) {
        self.core.rect = rect;

        // In theory, text rendering should be restricted as in EditBox.
        // In practice, it sometimes overflows a tiny bit, and looks better if
        // we let it overflow. Since the text is centred this is okay.
        // self.text_rect = ...
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        draw_handle.button(self.core.rect, mgr.highlight_state(self.id()));
        let align = (Align::Centre, Align::Centre);
        draw_handle.text(self.core.rect, self.text(), TextClass::Button, align);
    }
}

impl<M: Clone + Debug> ComboBox<M> {
    /// Construct a combobox
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    /// Each choice has some corresponding message of type `M` which is emitted
    /// by the event handler when this choice is selected.
    ///
    /// This constructor may be used with an iterator compatible with any
    /// [`FromIterator`] for `ComboBox`, for example:
    /// ```
    /// # use kas::widget::ComboBox;
    /// let combobox = ComboBox::<i32>::new([("one", 1), ("two", 2), ("three", 3)].iter());
    /// ```
    #[inline]
    pub fn new<T, I: IntoIterator<Item = T>>(iter: I) -> Self
    where
        ComboBox<M>: FromIterator<T>,
    {
        ComboBox::from_iter(iter)
    }

    #[inline]
    fn new_(choices: Vec<TextButton<M>>) -> Self {
        assert!(choices.len() > 0, "ComboBox: expected at least one choice");
        ComboBox {
            core: Default::default(),
            choices: Column::new(choices),
            active: 0,
        }
    }

    /// Get the text of the active choice
    pub fn text(&self) -> &str {
        self.choices[self.active].get_text()
    }

    /// Add a choice to the combobox, in last position
    pub fn push<T: Into<CowString>>(&mut self, mgr: &mut Manager, label: CowString, msg: M) {
        self.choices.push(mgr, TextButton::new(label, msg));
    }
}

impl<T: Into<CowString>, M: Clone + Debug> FromIterator<(T, M)> for ComboBox<M> {
    fn from_iter<I: IntoIterator<Item = (T, M)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut choices = Vec::with_capacity(iter.size_hint().1.unwrap_or(0));
        for (label, msg) in iter {
            choices.push(TextButton::new(label, msg));
        }
        ComboBox::new_(choices)
    }
}

impl<'a, M: Clone + Debug + 'static> FromIterator<&'a (&'static str, M)> for ComboBox<M> {
    fn from_iter<I: IntoIterator<Item = &'a (&'static str, M)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut choices = Vec::with_capacity(iter.size_hint().1.unwrap_or(0));
        for item in iter {
            choices.push(TextButton::new(item.0, item.1.clone()));
        }
        ComboBox::new_(choices)
    }
}

impl<M: Clone + Debug> event::Handler for ComboBox<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn action(&mut self, _: &mut Manager, action: Action) -> Response<M> {
        match action {
            // TODO
            Action::Activate => Response::None,
            a @ _ => Response::unhandled_action(a),
        }
    }
}
