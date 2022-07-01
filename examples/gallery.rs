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
use kas::model::SharedRc;
use kas::prelude::*;
use kas::resvg::Svg;
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

    let radio = RadioGroup::default();

    let widgets = impl_singleton! {
        #[widget{
            layout = aligned_column: [
                row: ["ScrollLabel", self.sl],
                row: ["EditBox", self.eb],
                row: ["TextButton", self.tb],
                row: ["Button<Image>", self.bi],
                row: ["CheckButton", self.cb],
                row: ["RadioButton", self.rb],
                row: ["RadioButton", self.rb2],
                row: ["ComboBox", self.cbb],
                row: ["Spinner", self.spin],
                row: ["Slider", self.sd],
                row: ["ScrollBar", self.sc],
                row: ["ProgressBar", self.pg],
                row: ["SVG", self.sv],
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
            #[widget] cb = CheckButton::new("&Check me")
                .with_state(true)
                .on_toggle(|mgr, check| mgr.push_msg(Item::Check(check))),
            #[widget] rb = RadioButton::new("radio button &1", radio.clone())
                .on_select(|mgr| mgr.push_msg(Item::Radio(1))),
            #[widget] rb2 = RadioButton::new("radio button &2", radio)
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
                s.min_factor = 0.1;
                s.ideal_factor = 0.2;
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
    };
    Box::new(ScrollBarRegion::new(widgets).with_invisible_bars(true, true))
}

fn editor() -> Box<dyn SetDisabled> {
    use kas::text::format::Markdown;

    #[derive(Clone, Debug)]
    struct MsgDirection;

    #[derive(Debug)]
    struct Guard;
    impl EditGuard for Guard {
        fn edit(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
            let result = Markdown::new(edit.get_str());
            edit.set_error_state(result.is_err());
            mgr.push_msg(result.unwrap_or_else(|err| Markdown::new(&format!("{}", err)).unwrap()));
        }
    }

    let doc = r"# Formatted text editing

Demonstration of *as-you-type* formatting from **Markdown**.

1. Edit below
2. View the result
3. In case of error, be informed

### Not all Markdown supported
```
> Block quotations

<h3>HTML</h3>

-----------------
```
";

    Box::new(impl_singleton! {
        #[widget{
            layout = float: [
                align(right, top): TextButton::new_msg("↻", MsgDirection),
                list(self.dir): [self.editor, self.label],
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            dir: Direction = Direction::Up,
            #[widget] editor: EditBox<Guard> =
                EditBox::new(doc).multi_line(true).with_guard(Guard),
            #[widget] label: ScrollLabel<Markdown> =
                ScrollLabel::new(Markdown::new(doc).unwrap()),
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(MsgDirection) = mgr.try_pop_msg() {
                    self.dir = match self.dir {
                        Direction::Up => Direction::Right,
                        _ => Direction::Up,
                    };
                    *mgr |= TkAction::RESIZE;
                } else if let Some(md) = mgr.try_pop_msg::<Markdown>() {
                    *mgr |= self.label.set_text(md);
                }
            }
        }
        impl SetDisabled for Self {
            fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool) {
                mgr.set_disabled(self.id(), state);
            }
        }
    })
}

fn filter_list() -> Box<dyn SetDisabled> {
    use kas::dir::Down;
    use kas::model::{filter::ContainsCaseInsensitive, SingleData};
    use kas::widgets::view::{driver, SelectionMode, SelectionMsg};

    const MONTHS: &[&str] = &[
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let data: Vec<String> = (2019..=2022)
        .flat_map(|year| MONTHS.iter().map(move |m| format!("{m} {year}")))
        .collect();

    let filter = ContainsCaseInsensitive::new("");
    type FilteredList = view::FilteredList<Vec<String>, ContainsCaseInsensitive>;
    type ListView = view::ListView<Down, FilteredList, driver::DefaultNav>;
    let filtered = FilteredList::new(data, filter.clone());

    let r = RadioGroup::default();

    Box::new(impl_singleton! {
        #[widget{
            layout = column: [
                row: ["Selection:", self.r0, self.r1, self.r2],
                row: ["Filter:", self.filter],
                self.list,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] r0 = RadioButton::new_msg("none", r.clone(), SelectionMode::None).with_state(true),
            #[widget] r1 = RadioButton::new_msg("single", r.clone(), SelectionMode::Single),
            #[widget] r2 = RadioButton::new_msg("multiple", r, SelectionMode::Multiple),
            #[widget] filter = EditBox::new("")
                .on_edit(move |s, mgr| filter.update(mgr, s.to_string())),
            #[widget] list: ScrollBars<ListView> =
                ScrollBars::new(ListView::new(filtered))
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(mode) = mgr.try_pop_msg() {
                    *mgr |= self.list.set_selection_mode(mode);
                } else if let Some(msg) = mgr.try_pop_msg::<SelectionMsg<usize>>() {
                    println!("Selection message: {:?}", msg);
                }
            }
        }
        impl SetDisabled for Self {
            fn set_disabled(&mut self, mgr: &mut EventMgr, state: bool) {
                mgr.set_disabled(self.id(), state);
            }
        }
    })
}

