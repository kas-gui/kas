KAS Macros
========

This is a sub-library of KAS containing its procedural macros, and exists
solely because procedural macros must current be in a dedicated crate.

Users are advised not to depend on this library directly, but instead rely on
the main KAS lib, which re-exports these macros in its API.


Stable vs nightly
-----------------

Note that proc macros may emit error messages on stable rust, but currently can
only emit warnings with nightly `rustc`.


Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
