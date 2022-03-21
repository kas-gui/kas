// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
//!
//! This is a test-bed to demonstrate most toolkit functionality
//! (excepting custom graphics).

use kas::draw::color::Rgb;
use kas::event::VirtualKeyCode as VK;
use kas::event::{Command, VoidResponse};
use kas::prelude::*;
use kas::resvg::Svg;
use kas::widgets::*;
use kas::{dir::Right, Future};

#[derive(Clone, Debug, VoidMsg)]
enum Item {
    Button,
    LightTheme,
    DarkTheme,
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

    fn activate(edit: &mut EditField<Self>, _: &mut EventMgr) -> Option<Self::Msg> {
        Some(Item::Edit(edit.get_string()))
    }

    fn edit(edit: &mut EditField<Self>, _: &mut EventMgr) -> Option<Self::Msg> {
        // 7a is the colour of *magic*!
        edit.set_error_state(edit.get_str().len() % (7 + 1) == 0);
        None
    }
}

widget! {
    #[derive(Debug)]
    #[widget{
        layout = grid: {
            0, 0..3: self.edit;
            1, 0: self.fill; 1, 1: self.cancel; 1, 2: self.save;
        };
    }]
    struct TextEditPopup {
        #[widget_core]
        core: CoreData,
        #[widget] edit: EditBox,
        #[widget] fill: Filler,
        #[widget(flatmap_msg = close)] cancel: TextButton<bool>,
        #[widget(flatmap_msg = close)] save: TextButton<bool>,
        commit: bool,
    }
    impl TextEditPopup {
        fn new<S: ToString>(text: S) -> Self {
            TextEditPopup {
                core: Default::default(),
                edit: EditBox::new(text).multi_line(true),
                fill: Filler::maximize(),
                cancel: TextButton::new_msg("&Cancel", false),
                save: TextButton::new_msg("&Save", true),
                commit: false,
            }
        }

        fn close(&mut self, mgr: &mut EventMgr, commit: bool) -> VoidResponse {
            self.commit = commit;
            mgr.send_action(TkAction::CLOSE);
            Response::Used
        }
    }
    impl WidgetConfig for TextEditPopup {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.register_nav_fallback(self.id());
        }
    }
    impl Handler for TextEditPopup {
        type Msg = VoidMsg;
        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response<Self::Msg> {
            match event {
                Event::Command(Command::Escape, _) => self.close(mgr, false),
                Event::Command(Command::Return, _) => self.close(mgr, true),
                _ => Response::Unused,
            }
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    #[cfg(feature = "stack_dst")]
    let theme = kas::theme::MultiTheme::builder()
        .add("flat", kas::theme::FlatTheme::new())
        .add("shaded", kas::theme::ShadedTheme::new())
        .build();
    #[cfg(not(feature = "stack_dst"))]
    let theme = kas::theme::ShadedTheme::new();
    let mut toolkit = kas::shell::Toolkit::new(theme)?;

    #[derive(Clone, Debug, VoidMsg)]
    enum Menu {
        Theme(&'static str),
        Colour(String),
        Disabled(bool),
        Quit,
    }

    let menubar = MenuBar::<Menu>::builder()
        .menu("&App", |menu| {
            menu.entry("&Quit", Menu::Quit);
        })
        .menu("&Theme", |menu| {
            menu.entry("&Flat", Menu::Theme("flat"))
                .entry("&Shaded", Menu::Theme("shaded"));
        })
        .menu("&Style", |menu| {
            menu.submenu("&Colours", |mut menu| {
                // Enumerate colour schemes. Access through the toolkit since
                // this handles config loading.
                for name in toolkit.theme().list_schemes().iter() {
                    let mut title = String::with_capacity(name.len() + 1);
                    match name {
                        &"" => title.push_str("&Default"),
                        &"dark" => title.push_str("Dar&k"),
                        name => {
                            let mut iter = name.char_indices();
                            if let Some((_, c)) = iter.next() {
                                title.push('&');
                                for c in c.to_uppercase() {
                                    title.push(c);
                                }
                                if let Some((i, _)) = iter.next() {
                                    title.push_str(&name[i..]);
                                }
                            }
                        }
                    }
                    menu.push_entry(title, Menu::Colour(name.to_string()));
                }
            })
            .separator()
            .toggle("&Disabled", |_, state| Some(Menu::Disabled(state)));
        })
        .build();

    let popup_edit_box = make_widget! {
        #[widget{
            layout = row: *;
        }]
        struct {
            #[widget] label: StringLabel = Label::from("Use button to edit →"),
            #[widget(use_msg = edit)] edit = TextButton::new_msg("&Edit", ()),
            future: Option<Future<Option<String>>> = None,
        }
        impl Self {
            fn edit(&mut self, mgr: &mut EventMgr, _: ()) {
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
            }
        }
        impl Handler for Self {
            type Msg = VoidMsg;
            fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response<Self::Msg> {
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
                        Response::Used
                    }
                    _ => Response::Unused,
                }
            }
        }
    };

    let text = "Example text in multiple languages.
