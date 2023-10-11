Contributing
========

Contributions to KAS are welcome. It will be assumed that all
contributions are made available under the terms of the Apache License v2.0.

For very small changes, you may simply open a pull request; in other cases it
is recommended to open an issue first. This project does not currently make use
of third-party message boards.


Code style
---------

Code style is more art than science, but should:

1.  Use `cargo fmt`
2.  Make code clear and readable
3.  Aim to reduce the line count, when this doesn't conflict with the above

Sometimes introducing an extra `let` binding helps. Sometimes it makes sense
to wrap a `match`. Sometimes it makes sense to use `return`.

### Spelling

As is industry standard, APIs should use US-English spellings.
This rule is not enforced for documentation or local variables,
so long readability is not significantly impaired.

Notes:

-   'Config' is used as an abbreviation for 'configuration' (noun), *not* for 'configure' (verb)

### Nightly features

KAS optionally uses several Rust nightly features, but is functional without
(aside from some minor features).

### Unsafe

Usage of `unsafe` is allowed, but not preferred. Current use cases:

-   Defining constants requiring `unwrap` (tracker: [`const_option`](https://github.com/rust-lang/rust/issues/58732)). Note that since 1.57, `panic!` in const fns is supported, hence a work-around using `match` is possible.
-   To get around lifetime restrictions on the theme API's `Theme::draw` and `Window::size`
    methods; this will no longer require `unsafe` once the
    `generic_associated_types` feature is stabilised.
-   `Id` uses `unsafe` code to support both inline and heap-allocated
    variants.
-   Implementing `bytemuck::Pod` and `Zeroable`, as required to assert that
    values may be copied to the GPU.
-   Constructing a `wgpu::Surface`, as required to assert validity of the window
    handle.

Dependencies imply many more uses of `unsafe`; this includes:

-   Extern C APIs are commonly required (especially by `winit`)
-   GPU APIs
-   `smallvec` is widely used for in-place vectors; note that `tinyvec`'s
    restriction (`Item: Default`) makes it unsuitable for most of these uses
