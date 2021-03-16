// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
//!
//! This is a test-bed to demonstrate most toolkit functionality
//! (excepting custom graphics).

use kas::event::{Command, VoidResponse};
use kas::prelude::*;
use kas::widget::*;
use kas::{dir::Right, Future};

#[derive(Clone, Debug, VoidMsg)]
enum Item {
    Button,
    Check(bool),
    Combo(i32),
    Radio(u32),
    Edit(String),
    Slider(i32),
    Scroll(i32),
}

#[derive(Debug)]
struct Guard;
impl EditGuard for Guard {
    type Msg = Item;

    fn activate(edit: &mut EditField<Self>, _: &mut Manager) -> Option<Self::Msg> {
        Some(Item::Edit(edit.get_string()))
    }

    fn edit(edit: &mut EditField<Self>, _: &mut Manager) -> Option<Self::Msg> {
        // 7a is the colour of *magic*!
        edit.set_error_state(edit.get_str().len() % (7 + 1) == 0);
        None
    }
}

#[derive(Debug, Widget)]
#[widget(config=noauto)]
#[layout(grid)]
#[handler(handle=noauto)]
struct TextEditPopup {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget(cspan = 3)]
    edit: EditBox,
    #[widget(row = 1, col = 0)]
    fill: Filler,
    #[widget(row=1, col=1, handler = close)]
    cancel: TextButton<bool>,
    #[widget(row=1, col=2, handler = close)]
    save: TextButton<bool>,
    commit: bool,
}
impl TextEditPopup {
    fn new<S: ToString>(text: S) -> Self {
        TextEditPopup {
            core: Default::default(),
            layout_data: Default::default(),
            edit: EditBox::new(text).multi_line(true),
            fill: Filler::maximize(),
            cancel: TextButton::new_msg("&Cancel", false),
            save: TextButton::new_msg("&Save", true),
            commit: false,
        }
    }

