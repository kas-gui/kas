# Changelog
The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2020-02-02
### Examples
-   New `sync-counter` example (#46)

### Widgets
-   Add a `ScrollBar` widget; add scroll bars to `ScrollRegion`
-   Re-added separate `Layout` trait (#42)
-   Widget configure now happens on init and may run user-defined code (#36)
-   Widgets can now directly schedule updates on timer (#42)
-   Widgets updates can now be triggerred via an `UpdateHandle` (#46)
-   Rename `WidgetCore::get_by_id` → `find` and add `find_mut` (#42)
-   Add `WidgetCore::find_coord_mut` (#47)

### Event handling
-   `Manager` renamed to `ManagerState`; new `Manager` wrapper type added (#39)
-   Event handlers take a `&mut Manager` parameter instead of `&mut dyn TkWindow` (#39)
-   `Manager::set_char_focus` renamed to `request_char_focus`
-   Remove `Event::Identify` (#47)
-   Several fixes for subtle event-handling issues

### Macros
-   `make_widget!` macro now uses attributes instead of `layout => Msg` parameter (#42)
-   Support generics within `make_widget!` macro (#46)

### Other KAS changes
-   Add `kas::WindowId` type
-   Add `TkAction::RegionMoved` to update `ManagerState` after scroll region move (#47)

### Dependencies
-   Use `log` crate for logging; improved log messages
-   Use `shaderc` to compile shaders
-   Use `smallvec` for some optimisations
-   Update to `winit` version 0.20.0 (from pre-release version)

### Toolkit
-   Support creation of new windows from event handlers (#34)
-   Add `ToolkitProxy`, allowing some control from other threads (#36)
-   Add `kas_wgpu::shared` private module
-   Rename `event` module to `event_loop`; refactor code (#36, #42, #46)

## [0.1.0] — 2019-12-22
-   Add install/testing section to README
-   `kas_wgpu::Toolkit::new` now returns a `Result`
-   Optimise drawing and event handling for long list widgets
-   Rename "dynamic" widgets to `List`, `BoxList`, `Row`, etc. and extend API

## [0.1.0-pre.1] — 2019-12-17
This is a *massive* release in terms of the number of changes since 0.0.2. Most
parts of the project have seen revision (perhaps least of all the macros). To
summarise:

-   `kas-gtk` has been replaced with `kas-rgx` which was then replaced with
    `kas-wgpu`.
-   Drawing and theming traits have been introduced in preliminary form.
-   Event handling has changed significantly.
-   `Class` has been removed, allowing fully custom widget types.

## [0.0.2] — 2019-01-24
-   Add trait references to `Class` enum as alternative to widget downcast
-   Generalise callbacks: new module, additional `Condition` codes
-   Replace `event::ignore` with `err_unhandler` and `err_num` fns
-   Add `Text::new()` and `Entry::new_on_activate(text, handler)`

### Macros
-   Tweak `make_widget` syntax to `container(vertical) => ...`
-   Move macro doc to main crate and improve examples
-   Allow `kas` crate self reference to simplify proc macros and allow in doc tests

## [0.0.1] — 2019-01-12
The initial release.
