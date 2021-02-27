// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Cursor gallery

use kas::event::{CursorIcon, VoidMsg};
use kas::prelude::*;
use kas::widget::{Column, Label, StrLabel, Window};

#[derive(Clone, Debug, Widget)]
#[widget(config = noauto)]
struct CursorWidget {
    #[widget_core]
    core: CoreData,
    #[widget]
    label: StrLabel,
    cursor: CursorIcon,
}
impl WidgetConfig for CursorWidget {
    fn cursor_icon(&self) -> CursorIcon {
        self.cursor
    }
}
// We implement Layout manually, mainly because we want Layout::find_id to point
// at *self*, not self.label.
impl Layout for CursorWidget {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.label.size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core_data_mut().rect = rect;
        self.label.set_rect(mgr, rect, align);
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        self.label.draw(draw_handle, mgr, disabled);
    }
}

// Using a macro lets us stringify! the type name
macro_rules! cursor {
    ($name: tt) => {
        CursorWidget {
            core: Default::default(),
            label: Label::new(stringify!($name)),
            cursor: CursorIcon::$name,
        }
    };
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    // These are winit::window::CursorIcon enum variants
    let column = Column::new(vec![
        cursor!(Default),
        cursor!(Crosshair),
        cursor!(Hand),
        cursor!(Arrow),
        cursor!(Move),
        cursor!(Text),
        cursor!(Wait),
        cursor!(Help),
        cursor!(Progress),
        cursor!(NotAllowed),
        cursor!(ContextMenu),
        cursor!(Cell),
        cursor!(VerticalText),
        cursor!(Alias),
        cursor!(Copy),
        cursor!(NoDrop),
        cursor!(Grab),
        cursor!(Grabbing),
        cursor!(AllScroll),
        cursor!(ZoomIn),
        cursor!(ZoomOut),
        cursor!(EResize),
        cursor!(NResize),
        cursor!(NeResize),
        cursor!(NwResize),
        cursor!(SResize),
        cursor!(SeResize),
        cursor!(SwResize),
        cursor!(WResize),
        cursor!(EwResize),
        cursor!(NsResize),
        cursor!(NeswResize),
        cursor!(NwseResize),
        cursor!(ColResize),
        cursor!(RowResize),
    ]);

    // Since Column has message type (usize, Child::Msg) we must convert
    // (maybe after Rust's specialisation is ready this can be automatic).
    let gallery = make_widget! {
        #[layout(single)]
        #[handler(msg = VoidMsg)]
        struct {
            #[widget(handler = handle)] _ = column,
        }
        impl {
            fn handle(&mut self, _: &mut Manager, msg: (usize, VoidMsg)) -> Response<VoidMsg> {
                // This variant is of course impossible...
                Response::Msg(msg.1)
            }
        }
    };

    let window = Window::new("Cursor gallery", gallery);
    let theme = kas_theme::FlatTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
