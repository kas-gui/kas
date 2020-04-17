// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![feature(proc_macro_hygiene)]

use kas::class::HasText;
use kas::event::{Manager, Response, UpdateHandle, VoidMsg, VoidResponse};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::*;
use kas::{Right, Widget, WidgetId};

#[derive(Clone, Debug, VoidMsg)]
enum Item {
    Button,
    Check(bool),
    Combo(i32),
    Radio(WidgetId),
    Edit(String),
    Slider(i32),
    Scroll(u32),
    Popup,
}

struct Guard;
impl EditGuard for Guard {
    type Msg = Item;

    fn activate(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        Some(Item::Edit(edit.get_text().to_string()))
    }

    fn edit(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        // 7a is the colour of *magic*!
        edit.set_error_state(edit.get_text().len() % (7 + 1) == 0);
        None
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    #[derive(Clone, Debug, VoidMsg)]
    enum Menu {
        Theme(&'static str),
        Colour(&'static str),
        Disabled(bool),
    }

    let menubar = MenuBar::<Right, _>::new(vec![
        SubMenu::new(
            "Theme",
            vec![
                Box::new(TextButton::new("Shaded", Menu::Theme("shaded")))
                    as Box<dyn Widget<Msg = Menu>>,
                Box::new(TextButton::new("Flat", Menu::Theme("flat"))),
            ],
        ),
        SubMenu::new(
            "Colours",
            vec![
                Box::new(TextButton::new("Default", Menu::Colour("default"))),
                Box::new(TextButton::new("Light", Menu::Colour("light"))),
                Box::new(TextButton::new("Dark", Menu::Colour("dark"))),
            ],
        ),
        SubMenu::new(
            "Style",
            vec![Box::new(
                CheckBox::new("Disabled").on_toggle(|state| Menu::Disabled(state)),
            )],
        ),
    ]);

    let radio = UpdateHandle::new();
    let widgets = make_widget! {
        #[layout(grid)]
        #[handler(msg = Item)]
        struct {
            #[widget(row=0, col=0)] _ = Label::new("Label"),
            #[widget(row=0, col=1)] _ = Label::new("Hello world"),
            #[widget(row=1, col=0)] _ = Label::new("EditBox"),
            #[widget(row=1, col=1)] _ = EditBox::new("edit me").with_guard(Guard),
            #[widget(row=2, col=0)] _ = Label::new("TextButton"),
            #[widget(row=2, col=1)] _ = TextButton::new("Press me", Item::Button),
            #[widget(row=3, col=0)] _ = Label::new("CheckBox"),
            #[widget(row=3, col=1)] _ = CheckBox::new("Check me").state(true)
                .on_toggle(|check| Item::Check(check)),
            #[widget(row=4, col=0)] _ = Label::new("RadioBox"),
            #[widget(row=4, col=1)] _ = RadioBox::new(radio, "radio box 1").state(false)
                .on_activate(|id| Item::Radio(id)),
            #[widget(row=5, col=0)] _ = Label::new("RadioBox"),
            #[widget(row=5, col=1)] _ = RadioBox::new(radio, "radio box 2").state(true)
                .on_activate(|id| Item::Radio(id)),
            #[widget(row=6, col=0)] _ = Label::new("ComboBox"),
            #[widget(row=6, col=1, handler = handle_combo)] cb: ComboBox<i32> =
                [("One", 1), ("Two", 2), ("Three", 3)].iter().cloned().collect(),
            #[widget(row=7, col=0)] _ = Label::new("Slider"),
            #[widget(row=7, col=1, handler = handle_slider)] s =
                Slider::<i32, Right>::new(-2, 2, 1).with_value(0),
            #[widget(row=8, col=0)] _ = Label::new("ScrollBar"),
            #[widget(row=8, col=1, handler = handle_scroll)] sc =
                ScrollBar::<Right>::new().with_limits(5, 2),
            #[widget(row=9)] _ = Label::new("Child window"),
            #[widget(row=9, col = 1)] _ = TextButton::new("Open", Item::Popup),
        }
        impl {
            fn handle_combo(&mut self, _: &mut Manager, msg: i32) -> Response<Item> {
                Response::Msg(Item::Combo(msg))
            }
            fn handle_slider(&mut self, _: &mut Manager, msg: i32) -> Response<Item> {
                Response::Msg(Item::Slider(msg))
            }
            fn handle_scroll(&mut self, _: &mut Manager, msg: u32) -> Response<Item> {
                Response::Msg(Item::Scroll(msg))
            }
        }
    };

    let window = Window::new(
        "Widget Gallery",
        make_widget! {
            #[layout(column)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(handler = menu)] _ = menubar,
                #[widget(halign = centre)] _ = Frame::new(Label::new("Widget Gallery")),
                #[widget(handler = activations)] gallery:
                    for<W: Widget<Msg = Item>> ScrollRegion<W> =
                    ScrollRegion::new(widgets).with_auto_bars(true),
            }
            impl {
                fn menu(&mut self, mgr: &mut Manager, msg: Menu) -> VoidResponse {
                    match msg {
                        Menu::Theme(name) => {
                            println!("Theme: {:?}", name);
                            #[cfg(not(feature = "stack_dst"))]
                            println!("Warning: switching themes requires feature 'stack_dst'");

                            mgr.adjust_theme(|theme| theme.set_theme(name));
                        }
                        Menu::Colour(name) => {
                            println!("Colour scheme: {:?}", name);
                            mgr.adjust_theme(|theme| theme.set_colours(name));
                        }
                        Menu::Disabled(state) => {
                            *mgr += self.gallery.inner_mut().set_disabled(state);
                        }
                    }
                    Response::None
                }
                fn activations(&mut self, mgr: &mut Manager, item: Item)
                    -> VoidResponse
                {
                    match item {
                        Item::Button => println!("Clicked!"),
                        Item::Check(b) => println!("CheckBox: {}", b),
                        Item::Combo(c) => println!("ComboBox: {}", c),
                        Item::Radio(id) => println!("RadioBox: {}", id),
                        Item::Edit(s) => println!("Edited: {}", s),
                        Item::Slider(p) => println!("Slider: {}", p),
                        Item::Scroll(p) => println!("ScrollBar: {}", p),
                        Item::Popup => {
                            let window = MessageBox::new("Popup", "Hello!");
                            mgr.add_window(Box::new(window));
                        }
                    };
                    Response::None
                }
            }
        },
    );

    #[cfg(feature = "stack_dst")]
    let theme = kas_theme::MultiTheme::builder()
        .add("shaded", kas_theme::ShadedTheme::new())
        .add("flat", kas_theme::FlatTheme::new())
        .build();
    #[cfg(not(feature = "stack_dst"))]
    let theme = kas_theme::ShadedTheme::new();

    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
