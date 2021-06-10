KAS WGPU
======

KAS shell interface over [winit] and [wgpu].

[winit]: https://github.com/rust-windowing/winit/
[wgpu]: https://github.com/gfx-rs/wgpu-rs


Compiling shaders
-----------------

This library uses GLSL shaders. Pre-compiled SPIR-V modules are distributed so
that users do not need a shader compiler.

For working on GLSL shaders, a compiler such as `glslc` (part of the [shaderc]
project) is required. `glslc` can be installed from Fedora packages, but on
other platforms manual installation may be required. Alternatively a web-based
tool such as [glslang.js] may be used.

Automatic re-compilation may be enabled by setting e.g. `SHADERC=glslc`.
See [`build.rs`] for details.

[glslang.js]: https://alexaltea.github.io/glslang.js/
[shaderc]: https://github.com/google/shaderc


Optional features
-------

This crate has the following feature flags:

-   `clipboard` (enabled by default): clipboard integration
-   `fontdue`: use [fontdue] library for font rasterisation (otherwise, `ab_glyph` is used)
-   `stack_dst` (enabled by default): enables `kas-theme::MultiTheme`
-   `gat`: enables usage of the Generic Associated Types feature (nightly only
    and currently unstable), allowing some usages of `unsafe` to be avoided.
    (The plan is to enable this by default once the feature is mature.)
-   `unsize`: forwards this feature flag to `kas-theme`

[fontdue]: https://github.com/mooman219/fontdue

Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
