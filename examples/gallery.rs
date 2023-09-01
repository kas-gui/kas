// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
//!
//! This is a test-bed to demonstrate most toolkit functionality
//! (excepting custom graphics).

use kas::dir::Right;
use kas::event::Key;
use kas::prelude::*;
use kas::resvg::Svg;
use kas::theme::{MarginStyle, ThemeControl};
use kas::widgets::*;

#[derive(Debug, Default)]
struct AppData {
    disabled: bool,
}

fn widgets() -> Box<dyn Widget<Data = AppData>> {
    use kas::widgets::dialog::{TextEdit, TextEditResult};
    use kas::Popup;

    // A real app might use async loading of resources here (Svg permits loading
    // from a data slice; DrawShared allows allocation from data slice).
    let img_light = Svg::new(include_bytes!("../res/contrast-2-line.svg"))
        .unwrap()
        .with_scaling(|scaling| scaling.margins = MarginStyle::Tiny);
    let img_dark = Svg::new(include_bytes!("../res/contrast-2-fill.svg"))
        .unwrap()
        .with_scaling(|scaling| scaling.margins = MarginStyle::Tiny);
    const SVG_WARNING: &[u8] = include_bytes!("../res/error-warning-line.svg");
    let img_rustacean = match Svg::new_path("res/rustacean-flat-happy.svg") {
        Ok(svg) => svg,
        Err(e) => {
            println!("Failed to load res/rustacean-flat-happy.svg: {e}");
            Svg::new(SVG_WARNING).unwrap()
        }
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[impl_default(Entry::One)]
    enum Entry {
        One,
        Two,
        Three,
    }
    let entries = [
        ("&One", Entry::One),
        ("T&wo", Entry::Two),
        ("Th&ree", Entry::Three),
    ];

    #[derive(Clone, Debug)]
    enum Item {
        Button,
        Theme(&'static str),
        Check(bool),
        Combo(Entry),
        Radio(u32),
        Edit(String),
        Slider(i32),
        Spinner(i32),
        Text(String),
    }

    impl_scope! {
        #[derive(Debug)]
        #[impl_default]
        struct Data {
            check: bool = true,
            radio: u32 = 1,
            value: i32 = 5,
            entry: Entry,
            ratio: f32 = 0.0,
            text: String,
        }
    }
    let data = Data {
        text: "Use button to edit →".to_string(),
        ..Default::default()
    };

    struct Guard;
    impl EditGuard for Guard {
        type Data = Data;

        fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Data) -> Response {
            cx.push(Item::Edit(edit.get_string()));
            Used
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Data) {
            // 7a is the colour of *magic*!
            *cx |= edit.set_error_state(edit.get_str().len() % (7 + 1) == 0);
        }
    }

    #[derive(Clone, Debug)]
    struct MsgEdit;

    let popup_edit_box = singleton! {
        #[widget{
            layout = row! [
                format_data!(data: &Data, "{}", &data.text),
                Button::label_msg("&Edit", MsgEdit).map_any(),
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget(&())] popup: Popup<TextEdit> = Popup::new(TextEdit::new("", true), Direction::Down),
        }
        impl Events for Self {
            type Data = Data;

            fn handle_messages(&mut self, cx: &mut EventCx, data: &Data) {
                if let Some(MsgEdit) = cx.try_pop() {
                    // TODO: do not always set text: if this is a true pop-up it
                    // should not normally lose data.
                    *cx |= self.popup.set_text(data.text.clone());
                    // let ed = TextEdit::new(text, true);
                    // cx.add_window::<()>(ed.into_window("Edit text"));
                    // TODO: cx.add_modal(..)
                    self.popup.open(cx, &(), self.id());
                } else if let Some(result) = cx.try_pop() {
                    match result {
                        TextEditResult::Cancel => (),
                        TextEditResult::Ok(text) => {
                            // Translate from TextEdit's output
                            cx.push(Item::Text(text));
                        }
                    }
                    self.popup.close(cx);
                }
            }
        }
    };

    let text = "Example text in multiple languages.
مثال على نص بلغات متعددة.
Пример текста на нескольких языках.
טקסט לדוגמא במספר שפות.";

    let widgets = kas::aligned_column![
        row!["ScrollLabel", ScrollLabel::new(text).map_any()],
        row![
            "EditBox",
            EditBox::string(|data: &Data| data.text.clone())
                .with_msg(|s| Item::Text(s.to_string())),
        ],
        row![
            "Button (text)",
            Button::label_msg("&Press me", Item::Button).map_any()
        ],
        row![
            "Button (image)",
            pack!(
                center,
                kas::row![
                    Button::new_msg(img_light.clone(), Item::Theme("light"))
                        .with_color("#B38DF9".parse().unwrap())
                        .with_access_key(Key::Character("h".into())),
                    Button::new_msg(img_light, Item::Theme("blue"))
                        .with_color("#7CDAFF".parse().unwrap())
                        .with_access_key(Key::Character("b".into())),
                    Button::new_msg(img_dark, Item::Theme("dark"))
                        .with_color("#E77346".parse().unwrap())
                        .with_access_key(Key::Character("k".into())),
                ]
                .map_any()
            )
        ],
        row![
            "CheckButton",
            CheckButton::new_msg("&Check me", |_, data: &Data| data.check, Item::Check)
        ],
        row![
            "RadioButton",
            RadioButton::new_msg(
                "radio button &1",
                |_, data: &Data| data.radio == 1,
                || Item::Radio(1)
            ),
        ],
        row![
            "RadioButton",
            RadioButton::new_msg(
                "radio button &2",
                |_, data: &Data| data.radio == 2,
                || Item::Radio(2)
            ),
        ],
        row![
            "ComboBox",
            ComboBox::new(entries, |_, data: &Data| data.entry).with_msg(|m| Item::Combo(m))
        ],
        row![
            "Spinner",
            Spinner::new_msg(0..=10, |_, data: &Data| data.value, Item::Spinner)
        ],
        row![
            "Slider",
            Slider::right(0..=10, |_, data: &Data| data.value).with_msg(Item::Slider)
        ],
        row![
            "ScrollBar",
            ScrollBar::right().with_limits(100, 20).map_any()
        ],
        row![
            "ProgressBar",
            ProgressBar::right(|_, data: &Data| data.ratio)
        ],
        row![
            "SVG",
            img_rustacean
                .with_scaling(|s| {
                    s.min_factor = 0.1;
                    s.ideal_factor = 0.2;
                    s.stretch = kas::layout::Stretch::High;
                })
                .map_any()
        ],
        row!["Child window", popup_edit_box],
    ];

    let ui = Adapt::new(widgets, data).on_messages(|cx, _, data| {
        if let Some(ScrollMsg(value)) = cx.try_pop() {
            println!("ScrollMsg({value})");
            data.ratio = value as f32 / 100.0;
            true
        } else if let Some(item) = cx.try_pop() {
            println!("Message: {item:?}");
            match item {
                Item::Check(v) => data.check = v,
                Item::Radio(radio) => data.radio = radio,
                Item::Combo(m) => data.entry = m,
                Item::Spinner(value) | Item::Slider(value) => {
                    data.value = value;
                }
                Item::Theme(name) => cx.adjust_theme(|theme| theme.set_scheme(name)),
                Item::Text(text) => data.text = text,
                _ => (),
            }
            true
        } else {
            false
        }
    });

    let ui = adapt::OnUpdate::new(ui)
        .on_update(|cx, w, data: &AppData| cx.set_disabled(w.id(), data.disabled));

    Box::new(ScrollBarRegion::new(ui))
}

