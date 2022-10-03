KAS WGPU
======

[KAS] shell interface over [winit] and [wgpu].

[KAS]: https://crates.io/crates/kas
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
See [`build.rs`](build.rs) for details.

[glslang.js]: https://alexaltea.github.io/glslang.js/
[shaderc]: https://github.com/google/shaderc


Optional features
-------

This crate has the following feature flags:

-   `clipboard` (enabled by default): clipboard integration
-   `raster` (enabled by default): use [kas-text]'s default backend for glyph
    rastering (alternatively, specify `kas-text/ab_glyph` or `kas-text/fontdue`)
-   `shaping` (enabled by default): use [kas-text]'s default backend (Rustybuzz)
    for text shaping (alternatively, specify `kas-text/harfbuzz` or do not use
    shaping)

Note: at least one of `ab_glyph`, `fontdue` is required. If both are enabled,
the choice of raster engine is controlled at run-time via theme configuration:

-   `mode = 0`: use `ab_glyph`
-   `mode = 1`: use `ab_glyph` and align glyphs to side-bearing
-   `mode = 2`: use `fontdue`

[ab_glyph]: https://crates.io/crates/ab_glyph
[fontdue]: https://crates.io/crates/fontdue

Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
