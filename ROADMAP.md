Roadmap
======

The past
--------

First, lets summarise KAS's journey so far.
For details, see the [CHANGELOG](CHANGELOG.md).

### 0.0.x — Jan 2019

Early releases, built over GTK.

### 0.1.0 — Dec 2019

A restart, replacing GTK with direct widget implementations including rendering
via the `rgx` crate. Initial theme traits, event-handling revision.

### 0.2.0 — Feb 2020

Lots of small but significant changes, including the introduction of the
`Manager` handle, user-defined `configure` code, scheduled updates (animation),
a scrollable region with scrollbars and addition of `ToolkitProxy`.

### 0.3.0 — Feb 2020

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

Fourthly, it added Alt-accelerator keys, including visible labels when Alt is
depressed and locality to the visible pop-up layer.

Small additions included recursive disabled states for all widgets and an error
state for `EditBox` (set via a user-defined input guard). Widgets included a
`Slider`, a resizable `Splitter`, a `ComboBox` and menu widgets.


Current work
-----------

The next release will include proper text-edit handling (positionable edit
marker, selection), and likely one further item from the list below.

Additionly, it includes a `CONTRIBUTING` guide and this `ROADMAP`.


Future work
-----------

These items are not versioned and appear only roughly in the expected order.
Each should be a decently sized work item (roughly one release).

### Standard geometry types

KAS has ad-hoc geometry types. *Possibly* it would be useful to use third-party
types instead. See #95.

### Images and icons

Support display of images in the GUI:

-   fixed-size raster images sized to the pixel count without scaling
-   scaling of fixed raster images
-   image display using a target size and multiple rastered versions, with
    the option of scaling to the target size or using the nearest size
-   vector images rastered to a target size
-   buttons with embedded images

Possibly as part of this topic, implement colour management #59.

### Configuration and resource management

Currently KAS has an ad-hoc font loader and fixed colour-schemes.
This work item includes:

-   discovery of resources (fonts, icons, colour-schemes) from the system and
    from user-local directories
-   configuration for e.g. fonts, colour schemes, icon sets
-   overriding the scale factor
-   shortcuts (e.g. Ctrl+Z), including configuration and maybe some localisation

### Standard resource sets

Ideally, KAS should provide an identifier for common icons and either map these
to a system-provided icon set or provide its own, so that apps can use icons
like "save" or "undo" without having to provide the icon themselves.

Colour schemes and short-cuts are similar in that potentially they can be
inherited from the desktop, but otherwise KAS should provide them.

### Multiple fonts and text markup

Support at least italic and bold fonts.

Support reading text markup (e.g. Markdown or HTML) and rendering this text
within e.g. the `Label` widget.

### Text shaping

Current text shaping uses the layout engine provided by `glyph_brush`. We should
likely replace this with a full text shaping engine, possibly `allsorts` or
`harfbuzz`.

While doing this, we can probably remove the requirement that `SizeHandle` have
a reference to per-window `draw` state, and possibly also support justified
alignment.

*Possibly* also within the same work-set, add a real multi-line text-edit
widget.

### Widget identifiers

Currently widgets are identified simply by enumerating all widgets. See #91

### View widgets

Viewing a table or spreadsheet should not require a dedicated widget for each
cell, for scalability reasons (even though KAS can viably use a million
widgets). This could be extended to viewing only a slice of a remote database.

The [view branch](https://github.com/kas-gui/kas/tree/view) has some initial
work and notes on this topic.

### Widget library

Although the current widget set covers a good portion of the "full complement"
mentioned by #2, its primary purpose is to prototype requirements for other APIs
(including the widget trait family, event handling, draw model).

Once other APIs are more stable, this focus should shift to providing the full
complement of standard GUI widgets, likely within a dedicated crate.

Additionally, several standard dialog boxes / pop-ups should be added, e.g. a
colour picker, a date picker, and a file-open dialog.

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


External dependencies
----------------------

### Rust

KAS currently *does* support stable `rustc`, but with feature limitations.
Getting everything working well on stable Rust *requires* some new Rust
features, though not all of these issues have a clear solution:

-   #25 lists existing (optional) usage of Rust nightly features; all of these
    would be great to have in stable Rust but must are not essential
-   `proc_macro_hygiene` is used to enable the `make_widget!` macro; this is not
    an *essential* part of KAS but is *extremely useful*; `proc-macro-hack`
    provides a workaround (with significant limitations)
-   #15 documents a major limitation of `make_widget!`; Rust's RFC 2524 provides
    a solution but has not been accepted or implemented
-   #11 documents one example of a bad error message; another case of bad error
    messages is [the sole reason `msg` does not have a default type within `make_widget!`](https://github.com/kas-gui/kas/blob/master/src/macros.rs#L381);
    it is not clear (to me) how best to solve these issues

### WebGPU and CPU rasterisation

Currently, KAS can only draw via `wgpu`, which currently does not support OpenGL
or CPU-rendered graphics, making KAS unusable on many older systems.

It seems likely that `wgpu` will support OpenGL in the future.

Additionally, KAS should provide a CPU-based renderer. See #33.

### Clipboard support

The current clipboard dependency is sub-par.
[window_clipboard](https://github.com/hecrj/window_clipboard) may be the path
forward, but still needs a lot of work (even copy-to-clipboard support).

This includes support for formats other than plain text, e.g. images and HTML.

This is not a trivial topic, especially considering that platforms have very
different approaches to this (e.g. both X11 and Wayland expect apps to publish
a list of available formats by mime type, then send contents in the window's
event handler, while other platforms usually have more restricted formats and
expect data to be sent to the clipboard provider in all formats up front).

### (winit) pop-up window support

It has been requested that winit support popup windows for things like menus
which are not restricted to the parent window; currently it doesn't.
See https://github.com/rust-windowing/winit/issues/950 (and other issues).

### (winit) drag and drop

Winit *does* have support for this, but only in a very limited fashion. With
its current event model it is difficult or impossible to determine the widget
receiving a drop or under a hovered drop.
See #98 and https://github.com/rust-windowing/winit/issues/1550

### (winit) full key-bindings

Winit's `VirtualKeyCode` enum is rather limited. See #27 (and *several* winit
issues) on this topic.
