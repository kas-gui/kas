# Changelog
The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] — 2020-08-13
This release largely concerns text formatting, with the new `kas-text` library.

It also saw the addition of the [CONTRIBUTING](CONTRIBUTING.md) and
[ROADMAP](ROADMAP.md) documents and, as usual, several doc improvements.

### text and kas-text library
This is a new library written for use with KAS and integrated from #110 onwards,
supporting glyph shaping, bidirectional text and much better line-wrapping
(faster, line metrics and direct support for coord/glyph-position translation).
For more on this library, see [kas-text](https://github.com/kas-gui/kas-text/).

-   Initial integration (#110)
-   Move `Align` to `kas-text` and re-export (#110)
-   Remove `CowString` (#111)
-   Support shaping via HarfBuzz (#113)
-   Add `PreparedTextExt` extension trait (#116)
-   Initial (hacky) support for text effects (#119)

### Text editing and EditBox
-   Draw a real text edit marker (#106)
-   Use `unicode-segmentation` for prev/next grapheme (#106)
-   Keyboard and mouse text navigation (#106, #113, #115, #119)
-   Enable text selection (#108, #113, #118)
-   Enable double-click and triple-click selection modes (#117)
-   Draw text selection (#108, #110, #112)
-   Support scrolling within `EditBox` (#116, #118)
-   PageUp/Down distance depends on view size (#119)

### Input handling

-   Widgets may now handle the Escape key (#118)
-   Initial support for shortcuts (hard-coded) (#116)
-   `ModifiersState` is no longer passed with `EventControl` but accessed via `Manager` (#113)
-   `Manager` now tracks double-clicks (and triple-, etc.)
-   Rename `event::NavKey` → `event::ControlKey` (#106)

### Sizing and styles

-   Setting a string no longer automatically resizes the UI (#116)
-   Separate inner and outer margins (#114, #115)
-   Various style tweaks (#105, #114)
-   `ShadedTheme` now re-uses parts of `FlatTheme` (#109)
-   New `DrawHandleExt` extension trait (#108)
-   Add `DrawHandleExt::size_handle` and `Manager::size_handle` (#107, #108)

### Widgets

-   Add `WidgetCore::with_disabled` (#104)
-   Adjust `SliderType` trait requirements (#105)
-   `Slider`: round to nearest int instead of rounding down (#114)
-   Extend `ComboBox` API (#104)

### Misc

-   Update for Rust 1.45: `proc_macro_hygiene` is obsolete (#113)
-   Revise class traits regarding text (#110, #111)
-   Align type now has `Default`, `TL`, `BR`, `Centre` and `Stretch` values (#110)
-   Update `wgpu_glyph` and `font-kit` dependencies (#106, #110)
-   Force window size to be at least 1×1 to avoid crash (#109)


## [0.4.1] — 2020-05-07
-   Remove screenshots from package
-   Document main breaking changes below

## [0.4.0] — 2020-05-07
Another fairly large release, with a lot of changes to input handling, a
revision to the trait model, support for pop-up widgets, and
(limited) support for **stable rustc**.

### Main breaking changes
The most likely changes needed to update 0.3 → 0.4:

-   Replace `Label::from` with `Label::new`
-   Replace `layout(horizontal)` with `layout(row)`, `layout(vertical)` with `layout(column)`
-   For several methods like `set_text`, replace `w.set_text(mgr, text)` with `*mgr += w.set_text(text)`

### Widget traits and API
The `Widget` trait model has seen significant revision (#75, #74, #85):

-   `WidgetChildren` added (split out from `WidgetCore`)
-   `WidgetConfig` added (taking all methods from `Widget`)
-   `event::Handler` split into `Handler` and `SendEvent`
-   `Widget` is now just a trait bound over all others
-   All `Widget` traits are impl'd by `derive(Widget)` on an opt-out basis

Widgets should now return a `TkAction` from methods modifying self and not
take a `Manager` reference, if possible (#79).

Widgets can now be downcast with `std::any::Any` (#97).

### Pop-ups and menus
Pop-up widgets are supported as a layer over an existing window (similar to a
borderless window, but without requiring windowing support, which winit lacks).
These are used for `ComboBox` and for (recursive) menus.
Added in #76, #85, #87, #93.

### Input behaviour
-   Add `EditGuard` allowing user hooks on `EditBox` updates (#68)
-   Add pan grabs — two-finger resize/rotate (#70)
-   Use standard events for timer and handle updates (#74)
-   Add reverse key navigation with Shift+Tab (#77)
-   Allow `Slider` to respond to arrow keys (#77)
-   Various small fixes (#73)
-   Add error and disabled states (#84, #85)
-   Merge `Action` and `Event` (#85)
-   Allow `ScrollRegion` (and `Mandlebrot`) to respond to navigation keys
    without focus (#90)
-   Let widgets request `Focus` when responding to events (#90)
-   Menu navigation with Tab key (#92) and arrows (#93, #97)
-   Allow accelerator keys to be derived from labels (e.g. "&File"),
    to use a separate layer for each pop-up,
    to be visually indicated when Alt is held,
    and to be activated with Alt held or `alt_bypass` set (#100)

### Graphics
-   Allow some wgpu state to be shared between windows (#69)
-   Tracks for sliders and scrollbars (#76)
-   Use floating-point types for mid-level draw API (#76)
-   Add a depth buffer (#94)
-   Fix: do not use `noperspective` in shaders (#99)

### Widgets
-   Add `DragHandle` component (#71)
-   Add `Slider` widget (#71)
-   Add `Frame` widget (#72)
-   Add `ComboBox` widget (#76, #85)
-   Add `Splitter` widget (#80)
-   Add `MenuBar`, `SubMenu` (#86)
-   Add `MenuEntry`, `MenuToggle` (#89)

### Sizing and layouts
-   Calculate window size before creation (#69)
-   Add (real) margins (#72, #79)
-   Replace `Horizontal` and `Vertical` with `Right`, `Left`, `Up`, `Down`,
    allowing reversed layouts and better pop-up placement (#78)
-   Do not pass `SizeHandle` to `Layout::set_rect` (#79)
-   Rewrite `SizeRules::solve_seq` to respect previous widths/heights and only modify minimally for validity (#79)
-   Reduce calls to `Layout::size_rules` (#79)

### Misc
-   Move `Vec2` to `kas::geom`; add `DVec2` (#70)
-   Add `CowString` and `CowStringL` as aliases over `std` types (#74)
-   Add `prelude` module (#75, #102)
-   Let `TkAction` support `Add<Output = Self>` and `Manager` support `AddAssign<TkAction>` (#79)
-   Update to wgpu 0.5, font-kit 0.6 and smallvec 1.4 (#94)
-   Support stable rustc, for a few examples only (#101)
-   New `unsize` feature flag (#101)
-   Revise documentation (#102)

## [0.3.1] — 2020-04-16
Bump version of `kas-wgpu` to fix build on docs.rs.

## [0.3.0] — 2020-02-24
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
-   Support custom draw pipes (#64, #65)
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
-   Widgets updates can now be triggered via an `UpdateHandle` (#46)
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
