// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::Menu;
use crate::Column;
use kas::draw::TextClass;
use kas::event::{self, Command, ConfigureManager};
use kas::prelude::*;
use kas::WindowId;

widget! {
    /// A sub-menu
    #[derive(Clone, Debug)]
    #[widget(config=noauto)]
    #[handler(noauto)]
    pub struct SubMenu<D: Directional, W: Menu> {
        #[widget_core]
        core: CoreData,
        direction: D,
        pub(crate) key_nav: bool,
        label: Text<AccelString>,
        label_off: Offset,
        frame_size: Size,
        #[widget]
        pub list: Column<W>,
        popup_id: Option<WindowId>,
    }

    impl Self where D: Default {
        /// Construct a sub-menu
        #[inline]
        pub fn new<S: Into<AccelString>>(label: S, list: Vec<W>) -> Self {
            SubMenu::new_with_direction(Default::default(), label, list)
        }
    }

    impl<W: Menu> SubMenu<kas::dir::Right, W> {
        /// Construct a sub-menu, opening to the right
        // NOTE: this is used since we can't infer direction of a boxed SubMenu.
        // Consider only accepting an enum of special menu widgets?
        // Then we can pass type information.
        #[inline]
        pub fn right<S: Into<AccelString>>(label: S, list: Vec<W>) -> Self {
            SubMenu::new(label, list)
        }
    }

    impl<W: Menu> SubMenu<kas::dir::Down, W> {
        /// Construct a sub-menu, opening downwards
        #[inline]
        pub fn down<S: Into<AccelString>>(label: S, list: Vec<W>) -> Self {
            SubMenu::new(label, list)
        }
    }

    impl Self {
        /// Construct a sub-menu
        #[inline]
        pub fn new_with_direction<S: Into<AccelString>>(direction: D, label: S, list: Vec<W>) -> Self {
            SubMenu {
                core: Default::default(),
                direction,
                key_nav: true,
                label: Text::new_single(label.into()),
                label_off: Offset::ZERO,
                frame_size: Size::ZERO,
                list: Column::new(list),
                popup_id: None,
            }
        }

        fn open_menu(&mut self, mgr: &mut Manager, set_focus: bool) {
            if self.popup_id.is_none() {
                self.popup_id = mgr.add_popup(kas::Popup {
                    id: self.list.id(),
                    parent: self.id(),
                    direction: self.direction.as_direction(),
                });
                if set_focus {
                    mgr.next_nav_focus(self, false, true);
                }
            }
        }
        fn close_menu(&mut self, mgr: &mut Manager, restore_focus: bool) {
            if let Some(id) = self.popup_id {
                mgr.close_window(id, restore_focus);
            }
        }

        fn handle_dir_key(&mut self, mgr: &mut Manager, cmd: Command) -> Response<W::Msg> {
            if self.menu_is_open() {
                if let Some(dir) = cmd.as_direction() {
                    if dir.is_vertical() == self.list.direction().is_vertical() {
                        let rev = dir.is_reversed() ^ self.list.direction().is_reversed();
                        mgr.next_nav_focus(self, rev, true);
                        Response::None
                    } else if dir == self.direction.as_direction().reversed() {
                        self.close_menu(mgr, true);
                        Response::None
                    } else {
                        Response::Unhandled
                    }
                } else if matches!(cmd, Command::Home | Command::End) {
                    mgr.clear_nav_focus();
                    let rev = cmd == Command::End;
                    mgr.next_nav_focus(self, rev, true);
                    Response::None
                } else {
                    Response::Unhandled
                }
            } else if Some(self.direction.as_direction()) == cmd.as_direction() {
                self.open_menu(mgr, true);
                Response::None
            } else {
                Response::Unhandled
            }
        }
    }

    impl WidgetConfig for Self {
        fn configure_recurse(&mut self, mut cmgr: ConfigureManager) {
            cmgr.mgr().push_accel_layer(true);
            self.list.configure_recurse(cmgr.child());
            self.core_data_mut().id = cmgr.next_id(self.id());
            let mgr = cmgr.mgr();
            mgr.pop_accel_layer(self.id());
            mgr.add_accel_keys(self.id(), self.label.text().keys());
        }

        fn key_nav(&self) -> bool {
            self.key_nav
        }
    }

    impl kas::Layout for Self {
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let frame_rules = size_handle.menu_frame(axis.is_vertical());
            let text_rules = size_handle.text_bound(&mut self.label, TextClass::MenuLabel, axis);
            let (rules, offset, size) = frame_rules.surround_as_margin(text_rules);
            self.label_off.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let size = rect.size - self.frame_size;
            self.label.update_env(|env| {
                env.set_bounds(size.into());
                env.set_align(align.unwrap_or(Align::Default, Align::Centre));
            });
        }

        fn spatial_nav(&mut self, _: &mut Manager, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
            let mut state = self.input_state(mgr, disabled);
            if self.popup_id.is_some() {
                state.insert(InputState::DEPRESS);
            }
            draw_handle.menu_entry(self.core.rect, state);
            let pos = self.core.rect.pos + self.label_off;
            draw_handle.text_accel(
                pos,
                &self.label,
                mgr.show_accel_labels(),
                TextClass::MenuLabel,
                state,
            );
        }
    }

    impl<D: Directional, M: 'static, W: Menu<Msg = M>> event::Handler for SubMenu<D, W> {
        type Msg = M;

        fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
            match event {
                Event::Activate => {
                    if self.popup_id.is_none() {
                        self.open_menu(mgr, true);
                    }
                }
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                }
                Event::Command(cmd, _) => return self.handle_dir_key(mgr, cmd),
                _ => return Response::Unhandled,
            }
            Response::None
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.is_disabled() {
                return Response::Unhandled;
            }

            if id <= self.list.id() {
                let r = self.list.send(mgr, id, event.clone());

                match r {
                    Response::None => Response::None,
                    Response::Pan(delta) => Response::Pan(delta),
                    Response::Focus(rect) => Response::Focus(rect),
                    Response::Unhandled => match event {
                        Event::Command(cmd, _) if self.popup_id.is_some() => {
                            self.handle_dir_key(mgr, cmd)
                        }
                        _ => Response::Unhandled,
                    },
                    Response::Select => {
                        self.set_menu_path(mgr, Some(id), true);
                        Response::None
                    }
                    r @ (Response::Update | Response::Msg(_)) => {
                        self.close_menu(mgr, true);
                        r
                    }
                }
            } else {
                Manager::handle_generic(self, mgr, event)
            }
        }
    }

    impl Menu for Self {
        fn menu_is_open(&self) -> bool {
            self.popup_id.is_some()
        }

        fn set_menu_path(&mut self, mgr: &mut Manager, target: Option<WidgetId>, set_focus: bool) {
            match target {
                Some(id) if self.is_ancestor_of(id) => {
                    if self.popup_id.is_some() {
                        // We should close other sub-menus before opening
                        let mut child = None;
                        for i in 0..self.list.len() {
                            if self.list[i].is_ancestor_of(id) {
                                child = Some(i);
                            } else {
                                self.list[i].set_menu_path(mgr, None, set_focus);
                            }
                        }
                        if let Some(i) = child {
                            self.list[i].set_menu_path(mgr, target, set_focus);
                        }
                    } else {
                        self.open_menu(mgr, set_focus);
                        if id != self.id() {
                            for i in 0..self.list.len() {
                                self.list[i].set_menu_path(mgr, target, set_focus);
                            }
                        }
                    }
                }
                _ => {
                    if self.popup_id.is_some() {
                        for i in 0..self.list.len() {
                            self.list[i].set_menu_path(mgr, None, set_focus);
                        }
                        self.close_menu(mgr, set_focus);
                    }
                }
            }
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl SetAccel for Self {
        fn set_accel_string(&mut self, string: AccelString) -> TkAction {
            let mut action = TkAction::empty();
            if self.label.text().keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            let avail = self.core.rect.size.clamped_sub(self.frame_size);
            action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
        }
    }
}
