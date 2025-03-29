// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
//!
//! This is a test-bed to demonstrate most toolkit functionality
//! (excepting custom graphics).

use kas::collection;
use kas::config::{ConfigMsg, ThemeConfigMsg};
use kas::dir::Right;
use kas::event::Key;
use kas::prelude::*;
use kas::resvg::Svg;
use kas::theme::MarginStyle;
use kas::widgets::{column, *};

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

    #[allow(unused)]
    #[derive(Clone, Debug)]
    enum Item {
        Button,
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

        fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Data) -> IsUsed {
            cx.push(Item::Edit(edit.clone_string()));
            Used
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Data) {
            // 7a is the colour of *magic*!
            edit.set_error_state(cx, edit.as_str().len() % (7 + 1) == 0);
        }
    }

    #[derive(Clone, Debug)]
    struct MsgEdit;

    let popup_edit_box = impl_anon! {
        #[widget{
            layout = row! [
                self.text,
                Button::label_msg("&Edit", MsgEdit),
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget] text: Text<Data, String> = format_data!(data: &Data, "{}", &data.text),
            #[widget(&())] popup: Popup<TextEdit> = Popup::new(TextEdit::new("", true), Direction::Down),
        }
        impl Events for Self {
            type Data = Data;

            fn handle_messages(&mut self, cx: &mut EventCx, data: &Data) {
                if let Some(MsgEdit) = cx.try_pop() {
                    // TODO: do not always set text: if this is a true pop-up it
                    // should not normally lose data.
                    self.popup.inner.set_text(cx, data.text.clone());
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

    let widgets = aligned_column![
        row!["ScrollLabel", ScrollLabel::new(text).map_any()],
        row![
            "EditBox",
            EditBox::new(Guard).with_text("length must not be a multiple of 8!"),
        ],
        row![
            "Button (text)",
            Button::label_msg("&Press me", Item::Button).map_any()
        ],
        row![
            "Button (image)",
            row![
                Button::new_msg(
                    img_light.clone(),
                    ConfigMsg::Theme(ThemeConfigMsg::SetActiveScheme("light".to_string()))
                )
                .with_background("#B38DF9".parse().unwrap())
                .with_access_key(Key::Character("h".into())),
                Button::new_msg(
                    img_light,
                    ConfigMsg::Theme(ThemeConfigMsg::SetActiveScheme("blue".to_string()))
                )
                .with_background("#7CDAFF".parse().unwrap())
                .with_access_key(Key::Character("b".into())),
                Button::new_msg(
                    img_dark,
                    ConfigMsg::Theme(ThemeConfigMsg::SetActiveScheme("dark".to_string()))
                )
                .with_background("#E77346".parse().unwrap())
                .with_access_key(Key::Character("k".into())),
            ]
            .map_any()
            .pack(AlignHints::CENTER),
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
                    s.stretch = Stretch::High;
                })
                .map_any()
        ],
        row!["Child window", popup_edit_box],
    ];

    let ui = widgets
        .with_state(data)
        .on_message(|_, data, ScrollMsg(value)| {
            println!("ScrollMsg({value})");
            data.ratio = value as f32 / 100.0;
        })
        .on_message(|_, data, item| {
            println!("Message: {item:?}");
            match item {
                Item::Check(v) => data.check = v,
                Item::Radio(radio) => data.radio = radio,
                Item::Combo(m) => data.entry = m,
                Item::Spinner(value) | Item::Slider(value) => {
                    data.value = value;
                }
                Item::Text(text) => data.text = text,
                _ => (),
            }
        })
        .on_message(|cx, _, msg| {
            println!("Message: {msg:?}");
            let act = cx.config().change_config(msg);
            cx.window_action(act);
        });

    let ui = adapt::AdaptEvents::new(ui)
        .on_update(|cx, _, data: &AppData| cx.set_disabled(data.disabled));

    Box::new(ScrollBarRegion::new(ui))
}

