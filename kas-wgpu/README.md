KAS WGPU
======

Toolkit rendering over the [wgpu](https://crates.io/crates/wgpu) lib.


Optional features
-------

This crate has the following feature flags:

-   `clipboard` (enabled by default): clipboard integration
-   `font-kit` (enabled by default): uses the `font-kit` crate to locate a
    suitable system font; otherwise a hard-coded font path is used
-   `stack_dst` (enabled by default): enables `kas-theme::MultiTheme`
-   `gat`: enables usage of the Generic Associated Types feature (nightly only
    and currently unstable), allowing some usages of `unsafe` to be avoided.
    (The plan is to enable this by default once the feature is mature.)

Copyright and Licence
-------

The [COPYRIGHT](../COPYRIGHT) file from the main KAS library includes a list of
contributors who claim copyright on this project. This list may be incomplete;
new contributors may optionally add themselves to this list.

The KAS WGPU library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-MIT> file or on
the following webpage: <https://opensource.org/licenses/MIT>
