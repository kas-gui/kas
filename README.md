KAS GUI
==========

[![home](https://img.shields.io/badge/GitHub-home-blue)](https://github.com/dhardy/kas)
[![old-home](https://img.shields.io/badge/GitLab-old--home-blueviolet)](https://gitlab.com/dhardy/kas)

KAS, the *toolKit Abstraction System*, is a general-purpose GUI toolkit.
**Goals** of the project are:

-   Fully-functional, intuitive GUIs
-   Embeddable within games or any window manager
-   Fancy/highly flexible hardware-accelerated rendering
    (but in theory software rendering could be supported too)
-   Easy, expressive specification within code (currently impeded by
    Rust language limitations which will hopefully be solved in the future)
-   Custom widgets in user code without limitations
-   Bug-free, with an API facilitating compiler correctness lints
-   High performance / low resource usage, other than optional fancy graphics

**Status** of the project is **alpha**: progress has been made towards all
goals, with signficiant limitations to features and goofy graphics.
Portability is somewhat limited, requiring nightly Rust and [`wgpu`] support.

**Crates:**

-   `kas`: the *core* of the GUI library, providing most interfaces and logic
    along with a selection of common widgets
-   `kas_macros`: a helper crate providing the procedural macros used by `kas`
-   `kas_wgpu`: provides windowing via [`winit`] and rendering via [`wgpu`]
-   `kas_widgets`: (unrealised) - providing extra widgets
-   `kas_graphs`: (unrealised) - plotting widgets

A user depends on `kas` to write their complete UI specification, and then
pastes a few lines of code to initialise `kas_wgpu::Toolkit`, choose a theme,
add window(s), and run the UI.

[`winit`]: https://github.com/rust-windowing/winit/
[`wgpu`]: https://github.com/gfx-rs/wgpu-rs


Examples
---------

![Calculator](screenshots/calculator.png) ![Dynamic](screenshots/dynamic.png)
![Theme](screenshots/theme.png) ![Gallery](screenshots/gallery.png)
![Layout](screenshots/layout.png)


Installation and Testing
------------------------

For the most part, Cargo should take care of dependencies, but note:

-   [shaderc may require some setup](https://github.com/google/shaderc-rs#setup)
-   [wgpu](https://github.com/gfx-rs/wgpu-rs) requires DirectX 11/12, Vulkan or
    Metal (in the future it may support OpenGL)

Next, clone the repository and run the examples as follows:

```
git clone https://github.com/dhardy/kas.git
cd kas
cargo test
cd kas-wgpu
cargo test
cargo run --example gallery
```


Features
----------

-   Custom parent widgets with embedded state (at in Qt)
-   Type-safe event handlers from the context of these widgets
-   Custom widgets over high- or low-level event API
-   Custom widgets over high-level draw API (TODO: low level option)
-   Column / row / grid+span layouts (TODO: flow boxes, manual positioning)
-   Width-for-height sizing
-   Custom themes (with full control of sizing and rendering)
-   Touch-screen support
-   Keyboard navigation & accelerator keys
-   Scalable (HiDPI) including fractional scaling
-   Memory and CPU efficient


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