fn editor() -> Box<dyn Widget<Data = AppData>> {
    use kas::text::format::Markdown;

    #[derive(Clone, Debug)]
    struct MsgDirection;

    #[derive(Clone, Debug)]
    struct SetLabelId(Id);

    impl_scope! {
        #[derive(Debug)]
        #[impl_default]
        struct Data {
            dir: Direction = Direction::Up,
            disabled: bool,
            label_id: Id,
        }
    }

    struct Guard;
    impl EditGuard for Guard {
        type Data = Data;

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &Data) {
            cx.set_disabled(edit.id(), data.disabled);
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Data) {
            let result = Markdown::new(edit.as_str());
            edit.set_error_state(cx, result.is_err());
            let text = result.unwrap_or_else(|err| Markdown::new(&format!("{err}")).unwrap());
            cx.send(data.label_id.clone(), text);
        }
    }

    const DOC: &'static str = r"# Formatted text editing

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

    let ui = float![
        Button::label_msg("↻", MsgDirection)
            .map_any()
            .pack(AlignHints::TOP_RIGHT),
        Splitter::new(collection![
            EditBox::new(Guard)
                .with_multi_line(true)
                .with_lines(4, 12)
                .with_text(DOC),
            ScrollLabel::new(Markdown::new(DOC).unwrap())
                .on_configure(|cx, label| {
                    cx.send(label.id(), SetLabelId(label.id()));
                })
                .on_message(|cx, label, text| {
                    label.set_text(cx, text);
                })
                .map_any()
        ])
        .on_update(|cx, list, data: &Data| {
            list.set_direction(cx, data.dir);
        }),
    ];

    let ui = ui
        .with_state(Data::default())
        .on_update(|_, data, app_data: &AppData| data.disabled = app_data.disabled)
        .on_message(|_, data, MsgDirection| {
            data.dir = match data.dir {
                Direction::Up => Direction::Right,
                _ => Direction::Up,
            };
        })
        .on_message(|_, data, SetLabelId(id)| data.label_id = id);

    Box::new(ui)
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
        list: (2019..=2025)
            .flat_map(|year| MONTHS.iter().map(move |m| format!("{m} {year}")))
            .collect(),
    };

    struct ListGuard;
    impl Driver<usize, String> for ListGuard {
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
            list.list_mut().set_selection_mode(cx, data.mode);
        });

    let sel_buttons = row![
        "Selection:",
        RadioButton::new_value("&n&one", SelectionMode::None),
        RadioButton::new_value("s&ingle", SelectionMode::Single),
        RadioButton::new_value("&multiple", SelectionMode::Multiple),
    ];
    let ui = column![
        sel_buttons.map(|data: &Data| &data.mode),
        ScrollBars::new(list_view),
    ];
    let ui = ui
        .with_state(data)
        .on_message(|_, data, mode| data.mode = mode)
        .on_message(|_, data, selection: SelectionMsg<usize>| match selection {
            SelectionMsg::Select(i) => println!("Selected: {}", &data.list[i]),
            _ => (),
        });
    let ui = adapt::AdaptEvents::new(ui)
        .on_update(|cx, _, data: &AppData| cx.set_disabled(data.disabled));
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
        fn draw(&self, pixmap: &mut Pixmap) {
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

    let ui = column![
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

    let ui = column![ScrollLabel::new(desc), Separator::new(), EventConfig::new()]
        .map_any()
        .on_update(|cx, _, data: &AppData| cx.set_disabled(data.disabled));
    Box::new(ui)
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let theme = kas::theme::MultiTheme::builder()
        .add("flat", kas::theme::FlatTheme::new())
        .add("simple", kas::theme::SimpleTheme::new())
        .add("shaded", kas_wgpu::ShadedTheme::new())
        .build();
    let mut runner = kas::runner::Default::with_theme(theme).build(())?;

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
                // Enumerate colour schemes.
                for (name, _) in runner.config().theme.color_schemes() {
                    let mut title = String::with_capacity(name.len() + 1);
                    match name {
                        "dark" => title.push_str("Dar&k"),
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

    let ui = column![
        menubar,
        Separator::new(),
        TabStack::from([
            ("&Widgets", widgets()), //TODO: use img_gallery as logo
            ("Te&xt editor", editor()),
            ("&List", filter_list()),
            ("Can&vas", canvas()),
            ("Confi&g", config()),
        ])
        .with_msg(|_, title| WindowCommand::SetTitle(format!("Gallery — {}", title))),
    ];

    let ui = ui
        .with_state(AppData::default())
        .on_message(|cx, state, msg| match msg {
            Menu::Theme(name) => {
                println!("Theme: {name:?}");
                let act = cx
                    .config()
                    .update_theme(|theme| theme.set_active_theme(name));
                cx.window_action(act);
            }
            Menu::Colour(name) => {
                println!("Colour scheme: {name:?}");
                let act = cx
                    .config()
                    .update_theme(|theme| theme.set_active_scheme(name));
                cx.window_action(act);
            }
            Menu::Disabled(disabled) => {
                state.disabled = disabled;
            }
            Menu::Quit => {
                cx.exit();
            }
        });

    runner.add(Window::new(ui, "Gallery — Widgets"));
    runner.run()
}
