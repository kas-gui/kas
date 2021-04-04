KAS GUI
==========

[![Test Status](https://github.com/kas-gui/kas/workflows/Tests/badge.svg?event=push)](https://github.com/kas-gui/kas/actions)
[![kas-text](https://img.shields.io/badge/GitHub-kas--text-blueviolet)](https://github.com/kas-gui/kas-text/)
[![Docs](https://docs.rs/kas/badge.svg)](https://docs.rs/kas)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.52+-lightgray.svg)

KAS, (historically the *toolKit Abstraction System*), is a general-purpose GUI toolkit.
KAS's design provides:

-   retained mode (library stores state), inspired by Qt (classic)
-   concise, partially declarative specification of widgets
-   type-safe, widget-local event handlers
-   simple ownership with no retained pointers into widget state
-   widgets embed state and handlers (easy reuse of complex components)
-   scalability to millions of widgets

## Documentation

-   API docs: <https://docs.rs/kas>, <https://docs.rs/kas-theme>, <https://docs.rs/kas-wgpu>
-   [KAS Tutorials](https://kas-gui.github.io/tutorials/)
-   [Examples](https://github.com/kas-gui/kas/tree/master/kas-wgpu/examples)
-   [Discuss](https://github.com/kas-gui/kas/discussions)
-   [KAS Blog](https://kas-gui.github.io/blog/)

## Examples

Examples can be found in [kas-wgpu/examples](kas-wgpu/examples).
Further examples can be found in [kas-gui/7guis](https://github.com/kas-gui/7guis/).

![Gallery](https://github.com/kas-gui/data-dump/blob/master/video/gallery.png)

## Status and Features

The below should give a rough idea of what's done and what's not. See also the
[ROADMAP].

We aim to make new minor releases (0.x.0) every couple of months and patch
releases (0.x.y) only for minor fixes where required.
Before 1.0 (which will *not* be the release after 0.9), some breaking changes
should be expected in each minor release.

### Layout

**Widget layout:** works well but not perfectly; automatic sizing according to
content or specified size for custom widgets; automatic position and stretching
within row/column or grid layouts. Some tweaking still needed.  
**Text layout:** custom engine handles shaping, bidi and line-wrapping (some
bugs). Several features missing: font fallbacks, emoticons, large-text support.
See separate [KAS-text] repository.  
**DPI scaling:** done.  
**Performance/scalability:** pretty fast. Exception: text layout of larger
documents (not done). Scalability is okay to at least thousands of widgets and
even usable to a million since many operations are `O(log n)` or better.  

### Graphics

**Support:** uses [WebGPU] for DirectX/Vulkan/Metal and (maybe) OpenGL
acceleration. Currently no CPU fallback.  
**Themes:** theme engine supports sizing and drawing many common widget parts
(not yet comprehensive), with two example themes (not especially good ones).  
**API:** via theme or via a few primitives. Basic.  
**Custom accelerated widgets:** yes (see [Mandlebrot example](kas-wgpu/examples/README.md#Mandlebrot)).  
**Textures/images:** missing (probably for v0.8).

### Event handling

**Mouse interactions:** most left-click actions implemented for existing
widgets, including double-click and delayed responses. Context-menus missing.  
**Keyboard interactions:** tab-navigation, arrow-key navigation and accelerator
keys all done. Menus navigable with arrows and Alt+Key combos. Widgets may
respond to Home, PageUp, etc.  
**Touch interactions:** most single-touch gestures done. Minimal support for
multi-touch gestures (see Mandlebrot demo app).  
**Text-editing:** most expected keyboard/mouse behaviours done. Basic touch
interactions supported but less complete and no virtual keyboard.  
**Shortcuts:** widget-local and some navigation shortcuts supported; global
shortcuts are missing. Several platform-specific bindings.  
**Configuration:** shortcuts and some event-handling behaviour configurable.
Serialisation to/from JSON and YAML. [See below](#Configuration).

### Platform integration

**Font discovery:** basic.  
**Platform-specific default config:** yes (but probably needs tuning).  
**Windowing:** uses [winit] which supports basic windows but lacks a few things,
including pop-up/modal windows and good text input support.
In theory a back-end could directly target Windows/X11/... instead but this has
not been done.  
**Platform-native menus:** no; waiting on [winit] support (see above).  
**Platform-native dialogs:** no; waiting on [winit] support (see above).  
**Embedding (within a game):** not supported; only really requires graphics
([WebGPU] or implementing basics over another backend) plus input binding
(currently only [winit] events are supported).  

### Data handling

**Embedded state in widgets:** yes.  
**Shared state:** yes (in progress). Used by the `sync-counter`, `filter-list`
and `dynamic-view` examples; allows custom widgets within a data-sharing "view"
over a single datum or list of data. Missing a few common widgets (tree-view,
table, spreadsheet).  
**Multi-thread communication:** yes (at least basic support). See the
`async-event` example.  

### Widget library

This is ad-hoc: it contains only the things wanted so far. See:
[available widgets (latest release)](https://docs.rs/kas/latest/kas/widget/).


Installation and dependencies
----------------

#### Rust

KAS requires [Rust] version 1.52 or greater (currently in **beta**: *usually*
we maintain compatibility with the latest stable release).
Using the **nightly** channel does have a couple of advantages:

-   Proceedural macros emit better diagnostics. In some cases, diagnostics are
    missed without nightly rustc, hence **nightly is recommended for development**.
-   Documentation generated via `cargo doc` requires nightly for links
-   A few minor option things: see [Feature flags](#feature-flags) below.

#### WebGPU

Currently, KAS's only drawing method is [WebGPU] which requires DirectX 11/12,
Vulkan or Metal.
In the future, there may be support for OpenGL and software rendering.

#### HarfBuzz (optional)

This is only needed if the `shaping` feature is enabled. On my system, the
following libraries are used: `libharfbuzz.so.0`, `libglib-2.0.so.0`,
`libgraphite2.so.3` and `libpcre.so.1`.

### Quick-start

Install dependencies:
```sh
# For Ubuntu:
sudo apt-get install build-essential git libxcb-shape0-dev libxcb-xfixes0-dev libharfbuzz-dev

# For Fedora:
# glslc is optional; see kas-wgpu/README.md
sudo dnf install libxcb-devel harfbuzz-devel glslc
```

Next, clone the repository and run the examples as follows:
```sh
git clone https://github.com/kas-gui/kas.git
cd kas
cargo test
cd kas-wgpu
cargo build --examples
cargo run --example gallery
cargo run --example layout
cargo run --example mandlebrot
```

To build docs locally:
```
RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc --features markdown --no-deps --all --open
```

### Crates

-   `kas`: the *core* of the GUI library, providing most interfaces and logic
    along with a selection of common widgets
-   `kas-macros`: a helper crate for proc macros (do not use directly)
-   [KAS-text]: font loading, text layout, text navigation
-   `kas-theme`: theming support for KAS (API plus two themes; organisation may change)
-   `kas-wgpu`: provides windowing via [winit] and rendering via [WebGPU]

A user depends on `kas` to write their complete UI specification, selects a
theme from `kas-theme`, instances a `kas_wgpu::Toolkit`, adds the window(s),
and runs the UI.

### Feature flags

The `kas` crate has the following feature flags:

-   `shaping`: enables complex glyph forming for languages such as Arabic.
    This requires that the HarfBuzz library is installed.
-   `markdown`: enables Markdown parsing for rich-text
-   `serde`: adds (de)serialisation support to various types
-   `json`: adds config (de)serialisation using JSON (implies `serde`)
-   `yaml`: adds config (de)serialisation using YAML (implies `serde`)
-   `winit`: adds compatibility code for winit's event and geometry types.
    This is currently the only functional windowing/event library.
-   `stack_dst`: some compatibility impls (see `kas-theme`'s documentation)
-   `internal_doc`: turns on some extra documentation intended for internal
    usage but not for end users. (This only affects generated documentation.)

### Configuration

Formats are not yet stabilised, hence reading/writing configuration is disabled
by default. Ensure that the `yaml` and/or `json` feature flag is enabled, then
configure with environment variables:
```sh
# Set the config path:
export KAS_CONFIG=kas-config.yaml
# Use write-mode to write out default config:
KAS_CONFIG_MODE=writedefault cargo run --example gallery
# Now just edit the config and run like normal:
cargo run --example gallery
```

[KAS-text]: https://github.com/kas-gui/kas-text/
[winit]: https://github.com/rust-windowing/winit/
[HarfBuzz]: https://harfbuzz.github.io/
[WebGPU]: https://github.com/gfx-rs/wgpu-rs
[ROADMAP]: ROADMAP.md


Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
