KAS GUI
==========

The toolKit Abstraction System
is not a GUI toolkit in the traditional sense — instead it provides an
abstraction layer between an application's GUI description and the toolkit.

Current features:

-   very succinct specification of GUIs via procedural macros
-   hierarchical widget tree *without* backreferences to parents
-   a minimal selection of widgets
-   (planned) easy switching between toolkits (backends)
-   low memory and CPU usage
-   an **unstable API**: this is still an early prototype

Planned features:

-   a Rust-native toolkit using OpenGL or Vulkan rendering
-   wrappers for system-native toolkits
-   a full widget selection


Background
-----------

Rust has several existing GUI tools / projects:

-   [gtk-rs] — high quality bindings for GTK+ 3;
    app code feels similar to GTK C code but with increased type safety
-   [Relm] — a Model-Update-View design inspired by Elm built over [gtk-rs]
-   [Conrod] — a cached immediate-mode GUI
-   [Azul] — a function-oriented GUI built over a DOM
-   [Druid] — Data-oriented Rust User Interface Design toolkit

[gtk-rs]: https://gtk-rs.org/
[Relm]: https://github.com/antoyo/relm
[Conrod]: https://github.com/PistonDevelopers/conrod
[Azul]: https://github.com/maps4print/azul
[Druid]: https://github.com/xi-editor/druid

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

Components
---------------

A GUI system needs at least the following components:

-   a windowing library (e.g. winit or GDK)
-   a graphics drawing library (provided by the windowing library directly or
    via another abstraction, e.g. OpenGL)
-   widget drawing (dimensions and graphics — the theme)
-   widget sizing and positioning code — the layout system
-   event handling framework
-   GUI description — the application

This library focuses on providing a clean API for the last item (the
application) via an abstraction layer over widget drawing, positioning and
event handling. Specifically, this library provides:

-   a *toolkit API*, whose implementation provides the windowing API and widget
    dimensions and graphics, and optionally layout and/or event handling code
-   an optional widget layout system
-   an optional event handling system
-   an API and tools to help users build their application GUIs

This approach allows both the usage of a high-level GUI toolkit doing all the
heavy lifting (the first toolkit being a wrapper around GTK) and implementation
of a complete toolkit from scratch (in theory; this has yet to be done).

This design should therefore eventually support building applications using
native widget rendering on all major desktops from a single source, as well as
the option to use a toolkit which minimises non-Rust dependencies for ultimate
performance and portability.


Widgets
--------

Widget behaviour is described via four traits, all of which are typically
implemented via macro:

-   The `Core` trait handles access to common, core data; this is typically
    implemented over a `CoreData` struct field.
-   There are two variants of the `Layout` trait; this must be implemented by
    macro since it requires access to non-public parts of the API.
-   The `Widget` trait handles a few common operations implemented over the
    above traits, including access to child widgets.
-   The `Handler` trait implements event handling. This trait uses an associated
    type to allow user-defined return values, which may be caught and handled
    by a parent widget.

### Built-in widgets

The library provides some standard widgets: a text label, a push-button, etc.
Currently only a few are available; this should be expanded to a full set.

Some of these standard widgets are templated in order to allow user-defined
payloads to be returned from handled events; e.g.:

    TextButton::new("+", || Message::Incr)

defines a push-button labelled `+` which returns the enum value `Message::Incr`
when clicked. This allows application logic to be implemented on a parent widget
which encapsulates its controls.

### Layout widgets and make_widget

Simple (or complex) widgets are typically encapsulated in parent widgets, which
position each sub-widget relative to the self and encapsulate event handling.
Typically such widgets are single-use. This library provides a convenient method
of constructing them: the `make_widget` macro.

This macro creates a new struct type, implements all widget traits for this
type, then constructs a new instance using the given values. Note that in many
cases the types of sub-widgets need not be explicitly given and often the names
of fields are not important; this macro allows both to be omitted.

The macro syntax is complex and likely to be refined; see the examples and the
API documentation for details.

### Custom widgets

The `make_widget` macro mentioned above is merely provided for convenience; its
usage is not required (compare the `counter` and `counter_expanded` examples).


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
