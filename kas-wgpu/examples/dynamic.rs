// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dynamic widget example
#![feature(proc_macro_hygiene)]

use kas::class::HasText;
use kas::event::{Callback, Manager, Response, VoidMsg};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{Column, EditBox, EditBoxVoid, Filler, Label, ScrollRegion, TextButton, Window};

#[derive(Clone, Debug, VoidMsg)]
enum Control {
    Decr,
    Incr,
    Set,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let controls = make_widget! {
        #[widget_config]
        #[layout(horizontal)]
        #[handler(msg = usize)]
        struct {
            #[widget] _ = Label::new("Number of rows:"),
            #[widget(handler = activate)] edit: impl HasText = EditBox::new("3")
                .on_afl(|text| text.parse::<usize>().ok()),
            #[widget(handler = button)] _ = TextButton::new("Set", Control::Set),
            #[widget(handler = button)] _ = TextButton::new("−", Control::Decr),
            #[widget(handler = button)] _ = TextButton::new("+", Control::Incr),
            n: usize = 3,
        }
        impl {
            fn activate(&mut self, _: &mut Manager, n: usize) -> Response<usize> {
                self.n = n;
                n.into()
            }
            fn button(&mut self, mgr: &mut Manager, msg: Control) -> Response<usize> {
                let n = match msg {
                    Control::Decr => self.n.saturating_sub(1),
                    Control::Incr => self.n.saturating_add(1),
                    Control::Set => self.n,
                };
                self.edit.set_text(mgr, n.to_string());
                self.n = n;
                n.into()
            }
        }
    };
    let mut window = Window::new(
        "Dynamic widget demo",
        make_widget! {
            #[widget_config]
            #[layout(vertical)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] _ = Label::new("Demonstration of dynamic widget creation / deletion"),
                #[widget(handler = handler)] controls -> usize = controls,
                #[widget] list: ScrollRegion<Column<EditBoxVoid>> =
                    ScrollRegion::new(Column::new(vec![])).with_bars(false, true),
                #[widget] _ = Filler::maximise(),
            }
            impl {
                fn handler(&mut self, mgr: &mut Manager, n: usize) -> Response<VoidMsg> {
                    self.list.inner_mut().resize_with(mgr, n, |i| EditBox::new(i.to_string()));
                    Response::None
                }
            }
        },
    );

    window.add_callback(Callback::Start, &|w, mgr| {
        let _ = w.handler(mgr, 3);
    });

    let theme = kas_theme::ShadedTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
