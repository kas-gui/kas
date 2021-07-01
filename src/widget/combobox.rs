// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use std::fmt::{self, Debug};
use std::rc::Rc;

use super::{Column, MenuEntry};
use kas::draw::TextClass;
use kas::event::{self, Command, GrabMode};
use kas::prelude::*;
use kas::WindowId;

/// A pop-up multiple choice menu
///
/// A combobox presents a menu with a fixed set of choices when clicked.
#[derive(Clone, Widget)]
#[widget(config(key_nav = true, hover_highlight = true))]
#[handler(noauto)]
pub struct ComboBox<M: 'static> {
    #[widget_core]
    core: CoreData,
    label: Text<String>,
    frame_size: Size,
    #[widget]
    popup: ComboPopup,
    active: usize,
    opening: bool,
    popup_id: Option<WindowId>,
    on_select: Option<Rc<dyn Fn(&mut Manager, usize) -> Option<M>>>,
}

impl<M: 'static> Debug for ComboBox<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ComboBox")
            .field("core", &self.core)
            .field("label", &self.label)
            .field("frame_size", &self.frame_size)
            .field("popup", &self.popup)
            .field("active", &self.active)
            .field("opening", &self.opening)
            .field("popup_id", &self.popup_id)
            .finish_non_exhaustive()
    }
}

impl<M: 'static> kas::Layout for ComboBox<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_rules = size_handle.button_surround(axis.is_vertical());
        let content_rules = size_handle.text_bound(&mut self.label, TextClass::Button, axis);

        let (rules, _offset, size) = frame_rules.surround(content_rules);
        self.frame_size.set_component(axis, size);
        rules
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Centre, Align::Centre));
        });
    }

    fn spatial_nav(&self, _: bool, _: Option<usize>) -> Option<usize> {
        // We have no child within our rect
        None
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let mut state = self.input_state(mgr, disabled);
        if self.popup_id.is_some() {
            state.depress = true;
        }
        draw_handle.button(self.core.rect, state);
        draw_handle.text(self.core.rect.pos, self.label.as_ref(), TextClass::Button);
    }
}

impl ComboBox<VoidMsg> {
    /// Construct a combobox
    ///
    /// Constructs a combobox with labels derived from an iterator over string
    /// types, and the chosen `active` entry. For example:
    /// ```
    /// # use kas::widget::ComboBox;
    /// let combobox = ComboBox::new(&["zero", "one", "two"], 0);
    /// ```
    #[inline]
    pub fn new<T: Into<AccelString>, I: IntoIterator<Item = T>>(iter: I, active: usize) -> Self {
        let entries = iter
            .into_iter()
            .map(|label| MenuEntry::new(label, ()))
            .collect();
        Self::new_entries(entries, active)
    }

    /// Construct a combobox with the given menu entries
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked,
    /// with the `active` choice selected (0-based index).
    #[inline]
    pub fn new_entries(entries: Vec<MenuEntry<()>>, active: usize) -> Self {
        let label = entries.get(active).map(|entry| entry.get_string());
        let label = Text::new_single(label.unwrap_or("".to_string()));
        ComboBox {
            core: Default::default(),
            label,
            frame_size: Default::default(),
            popup: ComboPopup {
                core: Default::default(),
                inner: Column::new(entries),
            },
            active,
            opening: false,
            popup_id: None,
            on_select: None,
        }
    }

    /// Set the selection handler `f`
    ///
    /// On selection of a new choice the closure `f` is called with the choice's
    /// index. The result of `f` is converted to [`Response::Msg`] or
    /// [`Response::Update`] and returned to the parent.
    #[inline]
    pub fn on_select<M, F>(self, f: F) -> ComboBox<M>
    where
        F: Fn(&mut Manager, usize) -> Option<M> + 'static,
    {
        ComboBox {
            core: self.core,
            label: self.label,
            frame_size: self.frame_size,
            popup: self.popup,
            active: self.active,
            opening: self.opening,
            popup_id: self.popup_id,
            on_select: Some(Rc::new(f)),
        }
    }
}

impl<M: 'static> ComboBox<M> {
    /// Get the index of the active choice
    ///
    /// This index is normally less than the number of choices (`self.len()`),
    /// but may not be if set programmatically or there are no choices.
    #[inline]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active choice
    #[inline]
    pub fn set_active(&mut self, index: usize) -> TkAction {
        if self.active != index {
            self.active = index;
            let string = if index < self.len() {
                self.popup.inner[index].get_string()
            } else {
                "".to_string()
            };
            let avail = self.core.rect.size.clamped_sub(self.frame_size);
            kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
        } else {
            TkAction::empty()
        }
    }

    /// Get the number of entries
    #[inline]
    pub fn len(&self) -> usize {
        self.popup.inner.len()
    }

    /// Remove all choices
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn clear(&mut self) -> TkAction {
        self.popup.inner.clear()
    }

    /// Add a choice to the combobox, in last position
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push<T: Into<AccelString>>(&mut self, label: T) -> TkAction {
        let column = &mut self.popup.inner;
        column.push(MenuEntry::new(label, ()))
        // TODO: localised reconfigure
    }

    /// Pops the last choice from the combobox
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn pop(&mut self) -> (Option<()>, TkAction) {
        let r = self.popup.inner.pop();
        (r.0.map(|_| ()), r.1)
    }

    /// Add a choice at position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn insert<T: Into<AccelString>>(&mut self, index: usize, label: T) -> TkAction {
        let column = &mut self.popup.inner;
        column.insert(index, MenuEntry::new(label, ()))
        // TODO: localised reconfigure
    }

    /// Removes the choice at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, index: usize) -> TkAction {
        self.popup.inner.remove(index).1
    }

    /// Replace the choice at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace<T: Into<AccelString>>(&mut self, index: usize, label: T) -> TkAction {
        self.popup.inner[index].set_accel(label)
    }
}

