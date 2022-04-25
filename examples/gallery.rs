// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
//!
//! This is a test-bed to demonstrate most toolkit functionality
//! (excepting custom graphics).

use kas::event::Command;
use kas::event::VirtualKeyCode as VK;
use kas::prelude::*;
use kas::resvg::Svg;
use kas::widgets::*;
use kas::{dir::Right, Future};

#[derive(Clone, Debug)]
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
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        mgr.push_msg(Item::Edit(edit.get_string()));
    }

    fn edit(edit: &mut EditField<Self>, _: &mut EventMgr) {
        // 7a is the colour of *magic*!
        edit.set_error_state(edit.get_str().len() % (7 + 1) == 0);
    }
}

#[derive(Clone, Debug)]
struct MsgClose(bool);

impl_scope! {
    #[derive(Debug)]
    #[widget{
        layout = grid: {
            0..3, 0: self.edit;
            0, 1: self.fill; 1, 1: self.cancel; 2, 1: self.save;
        };
    }]
    struct TextEditPopup {
        #[widget_core]
        core: CoreData,
        #[widget] edit: EditBox,
        #[widget] fill: Filler,
        #[widget] cancel: TextButton,
        #[widget] save: TextButton,
        commit: bool,
    }
    impl TextEditPopup {
        fn new<S: ToString>(text: S) -> Self {
            TextEditPopup {
                core: Default::default(),
                edit: EditBox::new(text).multi_line(true),
                fill: Filler::maximize(),
                cancel: TextButton::new_msg("&Cancel", MsgClose(false)),
                save: TextButton::new_msg("&Save", MsgClose(true)),
                commit: false,
            }
        }

        fn close(&mut self, mgr: &mut EventMgr, commit: bool) -> Response {
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
        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(Command::Escape, _) => self.close(mgr, false),
                Event::Command(Command::Return, _) => self.close(mgr, true),
                _ => Response::Unused,
            }
        }
        fn on_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(MsgClose(commit)) = mgr.try_pop_msg() {
                let _ = self.close(mgr, commit);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    #[cfg(feature = "stack_dst")]
    let theme = kas::theme::MultiTheme::builder()
        .add("flat", kas::theme::FlatTheme::new())
        .add("shaded", kas::theme::ShadedTheme::new())
        .build();
    #[cfg(not(feature = "stack_dst"))]
    let theme = kas::theme::ShadedTheme::new();
    let mut toolkit = kas::shell::Toolkit::new(theme)?;

    // A real app might use async loading of resources here (Svg permits loading
    // from a data slice; DrawShared allows allocation from data slice).
    let img_light = Svg::new(include_bytes!("../res/contrast-2-line.svg"));
    let img_dark = Svg::new(include_bytes!("../res/contrast-2-fill.svg"));
    let img_gallery = Svg::new(include_bytes!("../res/gallery-line.svg"));
    const SVG_WARNING: &'static [u8] = include_bytes!("../res/error-warning-line.svg");
    let img_rustacean = match Svg::new_path("res/rustacean-flat-happy.svg") {
        Ok(svg) => svg,
        Err(e) => {
            println!("Failed to load res/rustacean-flat-happy.svg: {}", e);
            Svg::new(SVG_WARNING)
        }
    };

    #[derive(Clone, Debug)]
    enum Menu {
        Theme(&'static str),
        Colour(String),
        Disabled(bool),
        Quit,
    }

    let menubar = menu::MenuBar::<Right>::builder()
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
            .toggle("&Disabled", |mgr, state| {
                mgr.push_msg(Menu::Disabled(state))
            });
        })
        .build();

    let popup_edit_box = make_widget! {
        #[widget{
            layout = row: *;
        }]
        struct {
            #[widget] label: StringLabel = Label::from("Use button to edit →"),
            #[widget] edit = TextButton::new("&Edit"),
            future: Option<Future<Option<String>>> = None,
        }
        impl Handler for Self {
            fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
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
            fn on_message(&mut self, mgr: &mut EventMgr, index: usize) {
                if index == widget_index![self.edit] {
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
        }
    };

    let text = "Example text in multiple languages.
مثال على نص بلغات متعددة.
Пример текста на нескольких языках.
טקסט לדוגמא במספר שפות.";

    let radio = RadioBoxGroup::default();
    let widgets = make_widget! {
        #[widget{
            layout = aligned_column: [
                row: ["ScrollLabel", self.sl],
                row: ["EditBox", self.eb],
                row: ["TextButton", self.tb],
                row: ["Button<Image>", self.bi],
                row: ["CheckBox", self.cb],
                row: ["RadioBox", self.rb],
                row: ["RadioBox", self.rb2],
                row: ["ComboBox", self.cbb],
                row: ["Slider", self.sd],
                row: ["ScrollBar", self.sc],
                row: ["ProgressBar", self.pg],
                row: ["SVG", align(center): self.sv],
                row: ["Child window", self.pu],
            ];
        }]
        struct {
            #[widget] sl = ScrollLabel::new(text),
            #[widget] eb = EditBox::new("edit me").with_guard(Guard),
            #[widget] tb = TextButton::new_msg("&Press me", Item::Button),
            #[widget] bi = row![
                Button::new_msg(img_light, Item::LightTheme)
                    .with_color("#FAFAFA".parse().unwrap())
                    .with_keys(&[VK::L]),
                Button::new_msg(img_dark, Item::DarkTheme)
                    .with_color("#404040".parse().unwrap())
                    .with_keys(&[VK::K]),
            ],
            #[widget] cb = CheckBox::new("&Check me")
                .with_state(true)
                .on_toggle(|mgr, check| mgr.push_msg(Item::Check(check))),
            #[widget] rb = RadioBox::new("radio box &1", radio.clone())
                .on_select(|mgr| mgr.push_msg(Item::Radio(1))),
            #[widget] rb2 = RadioBox::new("radio box &2", radio)
                .with_state(true)
                .on_select(|mgr| mgr.push_msg(Item::Radio(2))),
            #[widget] cbb = ComboBox::new_from_iter(&["&One", "T&wo", "Th&ree"])
                .on_select(|mgr, index| mgr.push_msg(Item::Combo((index + 1).cast()))),
            #[widget] sd = Slider::<i32, Right>::new(0, 10, 1)
                .with_value(0)
                .map_msg(|msg: i32| Item::Slider(msg)),
            #[widget] sc: ScrollBar<Right> = ScrollBar::new().with_limits(100, 20),
            #[widget] pg: ProgressBar<Right> = ProgressBar::new(),
            #[widget] sv = img_rustacean.with_scaling(|s| {
                s.size = kas::layout::SpriteSize::Relative(0.1);
                s.ideal_factor = 2.0;
                s.stretch = kas::layout::Stretch::High;
            }),
            #[widget] pu = popup_edit_box,
        }
        impl Handler for Self {
            fn on_message(&mut self, mgr: &mut EventMgr, index: usize) {
                if index == widget_index![self.sc] {
                    if let Some(msg) = mgr.try_pop_msg::<i32>() {
                        let ratio = msg as f32 / self.sc.max_value() as f32;
                        *mgr |= self.pg.set_value(ratio);
                        mgr.push_msg(Item::Scroll(msg))
                    }
                }
            }
        }
    };

    let head = make_widget! {
        #[widget{
            layout = row: ["Widget Gallery", self.img];
        }]
        struct {
            #[widget] img = img_gallery,
        }
    };

    let window = Window::new(
        "Widget Gallery",
        make_widget! {
            #[widget{
                layout = column: [
                    self.menubar,
                    align(center): self.head,
                    self.gallery,
                ];
            }]
            struct {
                #[widget] menubar = menubar,
                #[widget] head = Frame::new(head),
                #[widget] gallery:
                    for<W: Widget> ScrollBarRegion<W> =
                        ScrollBarRegion::new(widgets),
            }
            impl Handler for Self {
                fn on_message(&mut self, mgr: &mut EventMgr, _: usize) {
                    if let Some(msg) = mgr.try_pop_msg::<Menu>() {
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
                    } else if let Some(item) = mgr.try_pop_msg::<Item>() {
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
                        }
                    }
                }
            }
        },
    );

    toolkit.add(window)?;
    toolkit.run()
}
