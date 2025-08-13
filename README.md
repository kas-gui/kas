KAS GUI
=======

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![Crates.io](https://img.shields.io/crates/v/kas.svg)](https://crates.io/crates/kas)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)

The Kas GUI system strives to be both fast and follow Alan Kay's slogan:

> Simple things should be simple, complex things should be possible.

An excerpt from examples/hello.rs:
```rust
let hello_ui = column![
    "Hello, world!",
    Button::label("&Close").with(|cx, _| cx.exit())
];
```

An excerpt from examples/counter.rs:
```rust
#[derive(Clone, Debug)]
struct Increment(i32);

fn counter() -> impl Widget<Data = ()> {
    let tree = column![
        format_value!("{}").align(AlignHints::CENTER),
        row![
            Button::label_msg("−", Increment(-1)),
            Button::label_msg("+", Increment(1)),
        ]
        .map_any(),
    ];

    tree.with_state(0)
        .on_message(|_, count, Increment(add)| *count += add)
}
```

Concerning making complex things possible, the Kas widget library is built using the same custom widget functionality as is available to Kas users with few exceptions. Check the [`Widget`](https://docs.rs/kas/latest/kas/trait.Widget.html) API docs or browse the [widget library](https://docs.rs/kas/latest/kas/widgets/index.html) and click the "Source" link.

### Documentation

-   Docs: [Tutorials](https://kas-gui.github.io/tutorials/),
    [Wiki: Getting started](https://github.com/kas-gui/kas/wiki/Getting-started)
-   Prose: [Blog](https://kas-gui.github.io/blog/),
    [Design](https://github.com/kas-gui/design)
-   [API docs](https://docs.rs/kas)
-   Examples: [`examples` dir](examples), [kas-gui/7guis](https://github.com/kas-gui/7guis/).

Capabilities
------------

- [x] Fully keyboard-accessible
- [x] Screen reader support (partial: kas-gui/kas#509)
- [x] IME support (partial: kas-gui/kas#508)
- [ ] Integrated i18n support (not yet started;  kas-gui/kas#7)
- [x] Complex text support: use system fonts with glyph fallbacks, BiDi, common text effects (kas-gui/kas#13)
- [x] Automatic margins and layout with pixel-perfect scaling
- [x] Support for custom themes including theme-driven animations and sizing
- [x] Virtual scrolling (list or grid), including support for async data access
- [x] Extremely fast, monolithic Rust binaries

Every approach has its limitations. Ours are:

-   Custom widgets are stateful, supporting custom caches and minimal state updates but no protection from bad-state bugs.
-   Custom widgets have a lot of flexibility over management of child widgets; this comes with some expectations. Violating these expectations will result in a panic in debug builds.
-   Custom widget definitions require the use of macros; the latest versions of these have a low learning curve (except maybe `impl_anon!`) but unfortunately `impl_scope!` does not work with `rustfmt` and `#[impl_self]` causes `rust-analyzer` to inject `use` statements in the wrong location (kas-gui/impl-tools#57).


Crates and features
-------------------

[kas] is a meta-package serving as the library's public API, but containing no real code. Other crates in this repo:

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
See [Cargo.toml](https://github.com/kas-gui/kas/blob/master/Cargo.toml#L29).

[kas]: https://docs.rs/kas


Copyright and Licence
---------------------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