    fn close(&mut self, mgr: &mut Manager, commit: bool) -> VoidResponse {
        self.commit = commit;
        mgr.send_action(TkAction::CLOSE);
        Response::None
    }
}
impl WidgetConfig for TextEditPopup {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.register_nav_fallback(self.id());
    }
}
impl Handler for TextEditPopup {
    type Msg = VoidMsg;
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::Command(Command::Escape, _) => self.close(mgr, false),
            Event::Command(Command::Return, _) => self.close(mgr, true),
            _ => Response::Unhandled,
        }
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    #[derive(Clone, Debug, VoidMsg)]
    enum Menu {
        Theme(&'static str),
        Colour(&'static str),
        Disabled(bool),
        Quit,
    }

    let themes = vec![
        MenuEntry::new("&Shaded", Menu::Theme("shaded")).boxed(),
        MenuEntry::new("&Flat", Menu::Theme("flat")).boxed(),
    ];
    let colours = vec![
        MenuEntry::new("&White", Menu::Colour("white")),
        MenuEntry::new("&Grey", Menu::Colour("grey")),
        MenuEntry::new("&Light", Menu::Colour("light")),
        MenuEntry::new("Dar&k", Menu::Colour("dark")),
    ];
    let styles = vec![
        SubMenu::right("&Colours", colours).boxed(),
        Separator::infer().boxed(),
        MenuToggle::new("&Disabled")
            .on_toggle(|_, state| Some(Menu::Disabled(state)))
            .boxed(),
    ];
    let menubar = MenuBar::<_>::new(vec![
        SubMenu::new("&App", vec![MenuEntry::new("&Quit", Menu::Quit).boxed()]),
        SubMenu::new("&Theme", themes),
        SubMenu::new("&Style", styles),
    ]);

    let popup_edit_box = make_widget! {
        #[layout(row)]
        #[handler(handle = noauto)]
        struct {
            #[widget] label: StringLabel = Label::from("Use button to edit →"),
            #[widget(handler = edit)] edit = TextButton::new_msg("&Edit", ()),
            future: Option<Future<Option<String>>> = None,
        }
        impl {
            fn edit(&mut self, mgr: &mut Manager, _: ()) -> VoidResponse {
                if self.future.is_none() {
                    let text = self.label.get_string();
                    let mut window = Window::new("Edit text", TextEditPopup::new(text));
                    let (future, update) = window.on_drop(|w: &mut TextEditPopup| if w.commit {
                        Some(w.edit.get_string())
                    } else {
                        None
                    });
                    self.future = Some(future);
                    mgr.update_on_handle(update, self.id());
                    mgr.add_window(Box::new(window));
                }
                Response::None
            }
        }
        impl Handler {
            type Msg = VoidMsg;
            fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
                match event {
                    Event::HandleUpdate { .. } => {
                        // There should be no other source of this event,
                        // so we can assume our future is finished
                        if let Some(future) = self.future.take() {
                            let result = future.try_finish().unwrap();
                            if let Some(text) = result {
                                *mgr |= self.label.set_string(text);
                            }
                        }
                        Response::None
                    }
                    _ => Response::Unhandled,
                }
            }
        }
    };

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
            #[widget(row=2, col=1)] _ = TextButton::new_msg("&Press me", Item::Button),
            #[widget(row=3, col=0)] _ = Label::new("CheckBox"),
            #[widget(row=3, col=1)] _ = CheckBox::new("&Check me")
                .with_state(true)
                .on_toggle(|_, check| Some(Item::Check(check))),
            #[widget(row=4, col=0)] _ = Label::new("RadioBox"),
            #[widget(row=4, col=1)] _ = RadioBox::new("radio box &1", radio)
                .on_select(|_| Some(Item::Radio(1))),
            #[widget(row=5, col=0)] _ = Label::new("RadioBox"),
            #[widget(row=5, col=1)] _ = RadioBox::new("radio box &2", radio)
                .with_state(true)
                .on_select(|_| Some(Item::Radio(2))),
            #[widget(row=6, col=0)] _ = Label::new("ComboBox"),
            #[widget(row=6, col=1)] _ =
                ComboBox::new(&["&One", "T&wo", "Th&ree"], 0)
                .on_select(|_, index| Some(Item::Combo((index + 1).cast()))),
            #[widget(row=7, col=0)] _ = Label::new("Slider"),
            #[widget(row=7, col=1, handler = handle_slider)] s =
                Slider::<i32, Right>::new(-2, 2, 1).with_value(0),
            #[widget(row=8, col=0)] _ = Label::new("ScrollBar"),
            #[widget(row=8, col=1, handler = handle_scroll)] sc: ScrollBar<Right> =
                ScrollBar::new().with_limits(100, 20),
            #[widget(row=9, col=1)] pg: ProgressBar<Right> = ProgressBar::new(),
            #[widget(row=9, col=0)] _ = Label::new("ProgressBar"),
            #[widget(row=10, col=0)] _ = Label::new("Child window"),
            #[widget(row=10, col = 1)] _ = popup_edit_box,
        }
        impl {
            fn handle_slider(&mut self, _: &mut Manager, msg: i32) -> Response<Item> {
                Response::Msg(Item::Slider(msg))
            }
            fn handle_scroll(&mut self, mgr: &mut Manager, msg: i32) -> Response<Item> {
                let ratio = msg as f32 / self.sc.max_value() as f32;
                *mgr |= self.pg.set_value(ratio);
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
                    for<W: Widget<Msg = Item>> ScrollBarRegion<W> =
                        ScrollBarRegion::new2(widgets),
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
                            *mgr |= self.gallery.set_disabled(state);
                        }
                        Menu::Quit => {
                            *mgr |= TkAction::EXIT;
                        }
                    }
                    Response::None
                }
                fn activations(&mut self, _: &mut Manager, item: Item) -> VoidResponse {
                    match item {
                        Item::Button => println!("Clicked!"),
                        Item::Check(b) => println!("CheckBox: {}", b),
                        Item::Combo(c) => println!("ComboBox: {}", c),
                        Item::Radio(id) => println!("RadioBox: {}", id),
                        Item::Edit(s) => println!("Edited: {}", s),
                        Item::Slider(p) => println!("Slider: {}", p),
                        Item::Scroll(p) => println!("ScrollBar: {}", p),
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