impl<M: 'static> ComboBox<M> {
    fn map_response(
        &mut self,
        mgr: &mut Manager,
        id: WidgetId,
        event: Event,
        r: Response<(usize, ())>,
    ) -> Response<M> {
        match r {
            Response::None => Response::None,
            Response::Unhandled => match event {
                Event::Command(cmd, _) => {
                    let next = |mgr: &mut Manager, s, clr, rev| {
                        if clr {
                            mgr.clear_nav_focus();
                        }
                        mgr.next_nav_focus(s, rev);
                        Response::None
                    };
                    match cmd {
                        Command::Up => next(mgr, self, false, true),
                        Command::Down => next(mgr, self, false, false),
                        Command::Home => next(mgr, self, true, false),
                        Command::End => next(mgr, self, true, true),
                        _ => Response::Unhandled,
                    }
                }
                _ => Response::Unhandled,
            },
            Response::Pan(delta) => Response::Pan(delta),
            Response::Focus(x) => Response::Focus(x),
            Response::Update | Response::Select => {
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                }
                if let Some(index) = self.popup.inner.find_child_index(id) {
                    if index != self.active {
                        *mgr |= self.set_active(index);
                        return if let Some(ref f) = self.on_select {
                            Response::update_or_msg((f)(mgr, index))
                        } else {
                            Response::Update
                        };
                    }
                }
                Response::None
            }
            Response::Msg((index, ())) => {
                *mgr |= self.set_active(index);
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                }
                if let Some(ref f) = self.on_select {
                    Response::update_or_msg((f)(mgr, index))
                } else {
                    Response::Update
                }
            }
        }
        // NOTE: as part of the Popup API we are expected to trap
        // TkAction::CLOSE here, but we know our widget doesn't generate
        // this action.
    }
}

impl<M: 'static> event::Handler for ComboBox<M> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        let open_popup = |s: &mut Self, mgr: &mut Manager| {
            s.popup_id = mgr.add_popup(kas::Popup {
                id: s.popup.id(),
                parent: s.id(),
                direction: Direction::Down,
            });
            if let Some(id) = s.popup.inner.get_child(s.active).map(|w| w.id()) {
                mgr.set_nav_focus(id);
            }
        };
        match event {
            Event::Activate => {
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                } else {
                    open_popup(self, mgr);
                }
            }
            Event::PressStart {
                source,
                start_id,
                coord,
            } => {
                if self.is_ancestor_of(start_id) {
                    if source.is_primary() {
                        mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                        mgr.set_grab_depress(source, Some(start_id));
                        self.opening = self.popup_id.is_none();
                    }
                } else {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id);
                    }
                    return Response::Unhandled;
                }
            }
            Event::PressMove {
                source,
                cur_id,
                coord,
                ..
            } => {
                if self.popup_id.is_none() {
                    open_popup(self, mgr);
                }
                let cond = self.popup.inner.rect().contains(coord);
                let target = if cond { cur_id } else { None };
                mgr.set_grab_depress(source, target);
                if let Some(id) = target {
                    mgr.set_nav_focus(id);
                }
            }
            Event::PressEnd { end_id, .. } => {
                if let Some(id) = end_id {
                    if id == self.id() {
                        if self.opening {
                            if self.popup_id.is_none() {
                                open_popup(self, mgr);
                            }
                            return Response::None;
                        }
                    } else if self.popup_id.is_some() && self.popup.is_ancestor_of(id) {
                        let r = self.popup.send(mgr, id, Event::Activate);
                        return self.map_response(mgr, id, event, r);
                    }
                }
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                }
            }
            Event::NewPopup(id) => {
                // For a ComboBox, for any new Popup we should close self
                if id != self.popup.id() {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id);
                    }
                }
            }
            Event::PopupRemoved(id) => {
                debug_assert_eq!(Some(id), self.popup_id);
                self.popup_id = None;
            }
            _ => return Response::Unhandled,
        }
        Response::None
    }
}

impl<M: 'static> event::SendEvent for ComboBox<M> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled;
        }

        if id <= self.popup.id() {
            let r = self.popup.send(mgr, id, event.clone());
            self.map_response(mgr, id, event, r)
        } else {
            Manager::handle_generic(self, mgr, event)
        }
    }
}

#[derive(Clone, Debug, Widget)]
#[layout(single)]
#[handler(msg=(usize, ()))]
struct ComboPopup {
    #[widget_core]
    core: CoreData,
    #[widget]
    inner: Column<MenuEntry<()>>,
}
