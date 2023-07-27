// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Cursor gallery

use kas::event::CursorIcon;
use kas::prelude::*;
use kas::widget::{Column, Label, StrLabel};

impl_scope! {
    #[widget{
        Data = ();
        layout = self.label;
        cursor_icon = self.cursor;
    }]
    struct CursorWidget {
        core: widget_core!(),
        #[widget]
        label: StrLabel,
        cursor: CursorIcon,
    }
    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            // Steal mouse focus: hover points to self, not self.label
            self.rect().contains(coord).then(|| self.id())
        }
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

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    // These are winit::window::CursorIcon enum variants
    let column = Column::new_vec(vec![
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

    let window = Window::new(column, "Cursor gallery");
    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