مثال على نص بلغات متعددة.
Пример текста на нескольких языках.
טקסט לדוגמא במספר שפות.";

    let radio = RadioBoxGroup::default();
    let widgets = make_widget! {
        // TODO: this would be better expressed with a column layout, though we
        // want better alignment controls first (which are also needed for menus).
        #[widget{
            layout = grid: {
                0, 0: self.sll; 0, 1: self.sl;
                1, 0: self.ebl; 1, 1: self.eb;
                2, 0: self.tbl; 2, 1: self.tb;
                3, 0: self.bil; 3, 1: self.bi;
                4, 0: self.cbl; 4, 1: self.cb;
                5, 0: self.rbl; 5, 1: self.rb;
                6, 0: self.rb2l; 6, 1: self.rb2;
                7, 0: self.cbbl; 7, 1: self.cbb;
                8, 0: self.sdl; 8, 1: self.sd;
                9, 0: self.scl; 9, 1: self.sc;
                10, 0: self.pgl; 10, 1: self.pg;
                11, 0: self.svl; 11, 1: align(center): self.sv;
                12, 0: self.pul; 12, 1: self.pu;
            };
        }]
        #[handler(msg = Item)]
        struct {
            #[widget] sll = Label::new("ScrollLabel"),
            #[widget] sl = ScrollLabel::new(text),
            #[widget] ebl = Label::new("EditBox"),
            #[widget] eb = EditBox::new("edit me").with_guard(Guard),
            #[widget] tbl = Label::new("TextButton"),
            #[widget] tb = TextButton::new_msg("&Press me", Item::Button),
            #[widget] bil = Label::new("Button<Image>"),
            #[widget] bi = row![
                Button::new_msg(Image::new("res/sun_32.png"), Item::LightTheme)
                    .with_color(Rgb::rgb(0.3, 0.4, 0.5))
                    .with_keys(&[VK::L]),
                Button::new_msg(Image::new("res/moon_32.png"), Item::DarkTheme)
                    .with_color(Rgb::grey(0.1))
                    .with_keys(&[VK::K]),
            ],
            #[widget] cbl = Label::new("CheckBox"),
            #[widget] cb = CheckBox::new("&Check me")
                .with_state(true)
                .on_toggle(|_, check| Some(Item::Check(check))),
            #[widget] rbl = Label::new("RadioBox"),
            #[widget] rb = RadioBox::new("radio box &1", radio.clone())
                .on_select(|_| Some(Item::Radio(1))),
            #[widget] rb2l = Label::new("RadioBox"),
            #[widget] rb2 = RadioBox::new("radio box &2", radio)
                .with_state(true)
                .on_select(|_| Some(Item::Radio(2))),
            #[widget] cbbl = Label::new("ComboBox"),
            #[widget] cbb = ComboBox::new_from_iter(&["&One", "T&wo", "Th&ree"], 0)
                .on_select(|_, index| Some(Item::Combo((index + 1).cast()))),
            #[widget] sdl = Label::new("Slider"),
            #[widget(map_msg = handle_slider)] sd =
                Slider::<i32, Right>::new(0, 10, 1).with_value(0),
            #[widget] scl = Label::new("ScrollBar"),
            #[widget(map_msg = handle_scroll)] sc: ScrollBar<Right> =
                ScrollBar::new().with_limits(100, 20),
            #[widget] pg: ProgressBar<Right> = ProgressBar::new(),
            #[widget] pgl = Label::new("ProgressBar"),
            #[widget] svl = Label::new("SVG"),
            #[widget] sv = Svg::from_path_and_factors("res/rustacean-flat-happy.svg", 0.1, 0.3),
            #[widget] pul = Label::new("Child window"),
            #[widget] pu = popup_edit_box,
        }
        impl Self {
            fn handle_slider(&mut self, _: &mut EventMgr, msg: i32) -> Item {
                Item::Slider(msg)
            }
            fn handle_scroll(&mut self, mgr: &mut EventMgr, msg: i32) -> Item {
                let ratio = msg as f32 / self.sc.max_value() as f32;
                *mgr |= self.pg.set_value(ratio);
                Item::Scroll(msg)
            }
        }
    };

    let head = make_widget! {
        #[widget{
            layout = row: *;
        }]
        #[handler(msg = VoidMsg)]
        struct {
            #[widget] _ = Label::new("Widget Gallery"),
            #[widget] _ = Image::new("res/gallery.png"),
        }
    };

    let mut window = Window::new(
        "Widget Gallery",
        make_widget! {
            #[widget{
                layout = column: [
                    self.menubar,
                    align(center): self.head,
                    self.gallery,
                ];
            }]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(use_msg = menu)] menubar = menubar,
                #[widget] head = Frame::new(head),
                #[widget(use_msg = activations)] gallery:
                    for<W: Widget<Msg = Item>> ScrollBarRegion<W> =
                        ScrollBarRegion::new(widgets),
            }
            impl Self {
                fn menu(&mut self, mgr: &mut EventMgr, msg: Menu) {
                    match msg {
                        Menu::Theme(name) => {
                            println!("Theme: {:?}", name);
                            #[cfg(not(feature = "stack_dst"))]
                            println!("Warning: switching themes requires feature 'stack_dst'");

                            mgr.adjust_theme(|theme| theme.set_theme(name));
                        }
                        Menu::Colour(name) => {
                            println!("Colour scheme: {:?}", name);
                            mgr.adjust_theme(|theme| theme.set_scheme(&name));
                        }
                        Menu::Disabled(state) => {
                            mgr.set_disabled(self.gallery.inner().id(), state);
                        }
                        Menu::Quit => {
                            *mgr |= TkAction::EXIT;
                        }
                    }
                }
                fn activations(&mut self, mgr: &mut EventMgr, item: Item) {
                    match item {
                        Item::Button => println!("Clicked!"),
                        Item::LightTheme => mgr.adjust_theme(|theme| theme.set_scheme("light")),
                        Item::DarkTheme => mgr.adjust_theme(|theme| theme.set_scheme("dark")),
                        Item::Check(b) => println!("CheckBox: {}", b),
                        Item::Combo(c) => println!("ComboBox: {}", c),
                        Item::Radio(id) => println!("RadioBox: {}", id),
                        Item::Edit(s) => println!("Edited: {}", s),
                        Item::Slider(p) => println!("Slider: {}", p),
                        Item::Scroll(p) => println!("ScrollBar: {}", p),
                    };
                }
            }
        },
    );
    if let Err(err) = window.load_icon_from_path("res/gallery.png") {
        println!("Failed to load window icon: {}", err);
    }

    toolkit.add(window)?;
    toolkit.run()
}
