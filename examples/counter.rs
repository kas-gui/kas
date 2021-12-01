// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::layout;
use kas::macros::make_widget;
use kas::prelude::*;
use kas::widgets::{Label, TextButton, Window};

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let counter = make_widget! {
        #[handler(msg = VoidMsg)]
        struct {
            #[widget]
            display: Label<String> = Label::from("0"),
            #[widget(use_msg = update)]
            b_decr = TextButton::new_msg("−", -1),
            #[widget(use_msg = update)]
            b_incr = TextButton::new_msg("+", 1),
            count: i32 = 0,
        }
        impl Self {
            fn update(&mut self, mgr: &mut Manager, incr: i32) {
                self.count += incr;
                *mgr |= self.display.set_string(self.count.to_string());
            }
            fn layout<'a>(&'a mut self) -> layout::Layout<'a> {
                let (data, next) = self.core.layout.storage::<layout::FixedRowStorage<2>>();
                let (data2, _) = next.storage::<layout::FixedRowStorage<2>>();

                layout::Layout::list(
                    {
                        let arr = [
                            layout::Layout::single(self.display.as_widget_mut(), AlignHints::CENTER),
                            layout::Layout::list(
                                {
                                    let arr = [
                                        layout::Layout::single(self.b_decr.as_widget_mut(), AlignHints::NONE),
                                        layout::Layout::single(self.b_incr.as_widget_mut(), AlignHints::NONE),
                                    ]; arr.into_iter()
                                },
                                kas::dir::Right,
                                data2,
                                AlignHints::NONE,
                            ),
                        ]; arr.into_iter()
                    },
                    kas::dir::Down,
                    data,
                    AlignHints::NONE,
                )
            }
        }
        impl Layout for Self {
            fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
                self.layout().size_rules(size_handle, axis)
            }

            fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
                self.core.rect = rect;
                self.layout().set_rect(mgr, rect, align);
            }

            fn find_id(&self, coord: Coord) -> Option<WidgetId> {
                if !self.rect().contains(coord) {
                    None
                } else if let Some(id) = self.display.find_id(coord) {
                    Some(id)
                } else if let Some(id) = self.b_decr.find_id(coord) {
                    Some(id)
                } else if let Some(id) = self.b_incr.find_id(coord) {
                    Some(id)
                } else  {
                    Some(self.id())
                }
            }

            fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
                self.display.draw(draw_handle, mgr, disabled);
                self.b_decr.draw(draw_handle, mgr, disabled);
                self.b_incr.draw(draw_handle, mgr, disabled);
            }
        }
    };

    let window = Window::new("Counter", counter);

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
