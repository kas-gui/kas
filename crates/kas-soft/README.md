Kas CPU-rendering backend
=========================

[KAS] CPU-rendering backend over [softbuffer].

In its current state, this crate is usable with limitations:

-   Only the `SimpleTheme` theme is currently supported
-   Lines may be uglier than with `kas-wgpu` due to a lack of anti-aliasing and rounding
-   Fonts appear slightly darker than `kas-wgpu` due to differences in the blending
-   Performance seems mostly fine, though RGBA texture blending is a bit slow.

See also [#610](https://github.com/kas-gui/kas/pull/610).

[KAS]: https://crates.io/crates/kas
[softbuffer]: https://github.com/rust-windowing/softbuffer


Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
