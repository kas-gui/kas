// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::Menu;
use crate::Column;
use kas::event::{self, Command};
use kas::prelude::*;
use kas::theme::TextClass;
use kas::{layout, WindowId};

widget! {
    /// A sub-menu
    #[derive(Clone, Debug)]
    pub struct SubMenu<D: Directional, W: Menu> {
        #[widget_core]
        core: CoreData,
        direction: D,
        pub(crate) key_nav: bool,
        label: Text<AccelString>,
        label_store: layout::TextStorage,
        frame_store: layout::FrameStorage,
        #[widget]
        pub list: Column<W>,
        popup_id: Option<WindowId>,
        closing_menu: bool,
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
                label_store: Default::default(),
                frame_store: Default::default(),
                list: Column::new(list),
                popup_id: None,
                closing_menu: false,
            }
        }

        fn open_menu(&mut self, mgr: &mut EventMgr, set_focus: bool) {
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
        fn close_menu(&mut self, mgr: &mut EventMgr, restore_focus: bool) {
            if let Some(id) = self.popup_id {
                mgr.close_window(id, restore_focus);
                self.closing_menu = true;
            }
        }

        fn handle_dir_key(&mut self, mgr: &mut EventMgr, cmd: Command) -> Response<W::Msg> {
            if self.menu_is_open() {
                if let Some(dir) = cmd.as_direction() {
                    if dir.is_vertical() == self.list.direction().is_vertical() {
                        let rev = dir.is_reversed() ^ self.list.direction().is_reversed();
                        mgr.next_nav_focus(self, rev, true);
                        Response::Used
                    } else if dir == self.direction.as_direction().reversed() {
                        self.close_menu(mgr, true);
                        Response::Used
                    } else {
                        Response::Unused
                    }
                } else if matches!(cmd, Command::Home | Command::End) {
                    mgr.clear_nav_focus();
                    let rev = cmd == Command::End;
                    mgr.next_nav_focus(self, rev, true);
                    Response::Used
                } else {
                    Response::Unused
                }
            } else if Some(self.direction.as_direction()) == cmd.as_direction() {
                self.open_menu(mgr, true);
                Response::Used
            } else {
                Response::Unused
            }
        }
    }

    impl WidgetConfig for Self {
        fn pre_configure(&mut self, mgr: &mut SetRectMgr, id: WidgetId) {
            self.core_data_mut().id = id;
            mgr.add_accel_keys(self.id_ref(), self.label.text().keys());
            mgr.new_accel_layer(self.id(), true);
        }

        fn key_nav(&self) -> bool {
            self.key_nav
        }
    }

    impl kas::Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let label = layout::Layout::text(&mut self.label_store, &mut self.label, TextClass::MenuLabel);
            layout::Layout::frame(&mut self.frame_store, label)
        }

        fn spatial_nav(&mut self, _: &mut SetRectMgr, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.with_core(self.core_data());
            if self.popup_id.is_some() {
                if !self.closing_menu {
                    draw.state.insert(InputState::DEPRESS);
                }
            }
            draw.menu_entry(self.core.rect);
            draw.text_accel(
                self.label_store.pos,
                &self.label,
                draw.ev_state().show_accel_labels(),
                TextClass::MenuLabel,
            );
        }
    }

    impl<D: Directional, M: 'static, W: Menu<Msg = M>> event::Handler for SubMenu<D, W> {
        type Msg = M;

        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response<M> {
            match event {
                Event::Activate => {
                    if self.popup_id.is_none() {
                        self.open_menu(mgr, true);
                    }
                    Response::Used
                }
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                    self.closing_menu = false;
                    Response::Used
                }
                Event::Command(cmd, _) => self.handle_dir_key(mgr, cmd),
                _ => Response::Unused,
            }
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut EventMgr, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.is_disabled() {
                return Response::Unused;
            }

            if self.eq_id(&id) {
                EventMgr::handle_generic(self, mgr, event)
            } else {
                let r = self.list.send(mgr, id.clone(), event.clone());

                match r {
                    Response::Unused => match event {
                        Event::Command(cmd, _) if self.popup_id.is_some() => {
                            self.handle_dir_key(mgr, cmd)
                        }
                        _ => Response::Unused,
                    },
                    Response::Select => {
                        self.set_menu_path(mgr, Some(&id), true);
                        Response::Used
                    }
                    r @ (Response::Update | Response::Msg(_)) => {
                        self.close_menu(mgr, true);
                        r
                    }
                    r => r,
                }
            }
        }
    }

    impl Menu for Self {
        fn menu_is_open(&self) -> bool {
            self.popup_id.is_some()
        }

        fn set_menu_path(&mut self, mgr: &mut EventMgr, target: Option<&WidgetId>, set_focus: bool) {
            match target {
                Some(id) if self.is_ancestor_of(id) => {
                    if self.popup_id.is_none() {
                        self.open_menu(mgr, set_focus);
                    }
                    if !self.eq_id(id) {
                        for i in 0..self.list.len() {
                            self.list[i].set_menu_path(mgr, target, set_focus);
                        }
                    }
                }
                _ if self.popup_id.is_some() => {
                    self.close_menu(mgr, set_focus);
                }
                _ => (),
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
            let avail = self.core.rect.size.clamped_sub(self.frame_store.size);
            action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
        }
    }
}
