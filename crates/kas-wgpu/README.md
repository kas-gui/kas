KAS WGPU
======

[KAS] graphics backend over [wgpu].

[KAS]: https://crates.io/crates/kas
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

-   `shaping` (enabled by default): use [kas-text]'s default backend (Rustybuzz)
    for text shaping (alternatively, specify `kas-text/harfbuzz` or do not use
    shaping)

By default, this crate uses [Swash] to raster fonts. This is usually the best
choice. As an alternative, fonts can be rastered using [ab_glyph]; this requires
both that the `ab_glyph` feature is enabled and that the configuration item
`font.raster.mode`  is set (see [Run-time configuration], [`RasterConfig::mode`]).

[Swash]: https://github.com/dfrg/swash
[ab_glyph]: https://crates.io/crates/ab_glyph
[`RasterConfig::mode`]: https://docs.rs/kas/latest/kas/config/struct.RasterConfig.html#structfield.mode
[Run-time configuration]: https://github.com/kas-gui/kas/wiki/Run%E2%80%90time-configuration

Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
