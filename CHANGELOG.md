# Changelog
The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — unreleased
This is a decent sized release, focussing primarily on drawing, themes and
layouts. Highlights include a new `FlatTheme`, many small visual improvements,
access to medium-level and low-level drawing APIs for custom widgets,
window size limits, and switchable themes and colour schemes.

### Examples
-   Readme for examples (#61, #62)
-   Clock example now draws an analogue clock face (#63)
-   New Mandlebrot example with a custom draw pipe (#64)

### Dependencies
-   Update `winit` to 0.21 (#51)

### Core traits
-   Add `ThemeApi`, `ThemeAction` for run-time adjustment (#53)
-   `Layout::draw` methods now receive `&ManagerState`, not `&Manager` (#57)
-   Replace `WidgetCore::find_coord_mut` with `Layout::find_id` (#51)

### Layouts
-   Tweak margins; buttons now (temporarily) have an external margin (#53)
-   Allow parents to influence child aliginment (#52)
-   Non-parent widgets may now have frames (#52)
-   `layout_data` is no longer required for `single` layout (#52)
-   Add `StretchPolicy` to `SizeRules`; adjust `AxisInfo` (#52)
-   Move `Align`, `Direction` and friends to `kas::data`; `Direction` is now an enum (#52)

### Event handling
-   Allow setting a custom cursor for a mouse grab (#54)
-   Removal of `kas::event::Address`: all events are now addressed by `WidgetId` (#51)
-   Update handles gained a `u64` payload (#51)
-   `PressEnd` no longer sent without a press grab (#51)

### Macros
-   Support substitution of generics in handler impls (#51)

### Widgets
-   New `Filler` widget (#52)
-   New `RadioBox` widget (#51)
-   Add `CheckBoxBare` (#51)

### Draw APIs and pipes
-   Support custom draw pipes (#64)
-   New `FlatRound` pipe (#53)
-   Use multisampling in `FlatRound` and `ShadedRound` pipes (#53)
-   Move `Draw`* traits to `kas::draw` (#58, #60)
-   `Draw`: add `rect`, `frame` methods; remove `draw_quad` (#50)
-   New `DrawRounded`, `DrawShaded` traits (#50, #60)
-   `rounded_frame` method supports `inner_radius` (#58)
-   Use higher-level API in `DrawText` (#58, #63)

### Themes
-   New `kas-theme` crate, encapsulating content from `kas` and `kas-wgpu` (#60, #57)
-   New `FlatTheme` (#53)
-   Prettier navigation highlights and (some) frames (#58)
-   New `MultiTheme` for run-time switching (#53)
-   Add `stack_dst` feature, `ThemeDst` trait; use for `MultiTheme` (#54, #61)
-   Experimental, feature-gated use of Generic Associated Types to avoid `unsafe` usage (#57, #61)
-   Add `DrawHandle::draw_device` to get lower-level draw API (#63)
-   Add `Theme::update_window` method (#53)
-   Remove `Theme::light_direction`, `get_fonts` methods; add `init` (#63)
-   New `ThemeColours` struct; colour themes (#53)
-   New `Dimensions` struct and friends for `Theme::Window` impl (#53)
-   Themes now use integer arithmetic internally (#50)

### Toolkit
-   Read `KAS_BACKENDS`, `KAS_POWER_PREFERENCE` env vars (#50)
-   Additional parameters to `Toolkit::new_custom` (#50, )

### Misc
-   Continuous Integration via Travis (#61)
-   `kas::geom`: add `Coord::uniform`, `Rect::shrink` (#50)
-   Window sizes may now have upper/lower limits (#51)


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
