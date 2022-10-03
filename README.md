KAS GUI
=======

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![Crates.io](https://img.shields.io/crates/v/kas.svg)](https://crates.io/crates/kas)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.65+-lightgray.svg)

KAS is a pure-Rust GUI toolkit with stateful widgets:

-   Widgets retain state (retained mode), also supporting shared data (models)
-   Powerful, simple event model: events go down the widget tree, messages come back up
-   Layout DSL and complex layout solver
-   Widget library uses only user-facing parts of the core API
-   Abstraction for widget, theme and backend graphics code
-   Widget *and* theme driven animations
-   Embedded accelerated graphics (see [Mandlebrot example](examples/mandlebrot))
-   Accessible (full keyboard control, no screen reader yet)
-   Stepless DPI scaling
-   Very fast and low CPU usage

![Animated](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/video/animations.apng)
![Scalable](https://github.com/kas-gui/data-dump/blob/master/kas_0_10/image/scalable.png)

### Limitations

-   Slow compile times. See [Faster builds](https://github.com/kas-gui/kas/wiki/Getting-started#faster-builds).
-   Somewhat large binaries; e.g. for the `gallery` example: 333M (debug),
    38M (debug + strip), 20M (release), 12M (release + strip).
    Note that these binaries are statically linked, as is the norm for Rust.
    Some improvements may be possible, e.g. disabling features such as `shaping`
    and `image` or replacing the rendering backend.

### Documentation

-   Wiki: [Getting started](https://github.com/kas-gui/kas/wiki/Getting-started),
    [Configuration](https://github.com/kas-gui/kas/wiki/Configuration),
    [Troubleshooting](https://github.com/kas-gui/kas/wiki/Troubleshooting)
-   API docs: <https://docs.rs/kas>, <https://docs.rs/kas-core>,
    <https://docs.rs/kas-widgets>, <https://docs.rs/kas-theme>, <https://docs.rs/kas-wgpu>
-   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
-   [Changlelog](CHANGELOG.md)
-   [Roadmap](ROADMAP.md)
-   [Discuss](https://github.com/kas-gui/kas/discussions)
-   [KAS Blog](https://kas-gui.github.io/blog/)

### Examples

See the [`examples`](examples) directory and
[kas-gui/7guis](https://github.com/kas-gui/7guis/).

Precompiled example apps can be downloaded as follows:

-   go to <https://github.com/kas-gui/kas/actions/workflows/build.yml>
-   select the latest (complete) run
-   download one of the `examples-*` artifacts


Design
------

### Data or widget first?

KAS uses a widget-first design: widgets are persistent and retain state; data
must be pushed into widgets. *Many* modern UIs are data-first: widgets are
built per-frame over a single persistent data object. There are significant
trade-offs of a widget-first design:

-   (for) widgets have embedded state; data-first designs may require explicitly
    connecting widgets to state, for those that need it
-   (for) enables more complex layout solvers
-   (against) dynamic layout is harder
-   (for) updates are fast since only affected widgets need be touched
-   (against) (re)building the UI is slower
-   (for) pre-built "view widgets" providing scrolling and selection support
    over large external databases (only retrieving visible entries)
-   (against) "view widgets" are an emulation of data-first design over a
    widget-first model, and less flexible


Crates and features
-------------------

`kas` is a meta-package over the core (`kas-core`), widget library
(`kas-widgets`), etc. [See here](https://kas-gui.github.io/tutorials/#kas).

At this point in time, `kas-wgpu` is the only windowing/rendering implementation
and `kas-theme` the only theme (high-level drawing) implementation, thus `kas`
uses these crates by default, though they are optional.

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