fn editor() -> Box<dyn Widget<Data = AppData>> {
    use kas::text::format::Markdown;

    #[derive(Clone, Debug)]
    struct MsgDirection;

    struct Guard;
    impl EditGuard for Guard {
        type Data = AppData;

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &AppData) {
            cx.set_disabled(edit.id(), data.disabled);
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &AppData) {
            let result = Markdown::new(edit.get_str());
            *cx |= edit.set_error_state(result.is_err());
            cx.push(result.unwrap_or_else(|err| Markdown::new(&format!("{err}")).unwrap()));
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

    Box::new(singleton! {
        #[widget{
            layout = float! [
                pack!(right top, Button::label_msg("↻", MsgDirection).map_any()),
                list!(self.dir, [self.editor, non_navigable!(self.label)]),
            ];
        }]
        struct {
            core: widget_core!(),
            dir: Direction = Direction::Up,
            #[widget] editor: EditBox<Guard> =
                EditBox::new(Guard)
                    .with_multi_line(true)
                    .with_lines(4, 12)
                    .with_text(doc),
            #[widget(&())] label: ScrollLabel<Markdown> =
                ScrollLabel::new(Markdown::new(doc).unwrap()),
        }

        impl Events for Self {
            type Data = AppData;

            fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
                if let Some(MsgDirection) = cx.try_pop() {
                    self.dir = match self.dir {
                        Direction::Up => Direction::Right,
                        _ => Direction::Up,
                    };
                    *cx |= Action::RESIZE;
                } else if let Some(md) = cx.try_pop::<Markdown>() {
                    *cx |= self.label.set_text(md);
                }
            }
        }
    })
}

