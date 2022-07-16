KAS GUI
=======

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.62+-lightgray.svg)

KAS is a widget-first GUI toolkit:

-   widgets retain state
-   flexibile drawing, layout, event handling, animation
-   clean, type-safe widget interaction
-   full keyboard control of UI
-   fast, efficient, responsive

### Features

-   Good automatic layout (margins, text wrapping width-for-height logic)
-   Partially declarative UI descriptions possible through macros
-   Advanced text features: shaping, bidirectional support, font fallbacks
-   Embedded GPU shaders (see [Mandlebrot example](examples/mandlebrot))
-   Supports theming and end-user configuration
-   View widgets for seemless scrolling and sharing of large data
-   Stepless DPI scaling:

![Scalable](https://github.com/kas-gui/data-dump/blob/master/screenshots/scalable.png)

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
-   (against) dynamic layout is harder
-   (for) updates are fast since only affected widgets need be touched; using
    10,000+ widgets in a UI is not a problem
-   (against) rebuilding the UI is much slower
-   (for) pre-built "view widgets" with built-in support for scrolling
    over large external databases (only retrieving visible entries)
-   (against) "view widgets" are an emulation of data-first design over a
    widget-first model, and less flexible


Crates and features
-------------------

### Crates

-   `kas` is a meta-package; most of the time this is the only one you need to
    use directly
-   `kas-macros`: a helper crate for proc macros
-   `kas-core` provides most interfaces and logic concerning widgets (event
    handling, layout, draw API, geometry types)
-   [KAS-text]: provides text layout and font management
-   `kas-widgets`: the standard widget library
-   `kas-theme`: theming support for KAS (API, two themes, config support)
-   `kas-wgpu`: provides windowing via [winit] and rendering via [WGPU]
-   `kas-dylib`: support for dynamic linking
-   <https://docs.rs/easy-cast>: spin-off crate for checked casts
-   <https://docs.rs/impl-tools>: spin-off macro support crate

At this point in time, `kas-wgpu` is the only windowing/rendering implementation
and `kas-theme` the only theme (high-level drawing) implementation, thus `kas`
uses these crates by default, though they are optional.

### Feature flags

The `kas` crate enables most important features by default, excepting those
requiring nightly `rustc`. Other crates enable fewer features by defualt.

The following non-default features of `kas` are highlighted:

-   `dynamic`: enable dynamic linking for `kas` (see
    [Faster builds](https://github.com/kas-gui/kas/wiki/Getting-started#faster-builds))
-   `internal_doc`: turns on some extra documentation intended for internal
    usage but not for end users. (This only affects generated documentation.)
-   `nightly`: enables the less problematic unstable features
-   `min_spec` (enabled by `nightly`): use `min_specialization` for some visual
    improvements: scrolled regions are drawn under scroll bars,
    underlines on check-box accelerator keys show with the <kbd>Alt</kbd> key.
-   `spec`: use `specialization` to enable `TryFormat`

For full documentation of feature flags, see the [`Cargo.toml`](Cargo.toml).

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
