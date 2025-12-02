KAS Core
======

This is the core KAS crate.
See also the [kas crate ](https://crates.io/crates/kas).

For documentation of feature flags, see [Cargo.toml](Cargo.toml).


Font rendering
--------------

Font shaping (recommended) may be enabled using the `shaping` feature. This uses
[Rustybuzz](https://github.com/harfbuzz/rustybuzz).

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
