KAS GUI
=======

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![Crates.io](https://img.shields.io/crates/v/kas.svg)](https://crates.io/crates/kas)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.65+-lightgray.svg)

KAS is a pure-Rust GUI toolkit with stateful widgets:

- [x] Pure, portable Rust
- [x] Very fast and CPU efficient
- [x] Flexible event handling without data races
- [x] Theme abstraction layer
- [x] [Winit] + [WGPU] shell supporting embedded accelerated content
- [ ] More portable shells: OpenGL, CPU-rendered, integration
- [x] [Complex text](https://github.com/kas-gui/kas-text/)
- [ ] OS integration: menus, fonts, IME
- [ ] Accessibility: screen reader, translation

![Animated](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/video/animations.apng)
![Scalable](https://github.com/kas-gui/data-dump/blob/master/kas_0_10/image/scalable.png)

[Winit]: https://github.com/rust-windowing/winit
[WGPU]: https://github.com/gfx-rs/wgpu

### Documentation

-   Wiki: [Getting started](https://github.com/kas-gui/kas/wiki/Getting-started),
    [Configuration](https://github.com/kas-gui/kas/wiki/Configuration),
    [Troubleshooting](https://github.com/kas-gui/kas/wiki/Troubleshooting)
-   API docs: [kas](https://docs.rs/kas), [kas-core](https://docs.rs/kas-core),
    [kas-widgets](https://docs.rs/kas-widgets),
    [kas-wgpu](https://docs.rs/kas-wgpu)
-   Prose: [Tutorials](https://kas-gui.github.io/tutorials/),
    [Blog](https://kas-gui.github.io/blog/)

### Examples

See the [`examples`](examples) directory and
[kas-gui/7guis](https://github.com/kas-gui/7guis/).


Design
------

### Data or widget first?

KAS attempts to blend several GUI models:

-   Like many older GUIs, there is a persistent tree of widgets with state
-   Like Elm, event handling uses messages; unlike Elm, messages may be handled
    anywhere in the widget tree (proceeding from leaf to root until handled)
-   Widgets have a stable identity using a path over optionally explicit
    components
-   Like Model-View-Controller designs, data separation is possible; unlike Elm
    this is not baked into the core of the design

The results:

-   Natural support for multiple windows (there is no central data model)
-   Widget trees (without MVC) are static and pre-allocated, though efficient
    enough that maintaining (*many*) thousands
    of not-currently-visible widgets isn't a problem
-   Support for accessibility (only navigation aspects so far)
-   MVC supports virtual scrolling (including persistent IDs for unrealised
    widgets)
-   MVC supports shared (`Rc` or `Arc`) data
-   MVC and stateful widget designs feel like two different architectures
    forced into the same UI toolkit


Crates and features
-------------------

`kas` is a meta-package over the core (`kas-core`), widget library
(`kas-widgets`), etc. [See here](https://kas-gui.github.io/tutorials/#kas).

At this point in time, `kas-wgpu` is the only windowing/rendering implementation
thus `kas` uses this crate by default, though it is optional.

### Feature flags

The `kas` crate enables most important features by default, excepting those
requiring nightly `rustc`. Other crates enable fewer features by default.
See [Cargo.toml](https://github.com/kas-gui/kas/blob/master/Cargo.toml#L22).

[KAS-text]: https://github.com/kas-gui/kas-text/
[winit]: https://github.com/rust-windowing/winit/
[WGPU]: https://github.com/gfx-rs/wgpu
[`kas_wgpu::Options`]: https://docs.rs/kas-wgpu/latest/kas_wgpu/options/struct.Options.html


Copyright and Licence
---------------------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
