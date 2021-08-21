KAS GUI
=======

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.53+-lightgray.svg)

KAS is a widget-first GUI toolkit:

-   widgets retain state
-   flexibile drawing, layout, event handling, animation
-   clean, type-safe widget interaction
-   full keyboard control of UI
-   fast, efficient, responsive

### Features

-   Good automatic layout (margins, text wrapping width-for-height logic)
-   Partially declarative UI descriptions possible through macros
-   Stepless DPI scaling
-   Advanced text features: shaping, bidirectional support, font fallbacks
-   Embedded GPU shaders (see [Mandlebrot example](examples/mandlebrot))
-   Supports theming and end-user configuration
-   View widgets for seemless scrolling and sharing of large data

### Limitations

-   Slow compile times. The `dynamic` feature may help (see below).
-   Large binaries. Distributing feature-heavy applications without shared
    libraries will always have this problem, but if you seek a minimal GUI
    toolkit then you should probably look elsewhere.

### Documentation

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


Getting started
---------------

### Dependencies

KAS requires a recent [Rust] compiler. Currently, version 1.53 or greater is
required. Using the **nightly** channel does have a few advantages:

-   Proceedural macros emit better diagnostics. In some cases, diagnostics are
    missed without nightly rustc, hence **nightly is recommended for development**.
-   The `nightly` (`min_spec`) feature allows some visual improvements (see below).
-   The `doc_cfg` feature may be used for API docs.

#### Linux libraries

Install dependencies (glslc is optional; see [kas-wgpu's README](crates/kas-wgpu/README.md)):
```sh
# For Ubuntu:
sudo apt-get install build-essential git libxcb-shape0-dev libxcb-xfixes0-dev libharfbuzz-dev

# For Fedora:
sudo dnf install libxcb-devel harfbuzz-devel glslc
```

### Running examples

Clone the repository and run the examples as follows:
```sh
git clone https://github.com/kas-gui/kas.git
cd kas
cargo run --example gallery
cargo run --example layout
cargo run --example filter-list
cd examples/mandlebrot; cargo run
```

#### Buliding documentation locally

```
RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc --features=nightly --all --no-deps --open
```

### Run-time configuration

#### Graphics

KAS uses [WebGPU] for rendering, which targets Vulkan and OpenGL on Linux and
Android, Vulkan, DX12 and DX11 on Windows, and finally Metal on MacOS and iOS.
This should satisfy *most* devices, albeit support may be incomplete (refer to
[WebGPU] documentation).

To force use of a specific backend, set `KAS_BACKENDS`, for example:
```
export KAS_BACKENDS=GL
```
To prefer use of a discrete GPU over integrated graphics, set:
```
export KAS_POWER_PREFERENCE=HighPerformance
```

#### Config files

Configuration support is built but not enabled by default, since formats are not
yet stable. It may also be used programmatically.

To use, specify paths (`KAS_CONFIG`, `KAS_THEME_CONFIG`) and mode
(`KAS_CONFIG_MODE`: `Read` (default), `ReadWrite` or `WriteDefault`).

To get started:
```sh
export KAS_CONFIG=kas.yaml
export KAS_THEME_CONFIG=theme.yaml
export KAS_CONFIG_MODE=readwrite

# Optionally, force creation of default files:
KAS_CONFIG_MODE=WriteDefault cargo run --example gallery

# Now, just run:
cargo run --example gallery
```

For further documentation, see [`kas_wgpu::Options`].


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
-   `kas-wgpu`: provides windowing via [winit] and rendering via [WebGPU]
-   `kas-dylib`: support for dynamic linking
-   <https://docs.rs/easy-cast>: spin-off crate for checked casts

At this point in time, `kas-wgpu` is the only windowing/rendering implementation
and `kas-theme` the only theme (high-level drawing) implementation, thus `kas`
uses these crates by default, though they are optional.

Futher, capabilities such as text shaping and Markdown processing are enabled by
default. Image-loading support is not currently optional, and includes all
formats supported by the `image` crate. Some improvements to binary size and
compile time should be possible here.

### Feature flags

The `kas` crate enables most important features by default, excepting those
requiring nightly `rustc`. Other crates enable fewer features by defualt.

The following non-default features of `kas` are highlighted:

-   `dynamic`: enable dynamic linking for `kas`. This feature improves
    (re)compile times of examples and dependent code, at the cost of requiring
    extra libraries at run-time.
-   `internal_doc`: turns on some extra documentation intended for internal
    usage but not for end users. (This only affects generated documentation.)
-   `nightly`: enables "more stable" unstable features
-   `min_spec` (enabled by `nightly`): use `min_specialization` for some visual
    improvements: scrolled regions are drawn under scrollbars,
    underlines on checkbox accelerator keys show with the <kbd>Alt</kbd> key.
-   `spec`: use `specialization` to enable `TryFormat`

For full documentation of feature flags, see the [`Cargo.toml`](Cargo.toml).

[KAS-text]: https://github.com/kas-gui/kas-text/
[winit]: https://github.com/rust-windowing/winit/
[WebGPU]: https://github.com/gfx-rs/wgpu
[`kas_wgpu::Options`]: https://docs.rs/kas-wgpu/latest/kas_wgpu/options/struct.Options.html


Copyright and Licence
---------------------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