fn canvas() -> Box<dyn SetDisabled> {
    use kas::geom::Vec2;
    use kas_resvg::tiny_skia::*;
    use kas_resvg::{Canvas, CanvasProgram};
    use std::time::Instant;

    #[derive(Debug)]
    struct Program(Instant);
    impl CanvasProgram for Program {
        fn draw(&mut self, pixmap: &mut Pixmap) {
            let size = (200.0, 200.0);
            let scale = Transform::from_scale(
                f32::conv(pixmap.width()) / size.0,
                f32::conv(pixmap.height()) / size.1,
            );

            let paint = Paint {
                shader: LinearGradient::new(
                    Point::from_xy(0.0, 0.0),
                    Point::from_xy(size.0, size.1),
                    vec![
                        GradientStop::new(0.0, Color::BLACK),
                        GradientStop::new(1.0, Color::from_rgba8(0, 255, 200, 255)),
                    ],
                    SpreadMode::Pad,
                    Transform::identity(),
                )
                .unwrap(),
                ..Default::default()
            };

            let p = Vec2(110.0, 90.0);
            let t = self.0.elapsed().as_secs_f32();
            let c = t.cos();
            let s = t.sin();

            let mut vv = [
                Vec2(-90.0, -40.0),
                Vec2(-50.0, -30.0),
                Vec2(-30.0, 20.0),
                Vec2(-30.0, -5.0),
                Vec2(-10.0, -30.0),
                Vec2(-50.0, -50.0),
            ];
            for v in &mut vv {
                *v = p + Vec2(c * v.0 - s * v.1, s * v.0 + c * v.1);
            }

            let mut path = PathBuilder::new();
            path.push_circle(p.0 + 10.0, p.1, 100.0);
            path.push_circle(p.0, p.1, 50.0);
            let path = path.finish().unwrap();
            pixmap.fill_path(&path, &paint, FillRule::EvenOdd, scale, None);

            let path = PathBuilder::from_circle(30.0, 180.0, 20.0).unwrap();
            pixmap.fill_path(&path, &paint, FillRule::Winding, scale, None);

            let mut paint = Paint::default();
            paint.set_color_rgba8(230, 90, 50, 255);
            let mut path = PathBuilder::new();
            path.move_to(vv[0].0, vv[0].1);
            path.quad_to(vv[1].0, vv[1].1, vv[2].0, vv[2].1);
            path.quad_to(vv[3].0, vv[3].1, vv[4].0, vv[4].1);
            path.quad_to(vv[5].0, vv[5].1, vv[0].0, vv[0].1);
            let path = path.finish().unwrap();
            pixmap.fill_path(&path, &paint, FillRule::Winding, scale, None);
        }

        fn do_redraw_animate(&mut self) -> (bool, bool) {
            // Set false to disable animation
            (true, true)
        }
    }

    Box::new(impl_singleton! {
        #[widget{
            layout = column: [
                "Animated canvas demo",
                "This example uses kas_resvg::Canvas, which is CPU-rendered.",
                "Embedded GPU-rendered content is also possible (see separate Mandlebrot example).",
                self.canvas,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] canvas = Canvas::new(Program(Instant::now())),
        }
        impl SetDisabled for Self {
            fn set_disabled(&mut self, _: &mut EventMgr, _: bool) {}
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    #[cfg(feature = "stack_dst")]
    let theme = kas::theme::MultiTheme::builder()
        .add("flat", kas::theme::FlatTheme::new())
        .add("simple", kas::theme::SimpleTheme::new())
        .add("shaded", kas::theme::ShadedTheme::new())
        .build();
    #[cfg(not(feature = "stack_dst"))]
    let theme = kas::theme::FlatTheme::new();
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
            menu.entry("&Simple", Menu::Theme("simple"))
                .entry("&Flat", Menu::Theme("flat"))
                .entry("S&haded", Menu::Theme("shaded"));
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
                .with_title("Text editor", editor())
                .with_title("List", filter_list())
                .with_title("Canvas", canvas()),
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
