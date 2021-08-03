KAS Core
======


### Feature flags

The `kas` crate has the following feature flags:

-   `markdown`: enables Markdown parsing for rich-text
-   `config`: adds (de)serialisation support for configuration plus a few
    utility types (specifying `serde` instead only implements for utility types)
-   `json`: adds config (de)serialisation using JSON (implies `config`)
-   `yaml`: adds config (de)serialisation using YAML (implies `config`)
-   `ron`: adds config (de)serialisation using RON (implies `config`)
-   `winit`: adds compatibility code for winit's event and geometry types.
    This is currently the only functional windowing/event library.
-   `stack_dst`: some compatibility impls (see `kas-theme`'s documentation)
-   `internal_doc`: turns on some extra documentation intended for internal
    usage but not for end users. (This only affects generated documentation.)

Additionally, the following flags require a nightly compiler:

-   `spec`: use `specialization` to enable `TryFormat`
-   `gat`: compatibility with `kas-text/gat`


Copyright and Licence
-------

The [COPYRIGHT](COPYRIGHT) file includes a list of contributors who claim
copyright on this project. This list may be incomplete; new contributors may
optionally add themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the [LICENSE](LICENSE) file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
