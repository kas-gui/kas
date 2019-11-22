KAS GUI
==========

[![home](https://img.shields.io/badge/GitHub-home-blue)](https://github.com/dhardy/kas)
[![old-home](https://img.shields.io/badge/GitLab-old--home-blueviolet)](https://gitlab.com/dhardy/kas)

The name KAS comes from *toolKit Abstraction System*. The original goal of this
project was to build a UI using native widget libraries configured from pure
Rust; aims to use native widget libraries have been dropped.

The `gtk` branch is outdated but maintains integration with the GTK toolkit.


UI specification
--------------

UI models are written in pure Rust such that:

-   UI design specification is mostly declarative
-   UI specification is *succinct*, macro based
-   Event handling is local to the widgets and maximally type-safe

Specification relies heavily on procedural macros. This may have some impact on
compile times, but so far this does not appear to be a significant issue (tested
~10 seconds with 100 layout widgets vs ~5 sec for a simple UI).

### Event handling

To maximise use of type-safety, each widget's event handler may have a custom
response type, with handler and additional state integrated into the parent
widget.

### Example

The following is extracted from the `counter` example:

```rust
let buttons = make_widget! {
    container(horizontal) => Message;
    struct {
        #[widget] _ = TextButton::new("−", Message::Decr),
        #[widget] _ = TextButton::new("+", Message::Incr),
    }
};
let window = Window::new(make_widget! {
    container(vertical) => ();
    struct {
        #[widget] display: Label = Label::from("0"),
        #[widget(handler = handle_button)] buttons -> Message = buttons,
        counter: usize = 0,
    }
    impl {
        fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: Message)
            -> EmptyMsg
        {
            match msg {
                Message::Decr => {
                    self.counter = self.counter.saturating_sub(1);
                    self.display.set_text(tk, self.counter.to_string());
                }
                Message::Incr => {
                    self.counter = self.counter.saturating_add(1);
                    self.display.set_text(tk, self.counter.to_string());
                }
            };
            EmptyMsg
        }
    }
});
```

Style and rendering
-----------

UI style and rendering is offloaded to a separate crate, whose responsibilties
include window management, widget sizing, widget rendering, and
interfacing with platform event handling. Despite this, the majority of
widget specification, event handling and positioning is handled by the core
`kas` crate (plus internal `kas-macros` crate), facilitating creation of
additional rendering crates.

Rendering is currently handled by `kas-wgpu`. In the name of simplicity, this is
not configurable. Rendering is hardware-accelerated (via `wgpu` aka WebGPU with
support for multiple backends: Vulkan, DX12, DX11, Metal).

Custom styling should (eventually) be achievable in two ways:

-   development of a new rendering crate
-   adjusting run-time configuration of an existing rendering crate


Motivation
----------

Rust currently has a smattering of GUI libraries, but none currently offer the
full complement of features which really show off the strengths of the Rust
language:

-   **safe**: GUIs are complex, high-level constructions; they really should
    make it easy to write memory-safe, and thread-safe and correct code
-   **simple**: while the behaviour expressed by GUIs is complex, the ideas behind
    them are usually not; GUI app code should consist of a simple description
    of the GUI with minimal bindings to application logic
-   **flexible building blocks**: the Rust language has succeeded in keeping the
    language specification *moderately* simple while buliding a rich library
    on top of this; a Rust GUI library should do the same
-   **static type model**: Rust has succeeded in allowing most types to have
    compile-time known size and static linkage; not only does this avoid
    unnecessary run-time memory allocation, but also enables a *lot*
    of compile-time optimisation

Note that certain trade-offs must be made to allow the above goals; in
particular this means the library will not be easy to use via FFI (e.g. from C):

-   **complex types**: many widget types are complex or outright unnameable
-   **heavy use of macros**: while users *should* be able to implement
    functionality directly, this may be verbose and monotonous; macros can be
    used to construct the necessary implementations more succinctly

It is also worth noting that we currently make no attempt to support building
GUIs at run-time. This should eventually be possible to some extent, but is not
a current goal.


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
