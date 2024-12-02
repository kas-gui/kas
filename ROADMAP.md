Roadmap
======

The past
--------

First, lets summarise KAS's journey so far.
For details, see the [CHANGELOG](CHANGELOG.md).

### 0.0.x — January 2019

Early releases, built over GTK.

### 0.1.0 — December 2019

A restart, replacing GTK with direct widget implementations including rendering
via the `rgx` crate. Initial theme traits, event-handling revision.

### 0.2.0 — February 2020

Lots of small but significant changes, including the introduction of the
`Manager` handle, user-defined `configure` code, scheduled updates (animation),
a scrollable region with scrollbars and addition of `ToolkitProxy`.

### 0.3.0 — February 2020

Only three weeks later, this delivered two new levels of draw API (portrayed by
`clock` and `mandlebrot` examples respectively), a "flat" theme, run-time theme
and colour-scheme switching, widget alignment, `StretchPolicy`, more adjustments
to event handling (removal of by-coordinate addressing), a `RadioBox` widget,
and window size limits.

### 0.4.0 — May 2020

This release revised widget traits into an integral family with opt-out
derivation (thus allowing future extension without breaking all existing
widgets), separating child-enumeration out of `WidgetCore` and separating
routing and handling of events.

Secondly, the release improved layout generation and notation, adding proper
margins, replacing `Horizontal` and `Vertical` with the four principal
directions, rewriting `solve_seq` to make only required changes (enabling
manual column resizing), and calculating the ideal window size before creation.

Thirdly (and no less significantly), the release added pop-up menus, including
fixes for overlapping graphics and robust event handling (including delayed
opening of sub-menus and navigation via tab and arrow keys).

Fourthly, it added access keys, including visible labels when Alt is
depressed and locality to the visible pop-up layer.

Small additions included recursive disabled states for all widgets and an error
state for `EditBox` (set via a user-defined input guard). Widgets included a
`Slider`, a resizable `Splitter`, a `ComboBox` and menu widgets.

### 0.5.0 — August 2020

