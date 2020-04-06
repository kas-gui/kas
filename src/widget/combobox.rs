// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use std::fmt::Debug;
use std::iter::FromIterator;

use super::{Column, TextButton};
use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Action, Event, Manager, Response, UpdateHandle};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;
use kas::WindowId;

/// A pop-up multiple choice menu
#[handler(event)]
#[widget(config=noauto)]
#[derive(Clone, Debug, Widget)]
pub struct ComboBox<M: Clone + Debug + 'static> {
    #[widget_core]
    core: CoreData,
    // text_rect: Rect,
    column: Column<TextButton<u64>>,
    messages: Vec<M>, // TODO: is this a useless lookup step?
    active: usize,
    handle: UpdateHandle,
    popup: Option<WindowId>,
}

impl<M: Clone + Debug + 'static> kas::WidgetConfig for ComboBox<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.update_on_handle(self.handle, self.id());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug + 'static> kas::Layout for ComboBox<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), sides.0 + sides.1, margins);

        // TODO: should we calculate a bound over all choices or assume some default?
        let content_rules = size_handle.text_bound(self.text(), TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, rect: Rect, _align: kas::AlignHints) {
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
    fn new_(column: Vec<TextButton<u64>>, messages: Vec<M>) -> Self {
        assert!(column.len() > 0, "ComboBox: expected at least one choice");
        ComboBox {
            core: Default::default(),
            column: Column::new(column),
            messages,
            active: 0,
            handle: UpdateHandle::new(),
            popup: None,
        }
    }

    /// Get the text of the active choice
    pub fn text(&self) -> &str {
        self.column[self.active].get_text()
    }

    /// Add a choice to the combobox, in last position
    pub fn push<T: Into<CowString>>(&mut self, label: CowString, msg: M) -> TkAction {
        let len = self.column.len() as u64;
        self.messages.push(msg);
        self.column.push(TextButton::new(label, len))
    }
}

impl<T: Into<CowString>, M: Clone + Debug> FromIterator<(T, M)> for ComboBox<M> {
    fn from_iter<I: IntoIterator<Item = (T, M)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let len = iter.size_hint().1.unwrap_or(0);
        let mut choices = Vec::with_capacity(len);
        let mut messages = Vec::with_capacity(len);
        for (i, (label, msg)) in iter.enumerate() {
            choices.push(TextButton::new(label, i as u64));
            messages.push(msg);
        }
        ComboBox::new_(choices, messages)
    }
}

impl<'a, M: Clone + Debug + 'static> FromIterator<&'a (&'static str, M)> for ComboBox<M> {
    fn from_iter<I: IntoIterator<Item = &'a (&'static str, M)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let len = iter.size_hint().1.unwrap_or(0);
        let mut choices = Vec::with_capacity(len);
        let mut messages = Vec::with_capacity(len);
        for (i, (label, msg)) in iter.enumerate() {
            choices.push(TextButton::new(*label, i as u64));
            messages.push(msg.clone());
        }
        ComboBox::new_(choices, messages)
    }
}

impl<M: Clone + Debug + 'static> event::Handler for ComboBox<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn action(&mut self, mgr: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => {
                if let Some(id) = self.popup {
                    mgr.close_window(id);
                    self.popup = None;
                } else {
                    let id = mgr.add_popup(kas::Popup {
                        parent: self.id(),
                        direction: Direction::Down,
                        overlay: Box::new(ComboPopup::new(self.column.clone(), self.handle)),
                    });
                    self.popup = Some(id);
                }
                Response::None
            }
            Action::HandleUpdate { payload, .. } => {
                let index = payload as usize;
                assert!(index < self.column.len());
                self.active = index;
                if let Some(id) = self.popup {
                    mgr.close_window(id);
                    self.popup = None;
                }
                mgr.redraw(self.id());
                Response::Msg(self.messages[index].clone())
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

#[layout(single)]
#[handler(action)]
#[derive(Clone, Debug, Widget)]
struct ComboPopup {
    #[widget_core]
    core: CoreData,
    #[widget]
    column: Column<TextButton<u64>>,
    handle: UpdateHandle,
}

impl ComboPopup {
    #[inline]
    fn new(column: Column<TextButton<u64>>, handle: UpdateHandle) -> Self {
        ComboPopup {
            core: Default::default(),
            column,
            handle,
        }
    }
}

impl event::EventHandler for ComboPopup {
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if id <= self.column.id() {
            match self.column.event(mgr, id, event).try_into() {
                Ok(r) => r,
                Err(msg) => {
                    mgr.trigger_update(self.handle, msg);
                    mgr.send_action(TkAction::Close);
                    Response::None
                }
            }
        } else {
            Response::Unhandled(event)
        }
    }
}
