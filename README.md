KAS GUI
==========

[![home](https://img.shields.io/badge/GitHub-home-blue)](https://github.com/dhardy/kas)
[![old-home](https://img.shields.io/badge/GitLab-old--home-blueviolet)](https://gitlab.com/dhardy/kas)

KAS is the *toolKit Abstraction System*. It comprises:

-   `kas`: the *core* of the GUI library, providing core interfaces and types,
    standard widgets, widget layout and event handling
-   `kas_macros`: a helper crate providing the procedural macros used by `kas`
-   `kas_wgpu`: an interface to [`winit`] and [`wgpu`], providing windowing and
    hardware-accelerated rendering

A user depends on `kas` to write their complete UI specification, and then
pastes a few lines of code to initialise `kas_wgpu::Toolkit`, add the window
and run the UI.

[`winit`]: https://github.com/rust-windowing/winit/
[`wgpu`]: https://github.com/gfx-rs/wgpu-rs


Examples
---------

Several examples are available for the `kas_wgpu` sub-crate. Try e.g.

```
cargo run --example calculator
```

![Calculator](screenshots/calculator.png) ![Gallery](screenshots/gallery.png) ![Theme](screenshots/theme.png)
![Stopwatch](screenshots/stopwatch.png) ![Layout](screenshots/layout.png) ![Dynamic](screenshots/dynamic.png)

Cross-platform
------------

KAS uses cross-platform libraries, allowing it to target all major platforms.
Current development & test targets:

-   Linux / X11
-   Linux / Wayland

It *should* work fine on other platforms, although this is largely untested.
Feedback & fixes welcome.


Features
----------

-   Type-safe event handler allowing user data to be embedded in the widget tree
-   Built-in widget selection (currently limited)
-   Row / column layouts
-   Flexible grid layouts
-   Draw-command abstraction (currently limited to required features)
-   Custom themes (with full control of sizing and rendering)
-   Touch-screen support
-   Keyboard navigation & accelerator keys
-   Fully scalable (hidpi)
-   Mult-window support
-   Fast and low CPU usage

### Planned features

-   Dynamic layouts
-   Custom widgets
-   Better support for custom event handling
-   Allow direct access to wgpu when using `kas_wgpu` (custom shaders)
-   Animations


Data model and specification
--------------

KAS is in part motivated by some of the common limitations of UIs:

-   specification is often redundant, requiring widgets to be created, added
    to a parent, sometimes forcing elements to be named when *any* custom
    properties are required
-   declarative specifications may need to be recreated frequently
-   user state (data) and UI models are often separated, making event handling
    and data transfer more difficult than ought be necessary

KAS takes some inspiration from Qt (but using macros in place of language
extensions), in that custom widget structs may combine user state and UI
components. Most of KAS is inspired by finding a maximally-type-safe, flexible
"Rustic" solution to the problem at hand.

### Example

The following is extracted from the `counter` example:

```rust
let buttons = make_widget! {
    container(horizontal) => Message;
    struct {
        #[widget] _ = TextButton::new("−", Message::Decr),
        #[widget] _ = TextButton::new("+", Message::Incr),
    }
};
let window = Window::new(make_widget! {
    container(vertical) => ();
    struct {
        #[widget] display: Label = Label::from("0"),
        #[widget(handler = handle_button)] buttons -> Message = buttons,
        counter: usize = 0,
    }
    impl {
        fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: Message)
            -> VoidResponse
        {
            match msg {
                Message::Decr => {
                    self.counter = self.counter.saturating_sub(1);
                    self.display.set_text(tk, self.counter.to_string());
                }
                Message::Incr => {
                    self.counter = self.counter.saturating_add(1);
                    self.display.set_text(tk, self.counter.to_string());
                }
            };
            VoidResponse::None
        }
    }
});
```


Drawing and themes
--------

One of the key problems to solve in a UI is the question of *how are widgets
drawn?* Already, multiple approaches have been tried and abandoned:

-   `kas_gtk` used GTK to do the rendering; in practice this meant using GTK
    for event handling and widget layout too, and made building the desired API
    around GTK very difficult; an additional issue with this approach is that
    GTK libs can be difficult to install on some platforms
-   `kas_rgx` used RGX as a rendering API allowing custom widget rendering
    with a "mid-level graphics API"; ultimately this proved less flexible than
    desired while also lacking high-level drawing routines (e.g. pretty frames)

Thus, KAS has now moved to direct use of `wgpu` and `wgpu_glyph` for rendering,
providing its own high-level abstractions (the `Draw*` traits provided by
`kas` and `kas_wgpu`). This still needs fleshing out (more drawing primitives,
a better text API, and support for custom pipes & shaders), but looks to be a
viable path forward.

### Themes

A "theme" provides widget sizing and drawing implementations over the above
`Draw*` traits as well as a choice of fonts (and eventually icons).

Currently a single `SampleTheme` is provided, along with a custom theme example.


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
