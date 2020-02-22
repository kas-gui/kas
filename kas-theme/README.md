KAS Theme
======

Theme support for KAS.


Optional features
-------

This crate has the following feature flags:

-   `stack_dst`: enables `ThemeDst`, `WindowDst` and `MultiTheme` types via the
    `stack_dst` crate. This crate relies on `unsafe` code and its functionality
    is required only for run-time switching of themes.
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