fn filter_list() -> Box<dyn Widget<Data = AppData>> {
    use kas::view::{filter, Driver, ListView, SelectionMode, SelectionMsg};

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

    #[derive(Debug)]
    struct Data {
        mode: SelectionMode,
        list: Vec<String>,
    }
    let data = Data {
        mode: SelectionMode::None,
        list: (2019..=2023)
            .flat_map(|year| MONTHS.iter().map(move |m| format!("{m} {year}")))
            .collect(),
    };

    struct ListGuard;
    type FilteredList = filter::UnsafeFilteredList<Vec<String>>;
    impl Driver<String, FilteredList> for ListGuard {
        type Widget = NavFrame<Text<String, String>>;
        fn make(&mut self, _: &usize) -> Self::Widget {
            Default::default()
        }
    }
    let filter = filter::ContainsCaseInsensitive::new();
    let guard = filter::KeystrokeGuard;
    let list_view = filter::FilterBoxList::new(ListView::down(ListGuard), filter, guard)
        .map(|data: &Data| &data.list)
        .on_update(|cx, list, data| {
            *cx |= list.set_selection_mode(data.mode);
        });

    let ui = kas::column![
        kas::row![
            "Selection:",
            RadioButton::new_value("&n&one", SelectionMode::None),
            RadioButton::new_value("s&ingle", SelectionMode::Single),
            RadioButton::new_value("&multiple", SelectionMode::Multiple),
        ]
        .map(|data: &Data| &data.mode),
        ScrollBars::new(list_view),
    ];
    let ui = Adapt::new(ui, data)
        .on_message(|_, data, mode| data.mode = mode)
        .on_message(|_, data, selection: SelectionMsg<usize>| match selection {
            SelectionMsg::Select(i) => println!("Selected: {}", &data.list[i]),
            _ => (),
        });
    let ui = adapt::OnUpdate::new(ui)
        .on_update(|cx, w, data: &AppData| cx.set_disabled(w.id(), data.disabled));
    Box::new(ui)
}

fn canvas() -> Box<dyn Widget<Data = AppData>> {
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

        fn need_redraw(&mut self) -> bool {
            // Set false to disable animation
            true
        }
    }

    let ui = kas::column![
        Label::new(
            "Animated canvas demo (CPU-rendered, async). Note: scheduling is broken on X11."
        ),
        Canvas::new(Program(Instant::now())),
    ];
    Box::new(ui.map_any())
}

fn config() -> Box<dyn Widget<Data = AppData>> {
    let desc = kas::text::format::Markdown::new(
        "\
Event configuration editor
================

Updated items should have immediate effect.

To persist, set the following environment variables:
```
KAS_CONFIG=config.yaml
KAS_CONFIG_MODE=readwrite
```
",
    )
    .unwrap();

    let ui = kas::column![ScrollLabel::new(desc), Separator::new(), EventConfig::new(),]
        .map_any()
        .on_update(|cx, w, data: &AppData| cx.set_disabled(w.id(), data.disabled));
    Box::new(ui)
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas::theme::MultiTheme::builder()
        .add("flat", kas::theme::FlatTheme::new())
        .add("simple", kas::theme::SimpleTheme::new())
        .add("shaded", kas_wgpu::ShadedTheme::new())
        .build();
    let mut shell = kas::shell::DefaultShell::new((), theme)?;

    // TODO: use as logo of tab
    // let img_gallery = Svg::new(include_bytes!("../res/gallery-line.svg"));

    #[derive(Clone, Debug)]
    enum Menu {
        Theme(&'static str),
        Colour(String),
        Disabled(bool),
        Quit,
    }

    let menubar = menu::MenuBar::<AppData, Right>::builder()
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
                // Enumerate colour schemes. Access through the shell since
                // this handles config loading.
                for name in shell.theme().list_schemes().iter() {
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
            .toggle(
                "&Disabled",
                |_, data| data.disabled,
                |state| Menu::Disabled(state),
            );
        })
        .build();

    let ui = singleton! {
        #[widget{
            layout = column! [
                self.menubar,
                Separator::new(),
                self.stack,
            ];
        }]
        struct {
            core: widget_core!(),
            state: AppData,
            #[widget(&self.state)] menubar: menu::MenuBar::<AppData, Right> = menubar,
            #[widget(&self.state)] stack: TabStack<Box<dyn Widget<Data = AppData>>> = TabStack::from([
                ("&Widgets", widgets()), //TODO: use img_gallery as logo
                ("Te&xt editor", editor()),
                ("&List", filter_list()),
                ("Can&vas", canvas()),
                ("Confi&g", config()),
            ]).with_msg(|_, title| WindowCommand::SetTitle(format!("Gallery — {}", title))),
        }
        impl Events for Self {
            type Data = ();

            fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
                if let Some(msg) = cx.try_pop::<Menu>() {
                    match msg {
                        Menu::Theme(name) => {
                            println!("Theme: {name:?}");
                            cx.adjust_theme(|theme| theme.set_theme(name));
                        }
                        Menu::Colour(name) => {
                            println!("Colour scheme: {name:?}");
                            cx.adjust_theme(|theme| theme.set_scheme(&name));
                        }
                        Menu::Disabled(state) => {
                            self.state.disabled = state;
                            cx.update(self.as_node(&()));
                        }
                        Menu::Quit => {
                            *cx |= Action::EXIT;
                        }
                    }
                }
            }
        }
    };

    shell.add(Window::new(ui, "Gallery — Widgets"));
    shell.run()
}
