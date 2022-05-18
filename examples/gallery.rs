// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
//!
//! This is a test-bed to demonstrate most toolkit functionality
//! (excepting custom graphics).

use kas::dir::Right;
use kas::event::VirtualKeyCode as VK;
use kas::prelude::*;
use kas::resvg::Svg;
use kas::updatable::SharedRc;
use kas::widgets::{menu::MenuEntry, view::SingleView, *};

#[derive(Clone, Debug)]
enum Item {
    Button,
    Theme(&'static str),
    Check(bool),
    Combo(i32),
    Radio(u32),
    Edit(String),
    Slider(i32),
    Scroll(i32),
    Spinner(i32),
}

// Using a trait allows control of content
//
// We do not wish to disable navigation, but do with to disable controls.
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
trait SetDisabled: Widget {
    fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool);
}
impl<T: SetDisabled> SetDisabled for ScrollBarRegion<T> {
    fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool) {
        self.inner_mut().set_disabled(mgr, state);
    }
}
impl<T: SetDisabled> SetDisabled for TabStack<T> {
    fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool) {
        for index in 0..self.len() {
            if let Some(w) = self.get_mut(index) {
                w.set_disabled(mgr, state);
            }
        }
    }
}
impl<G: EditGuard> SetDisabled for EditBox<G> {
    fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool) {
        mgr.set_disabled(self.id(), state);
    }
}

fn widgets() -> Box<dyn SetDisabled> {
    // A real app might use async loading of resources here (Svg permits loading
    // from a data slice; DrawShared allows allocation from data slice).
    let img_light = Svg::new(include_bytes!("../res/contrast-2-line.svg"));
    let img_dark = Svg::new(include_bytes!("../res/contrast-2-fill.svg"));
    const SVG_WARNING: &'static [u8] = include_bytes!("../res/error-warning-line.svg");
    let img_rustacean = match Svg::new_path("res/rustacean-flat-happy.svg") {
        Ok(svg) => svg,
        Err(e) => {
            println!("Failed to load res/rustacean-flat-happy.svg: {}", e);
            Svg::new(SVG_WARNING)
        }
    };

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
    struct MsgEdit;

    let popup_edit_box = impl_singleton! {
        #[widget{
            layout = row: [
                self.label,
                TextButton::new_msg("&Edit", MsgEdit),
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] label: SingleView<SharedRc<String>> =
                SingleView::new(SharedRc::new("Use button to edit →".to_string())),
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(MsgEdit) = mgr.try_pop_msg() {
                    let text = self.label.data().clone();
                    let window = dialog::TextEdit::new("Edit text", true, text);
                    mgr.add_window(Box::new(window));
                }
            }
        }
    };

    let text = "Example text in multiple languages.
مثال على نص بلغات متعددة.
Пример текста на нескольких языках.
טקסט לדוגמא במספר שפות.";

    let radio = RadioBoxGroup::default();

    Box::new(ScrollBarRegion::new(impl_singleton! {
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
                row: ["Spinner", self.spin],
                row: ["Slider", self.sd],
                row: ["ScrollBar", self.sc],
                row: ["ProgressBar", self.pg],
                row: ["SVG", align(center): self.sv],
                row: ["Child window", self.pu],
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] sl = ScrollLabel::new(text),
            #[widget] eb = EditBox::new("edit me").with_guard(Guard),
            #[widget] tb = TextButton::new_msg("&Press me", Item::Button),
            #[widget] bi = Row::new_vec(vec![
                Button::new_msg(img_light.clone(), Item::Theme("light"))
                    .with_color("#B38DF9".parse().unwrap())
                    .with_keys(&[VK::L]),
                Button::new_msg(img_light, Item::Theme("blue"))
                    .with_color("#7CDAFF".parse().unwrap())
                    .with_keys(&[VK::L]),
                Button::new_msg(img_dark, Item::Theme("dark"))
                    .with_color("#E77346".parse().unwrap())
                    .with_keys(&[VK::K]),
            ]),
            #[widget] cb = CheckBox::new("&Check me")
                .with_state(true)
                .on_toggle(|mgr, check| mgr.push_msg(Item::Check(check))),
            #[widget] rb = RadioBox::new("radio box &1", radio.clone())
                .on_select(|mgr| mgr.push_msg(Item::Radio(1))),
            #[widget] rb2 = RadioBox::new("radio box &2", radio)
                .with_state(true)
                .on_select(|mgr| mgr.push_msg(Item::Radio(2))),
            #[widget] cbb = ComboBox::new_vec(vec![
                MenuEntry::new("&One", Item::Combo(1)),
                MenuEntry::new("T&wo", Item::Combo(2)),
                MenuEntry::new("Th&ree", Item::Combo(3)),
            ]),
            #[widget] spin: Spinner<i32> = Spinner::new(0..=10, 1),
            #[widget] sd: Slider<i32, Right> = Slider::new(0..=10, 1),
            #[widget] sc: ScrollBar<Right> = ScrollBar::new().with_limits(100, 20),
            #[widget] pg: ProgressBar<Right> = ProgressBar::new(),
            #[widget] sv = img_rustacean.with_scaling(|s| {
                s.size = kas::layout::SpriteSize::Relative(0.1);
                s.ideal_factor = 2.0;
                s.stretch = kas::layout::Stretch::High;
            }),
            #[widget] pu = popup_edit_box,
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
                if let Some(msg) = mgr.try_pop_msg::<i32>() {
                    if index == widget_index![self.spin] {
                        *mgr |= self.sd.set_value(msg);
                        mgr.push_msg(Item::Spinner(msg));
                    } else if index == widget_index![self.sd] {
                        *mgr |= self.spin.set_value(msg);
                        mgr.push_msg(Item::Slider(msg));
                    } else if index == widget_index![self.sc] {
                        let ratio = msg as f32 / self.sc.max_value() as f32;
                        *mgr |= self.pg.set_value(ratio);
                        mgr.push_msg(Item::Scroll(msg))
                    }
                }
            }
        }
        impl SetDisabled for Self {
            fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool) {
                mgr.set_disabled(self.id(), state);
            }
        }
    }))
}

fn editor() -> Box<dyn SetDisabled> {
    Box::new(EditBox::new("").multi_line(true))
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

    // TODO: use as logo of tab
    // let img_gallery = Svg::new(include_bytes!("../res/gallery-line.svg"));

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

    let window = impl_singleton! {
        #[widget{
            layout = column: [
                self.menubar,
                self.stack,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] menubar = menubar,
            #[widget] stack: TabStack<Box<dyn SetDisabled>> = TabStack::new()
                .with_title("Widgets", widgets()) //TODO: use img_gallery as logo
                .with_title("Text editor", editor()),
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
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
                            self.stack.set_disabled(mgr, state);
                        }
                        Menu::Quit => {
                            *mgr |= TkAction::EXIT;
                        }
                    }
                } else if let Some(item) = mgr.try_pop_msg::<Item>() {
                    println!("Message: {item:?}");
                    match item {
                        Item::Theme(name) => mgr.adjust_theme(|theme| theme.set_scheme(name)),
                        _ => (),
                    }
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str {
                "Widget Gallery"
            }
        }
    };

    toolkit.add(window)?;
    toolkit.run()
}
