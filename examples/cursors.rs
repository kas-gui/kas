// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Cursor gallery

use kas::event::CursorIcon;
use kas::prelude::*;
use kas::widgets::{Column, Label};

#[impl_self]
mod CursorWidget {
    #[widget]
    #[layout(self.label)]
    struct CursorWidget {
        core: widget_core!(),
        #[widget]
        label: Label<&'static str>,
        cursor: CursorIcon,
    }

    impl Tile for Self {
        fn probe(&self, _: Coord) -> Id {
            // This widget takes mouse focus, not self.label
            self.id()
        }
    }

    impl Events for Self {
        type Data = ();

        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            Some(self.cursor)
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

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    // These are winit::window::CursorIcon enum variants
    let column = Column::new([
        cursor!(Default),
        cursor!(ContextMenu),
        cursor!(Help),
        cursor!(Pointer),
        cursor!(Progress),
        cursor!(Wait),
        cursor!(Cell),
        cursor!(Crosshair),
        cursor!(Text),
        cursor!(VerticalText),
        cursor!(Alias),
        cursor!(Copy),
        cursor!(Move),
        cursor!(NoDrop),
        cursor!(NotAllowed),
        cursor!(Grab),
        cursor!(Grabbing),
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
        cursor!(AllScroll),
        cursor!(ZoomIn),
        cursor!(ZoomOut),
    ]);

    let window = Window::new(column, "Cursor gallery");
    kas::runner::Runner::new(())?.with(window).run()
}
