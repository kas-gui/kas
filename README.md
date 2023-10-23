KAS GUI
=======

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![Crates.io](https://img.shields.io/crates/v/kas.svg)](https://crates.io/crates/kas)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.66+-lightgray.svg)

KAS is a stateful, pure-Rust GUI toolkit supporting:

- [x] Mostly declarative UI descriptions despite stateful widgets
- [x] Custom widgets using state for caches and input state (e.g. selection range)
- [x] Virtual scrolling (list or matrix), including support for external data sources
- [x] Theme abstraction including theme-driven animations and sizing
- [ ] Multiple renderer backends
- [ ] Integrated i18n support
- [ ] Accessibility tool integration
- [ ] Platform integration: persistent configuration, theme discovery, external menus, IME
- [x] Most of the basics you'd expect: complex text, fractional scaling, automatic margins
- [x] Extremely fast, monolithic binaries

### More

-   Wiki: [Getting started](https://github.com/kas-gui/kas/wiki/Getting-started),
    [Configuration](https://github.com/kas-gui/kas/wiki/Configuration),
    [Troubleshooting](https://github.com/kas-gui/kas/wiki/Troubleshooting)
-   [API docs](https://docs.rs/kas)
-   Docs: [Tutorials](https://kas-gui.github.io/tutorials/),
    [Blog](https://kas-gui.github.io/blog/),
    [Design](https://github.com/kas-gui/design)
-   Examples: [`examples` dir](examples), [kas-gui/7guis](https://github.com/kas-gui/7guis/).


Crates and features
-------------------

[kas] is a meta-package serving as the library's public API, yet
containing no real code. Other crates in this repo:

-   [kas-core](https://docs.rs/kas-core): the core library
-   [kas-widgets](https://docs.rs/kas-widgets): the main widget library
-   [kas-view](https://docs.rs/kas-view): view widgets supporting virtual scrolling
-   [kas-resvg](https://docs.rs/kas-resvg): extra widgets over [resvg](https://crates.io/crates/resvg)
-   [kas-dylib](https://crates.io/crates/kas-dylib): helper crate to support dynamic linking
-   kas-macros: proc-macro crate

Significant external dependencies:

-   [kas-text](https://crates.io/crates/kas-text): complex text support
-   [impl-tools](https://crates.io/crates/impl-tools): `autoimpl` and `impl_scope` (extensible) macros
-   [winit](https://github.com/rust-windowing/winit): platform window integration
-   [wgpu](https://github.com/gfx-rs/wgpu): modern accelerated graphics API

### Feature flags

The `kas` crate enables most important features by default, excepting those
requiring nightly `rustc`. Other crates enable fewer features by default.
See [Cargo.toml](https://github.com/kas-gui/kas/blob/master/Cargo.toml#L22).

[kas]: https://docs.rs/kas


Copyright and Licence
---------------------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
