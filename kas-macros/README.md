KAS Macros
========

This is a sub-library of KAS containing its procedural macros, and exists
solely because procedural macros must current be in a dedicated crate.

Users are advised not to depend on this library directly, but instead rely on
the main KAS lib, which re-exports these macros in its API.


Stable vs nightly
-----------------

This crate is compatible with **stable rustc**, however, usage of **nightly**
has some benefits:

-   More macro diagnostics are emitted, resulting in better error messages
    (without this, some errors may not even be reported)
-   With `#![feature(proc_macro_hygiene)]`, the `make_widget!` macro may be used


Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