This release focussed on text presentation and editing with a new library,
[kas-text](https://github.com/kas-gui/kas-text/). The text editing experience
has been massively improved, especially for multi-line text, and now supports
bidirectional text, glyph shaping, and all the expected selection, navigation
and editing functionality. Text presentation now supports justified text and
faster re-wrapping when resizing the window.

Some tweaks to input handling and resizing landed, including (unconfigurable)
short-cuts, double-click tracking and separate inner and outer margins.

Additionally, a [CONTRIBUTING](CONTRIBUTING.md) guide and this `ROADMAP`
have been added.

### 0.6.0 — November 2020

A continuation of the work on text, with intial support for rich text,
exemplified via a Markdown parser. Underline and strike-through may be applied
to texts at draw-time, and font adjustments (size, bold/italic and font family)
may be applied as part of text layout.

To allow retrieval of data from temporary (dialog) windows, the
`Window::on_drop` method was added.

This release also simplifies distribution by bundling pre-compiled GLSL shaders.

### 0.7.0 — April 2021

This release focussed on one of the remaining hard problems: data sharing. A new
framework for "view widgets" was introduced, allowing (interactive) views over
shared data.

View widgets enable synchronised access to shared data from multiple locations.
Additionally, by avoiding dedicated widgets for each data item in a set, views
are scalable to large data sets.

Also included are several improvements to sizing, widget construction and event
handling, as well as a new type-conversion library.

### 0.8.0 — June 2021

This release finally addressed one of the most obvious missing features of KAS:
images. At least, in their most basic form: static raster images loaded from
file sources. This new raster-image rendering system was then tweaked and used
as a replacement for the `glyph_brush` library, giving us better control over
glyph rendering and caching, and allowing a choice of font glyph rasterer.

The work on fonts and text did not stop there: as part of KAS-text 0.3, the
(rather large and mostly unused) dependency `font-kit` was replaced with
`fontdb` (plus a collection of custom aliases), extended to support font
fallbacks and run-time configuration, meanwhile `rustybuzz` was integrated as
a pure-Rust alternative to HarfBuzz for text shaping. These changes removed
all non-Rust dependencies from the text system.

Partially related to the above was the work on theme configuration, covering
colour schemes, font size, font family (per font class and global), and glyph
rastering.

### 0.9.0 — August 2021

Following the recent addition of image support, this release takes the
opportunity to revise the draw API, culminating in a new unified `Draw` trait
and `DrawIface` implementation, accessible via `DrawHandle::draw_device`. This
simplifies usage of mid-level draw APIs, e.g. in themes and the `clock` example.

Also in this release is SVG support, a `Canvas` widget, image buttons, and
fixed sub-pixel positioning (clearer rendering for small text).

### 0.10.0 — September 2021

This release responds to three key criticisms of KAS: (1) slow compile times,
(2) non-standard keyboard navigation, (3) ugly themes. While these issues are
not fully solved, they are significantly improved. Additionally,
the latest version of WGPU now gives us a working OpenGL backend.

### 0.11.0 — September 2022

Widget trait revision: remove `Handler::Msg` associated type, replacing with a
variadic message stack: `EventMgr::push_msg` and `try_pop_msg`. Notify of data
model updates via broadcast.

New `#[autoimpl]` and `impl_scope!` macros, published in the
[impl-tools crate](https://crates.io/crates/impl-tools).

Added a domain-specific macro-driven language for widget layout, thus permitting
complex layout from a single widget.

Support sub-tree configuration. Make `WidgetId` a path.

Support theme- and widget-driven animations. Support momentum scrolling.

### 0.12.0 — December 2022

Stabilised support for Generic Associated Types, allowing some changes to data
models.

### 0.13.0 — February 2023

Async rendering for `Canvas` and `Svg` widgets.

Support Linux's primary buffer. Support transparent and borderless windows
with very basic support for a toolkit-drawn titlebar.

Removal of `kas-theme` crate (most contents merged into `kas-core`).

### 0.14.0 — December 2023

Add *input data*: widgets now have a `Data` associated type, passed by reference
to event handlers and to a new `update` method. The key advantages of this change are:

-   Declarative specification of trees with changeable widgets. For example, before this change a `counter` must construct a label displaying the initial count *and* explicitly update this label when the count changes; now the displayed label is effectively a function of the count.
-   As a direct result of the above, it is no longer necessary for parent nodes to refer to children by name, thus it is no longer necessary to use structs with named fields (at least in the common cases).
-   "View widgets" are now much more tightly integrated with other widgets, making the `SingleView` widget redundant.

For more on *input data*, read the [design document](https://github.com/kas-gui/design/blob/input-data/widget/input-data.md).

### 0.15.0 — December 2024

Add traits `Collection` and `CellCollection` representing a list/tuple of widgets.
Revise layout macros with `.align` / `.pack` / `.margins` adapters.


Future work
-----------

These items are not versioned and appear only roughly in the expected order.
Each should be a decently sized work item (roughly one release).

### Standard geometry types

KAS has ad-hoc geometry types. *Possibly* it would be useful to use third-party
types instead. See [#95](https://github.com/kas-gui/kas/issues/95).

### Context menu and undo

KAS supports pop-up menus since 0.4. Context menus are a little different, in
that the widget opening the menu does not own it, yet can still customise it
(enabling or disabling items like undo, paste, perhaps providing custom items,
perhaps selecting a special context menu).

At the same time, the undo history should probably be removed from widgets and
stored in some shared state.

Slightly related to this is support for global and standard shortcuts such as
undo, copy selection, save file, quit app.

### Desktop integration

This is less a separate work item than it is a long-term goal, one which will
require substantial help. KAS should attempt to discover initial configuration
from the current desktop environment, e.g. the current font and font size, the
icon set, locale and short-cuts.

Additionally, on Linux it is expected that the desktop environment provide a
few standard dialog boxes (e.g. file open/save), not only for consistency but
also security (e.g. a container may not let an app explore the filesystem).
Such dialogs should automatically use desktop-provided equivalents where
available.

### Input Method Editors and virtual keyboards

Winit has at least partial support for IME now. Kas should add support for this
(mostly this means an additional `Event` variant, one or two API calls to
enable IME input for the current widget, and adjusting `EditField`).

### Accessibility tools

`AccessKit` would appear to be the defacto Rust API for accessibility tools with
(limited) support from egui and Xilem toolkits. Widgets could support this via
additional code in the `update` method, possibly with additional tracking to
avoid unnecessarily replacing nodes.

### Internationalisation (i18n)

[Project Fluent](https://projectfluent.org/) offers the core functionality
required for localisation (l10n). We may need additional code for loading
resources and selecting languages; potentially also for fallbacks (in case of
incomplete translations). Widgets could translate their strings in the
`configure` method when `fluent` integration is enabled with labels treating
their input as a message key.

### Embedded video

To investigate. Gstreamer integration should be viable when using a (new) OpenGL
backend.

Integrating any video player as a child window should be possible (see Winit's
`WindowBuilder::with_parent_window`, which is not yet supported everywhere).

### Embedded web browser

To investigate. Possibly use Servo (components), though it may not matter much.

The browser should probably be integrated via a child window (see Winit's `WindowBuilder::with_parent_window`, which is not yet supported everywhere).


External dependencies
----------------------

### Clipboard support

We have plain text clipboard support via
[window_clipboard](https://github.com/hecrj/window_clipboard), but lack support
for formatted text, images, etc.

### (winit) pop-up window support

It has been requested that winit support popup windows for things like menus
which are not restricted to the parent window; currently it doesn't.
See [winit#950](https://github.com/rust-windowing/winit/issues/950) (and other
issues).

### (winit) drag and drop

Winit *does* have support for this, but only in a very limited fashion. With
its current event model it is difficult or impossible to determine the widget
receiving a drop or under a hovered drop.
See [#98](https://github.com/kas-gui/kas/issues/98) and
[winit#1550](https://github.com/rust-windowing/winit/issues/1550).
