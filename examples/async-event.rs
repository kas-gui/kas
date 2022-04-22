// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Asynchronous event demo
//!
//! This is a copy-cat of Druid's async event example.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use kas::draw::color::Rgba;
use kas::prelude::*;
use kas::widgets::Window;

fn main() -> kas::shell::Result<()> {
    env_logger::init();
    let theme = kas::theme::FlatTheme::new();
    let toolkit = kas::shell::Toolkit::new(theme)?;

    // We construct a proxy from the toolkit to enable cross-thread communication.
    let proxy = toolkit.create_proxy();

    // An "update handle" is used to notify our widget.
    let handle = UpdateHandle::new();

    // The sender and receiver need to communicate. We use Arc<Mutex<T>>, but
    // could instead use global statics or std::sync::mpsc or even encode our
    // data within the update payload (a u64, so some compression required).
    let colour = Arc::new(Mutex::new(Rgba::grey(1.0)));
    let colour2 = colour.clone();

    thread::spawn(move || generate_colors(proxy, handle, colour2));

    let widget = ColourSquare {
        core: Default::default(),
        colour,
        handle,
    };

    let window = Window::new("Async event demo", widget);
    toolkit.with(window)?.run()
}

impl_scope! {
    #[derive(Debug)]
    #[widget]
    struct ColourSquare {
        #[widget_core]
        core: CoreData,
        colour: Arc<Mutex<Rgba>>,
        handle: UpdateHandle,
    }
    impl WidgetConfig for ColourSquare {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            // register to receive updates on this handle
            mgr.update_on_handle(self.handle, self.id());
        }
    }
    impl Layout for ColourSquare {
        fn size_rules(&mut self, mgr: SizeMgr, _: AxisInfo) -> SizeRules {
            SizeRules::fixed_scaled(100.0, 10.0, mgr.scale_factor())
        }
        fn draw(&mut self, mut draw: DrawMgr) {
            let draw = draw.draw_device();
            let col = *self.colour.lock().unwrap();
            draw.rect((self.rect()).cast(), col);
        }
    }
    impl Handler for ColourSquare {
        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::HandleUpdate { .. } => {
                    // Note: event has `handle` and `payload` params.
                    // We only need to request a redraw.
                    mgr.redraw(self.id());
                    Response::Used
                }
                _ => Response::Unused,
            }
        }
    }
}

fn generate_colors(
    proxy: kas::shell::ToolkitProxy,
    handle: UpdateHandle,
    colour: Arc<Mutex<Rgba>>,
) {
    // This function is called in a separate thread, and runs until the program ends.
    let start_time = Instant::now();

    loop {
        let hue = (Instant::now() - start_time).as_secs_f32() / 5.0;

        // convert from HSV, using S=V=1 (see Wikipedia):
        let f = |n| {
            let k: f32 = (n + hue * 6.0) % 6.0;
            1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
        };
        let c = Rgba::rgb(f(5.0), f(3.0), f(1.0));

        // Communicate the colour ...
        *colour.lock().unwrap() = c;
        // .. and notify of an update.
        // (Note: the 0 here is the u64 payload, which could pass useful data!)
        if proxy.trigger_update(handle, 0).is_err() {
            // Sending failed; we should quit
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }
}
