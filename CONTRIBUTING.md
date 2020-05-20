Contributing
========

Contributions to KAS are welcome. It will be assumed that all
contributions are made available under the terms of the Apache License v2.0.

For very small changes, you may simply open a pull request; in other cases it
is recommended to open an issue first. This project does not currently make use
of third-party message boards.
If necessary, I can be reached by email (@dhardy.name).


Code style
---------

Code style is more art than science, but should:

1.  Use `cargo fmt`
2.  Make code clear and readable
3.  Aim to reduce the line count, when this doesn't conflict with the above

Sometimes introducing an extra `let` binding helps. Sometimes it makes sense
to wrap a `match`. Sometimes it makes sense to use `return`.


Crates and repositories
--------------------------

A few separate crates are used: `kas-macros` because it is required,
`kas-theme` since themes should be independent of the core and the renderer,
`kas-wgpu` since multiple renderer implementations are intended. Additional
crates may be created, but one should consider whether or not this degree of
independence serves a purpose in the immediate future.

For now, almost everything is in a single repository to simplify version
management. Once APIs are fairly stable, it is likely that themes and renderers
and non-core widgets will end up in their own repositories.


Stability of APIs
-----------------------

Due to the early stage of development, APIs are frequently developed on an
as-needed basis. Some are merely placeholders, and some expose only a few of
the expected methods. For now, this will continue to be the case, but feel free
to add methods that are missing.

In some cases, home-grown types have been used to kick-start development where
ideally a third-party library should be used. This is especially true of colour
and geometry types; there are already issues for replacing these.

At this stage of development, breaking changes are fairly common.


Stable vs Nightly Rust
--------------------

KAS already makes use of several nightly-only features.
The following rules should be respected:

1.  At least some examples should be compatible with the latest stable `rustc`.
2.  Any nightly features with an obligatory warning on usage (e.g.
    `generic_associated_types`) should never be required.

Where nightly features are used to support additional API, this must be
feature-gated. In other cases (e.g. macro diagnostics), it should be
auto-enabled on nightly compilers.


Unsafe
------

Usage of `unsafe` is allowed, but should be minimised. Where used for non-core
functionality, this should be feature-gated. 

Current uses are:

-   defining constants (see https://github.com/rust-lang/rust/issues/58732)
-   implementing `Clone` for `Box<dyn Widget>` and the supporting `clone_to`
    method (this functionality is and will probably remain feature-gated)
-   to get around lifetime restrictions on the `draw_handle` and `size_handle`
    methods; this will no longer require `unsafe` once the
    `generic_associated_types` feature is stabilised
-   implementing `bytemuck::Pod` and `Zeroable` for data-types copied to GPU
    buffers; some type of assertion is required here

These rules extend to usage of third-party crates, with more allowances:

-    `winit` and `wgpu` must by their nature make frequent use of `unsafe` and
    we have no alternative (although it would be nice to have an alternative
    renderer which only uses the CPU)
-   `smallvec` is widely used for in-place vectors; note that `tinyvec`'s
    restriction (`Item: Default`) makes it unsuitable for most of these uses
-   `stack_dst` is used to avoid boxing `dyn Trait` objects in a few cases for
    optional functionality (switchable themes); this functionality may in the
    future be supported without `stack_dst` via `Box`, but for now there are
    already too many variants of this code!
