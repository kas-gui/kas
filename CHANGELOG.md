# Changelog
The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0] — 2021-06-17

This release sees basic support for images, improved text rendering,
replacements for all non-Rust dependencies, and theme configuration support.

### Themes and text style/layout

-   Support theme configuration (#196, #198)
-   Support write-on-exit for updated configuration files (#196, #197)
-   Revise colour types: replace `Colour` with `Rgba` and `Rgba8Srgb` (#198)

### Text layout and configuration

-   Use rustybuzz for text shaping, enabled by default (#199)
-   Allow custom font aliases in config (#199)
-   Allow assigning a custom font per text class; new `TextClass::MenuLabel`,
    use (by default) serif fonts for edit fields (#200)
-   Vertically-align text lines (#201)
-   Configuration for text rastering (#201)

### Images and (text) rendering

-   Support raster images loaded from the file-system (#185)
-   Use dynamic image atlases (#186) created on demand (#187)
-   Replace `wgpu_glyph` and `glyph_brush` with our own glyph caching (#190)
-   Add transparent shadows to pop-up layers (#194)
-   Support `fontdue` for glyph rastering (#201)
-   Move glyph raster code to KAS-text (#202)

Details:

-   Use push constants (#184)
-   Remove depth-buffer (#185)
-   Use common bind group for graphics pipes (#187)
-   Use a staging belt for vertex uploads (#189)
-   Significant code revision (#184 - #190)
-   Update `wgpu-rs` to 0.8 (#193)

### Other

-   Fix (or work-around) hang-on-exit with multiple windows open (#184)
-   Better scrolling for widgets not receiving the initial event (#192)
-   Avoid unnecessary redraws on mouse-movement while a mouse-grab is in
    effect (#196)

## [0.7.1] — 2021-05-03

-   Fix for spans in grids (landed in master in #184)

## [0.7.0] — 2021-04-03

The major focus of this release is "view widgets": the ability to construct a
synchronised view over shared data.

The new view widgets allow completion of the "CRUD" (Create, Read, Update,
Delete) [7GUIs challenge app](https://github.com/kas-gui/7guis/). The use of
view widgets allows data-UI separation and scaling to larger data sets (mainly
limited by the `O(n)` filter in this example). Work also started on the Cells
challenge (mini-spreadsheet), though many details of the app and of the
`MatrixView` widget still need addressing, especially drawing, multi-cell
selection and keyboard handling.

Additionally, this version saw development of a new mini-library, `kas::conv`,
which spun off into its own lib [easy-cast](https://crates.io/crates/easy-cast).

### Configuration

-   Use platform-specific shortcuts, especially for MacOS (#154)
-   Add support for reading and writing config files, controlled by env vars (#155)
-   Additional configuration items (#171):

    -   menu-open delay
    -   text selection via touch delay
    -   distance moved before touch events pan text
    -   when mouse click-and-drag can pan text and other objects

### Conv mini-lib

-   Add `kas::conv::Conv` trait and use to replace various casts (#152)
-   Add `Cast` and `CastFloat` traits (#154)
-   Switch to `easy-cast` library and export as `kas::cast` (#182)

### Examples

-   Add `filter-list` (#146, #148, #149)
-   Simplify `sync-counter` via use of a view widget (#146)
-   Add `async-event` demonstrating "computation" in a background thread (#169)
-   Add `cursors` showing a gallery of mouse cursors (#169)
-   Rename `dynamic` to `data-list` (#176); add `data-list-view`, a scalable
    (indirect data) variant (#170)

### Widgets

-   Add a progress bar widget (#143)
-   Replace `EditBoxVoid = EditBox<EditVoid>` with `EditBox = EditBox<()>` (#146)
-   Frame widgets now implement `Deref` and `DerefMut` (#147)
-   Add `ScrollBars` wrapper and `Scrollable` trait (#149, #170)
-   Rename `WidgetChildren` methods to be more distinctive (#157, #169)
-   Separate `EditBox` into `EditField` with frame wrapper `EditBox` (#158)
-   Move core widget traits into `kas::core` private module (#172)
-   Add `NavFrame` wrapper widget (#174)
-   Add `WidgetExt` extension trait over any `Widget` (#175)
-   Add `kas::adapter` module (#175)
-   Add `kas::adapter::WithLabel` and `WidgetExt::with_label` (#182)
-   Add `draw` optional parameter to `layout` attribute of `derive(Widget)` macro (#182)

#### View widgets

-   Add `Accessor`, `SharedRc` as an abstraction over shared data (#146);
    revise into `SingleData`, `ListData` (#164); add `update` method (#170);
    more revision, adding `SharedData` and `SharedDataRec` (#172);
    add `UpdatableHandler` (#176)
-   Add ~~`FilterAccessor`~~ `FilteredList` (#148, #157, #164, #70))
-   Add `ViewWidget` trait and `DefaultView` for view widgets (#146);
    revise into `View` with associated `Widget `type (#170);
    rename `View` to `Driver` and add `DefaultNav` (#174);
    simplify and add `Driver::Msg` (#176)
-   Add `ListView` widget (#146-148, #162-163)
-   Add `SingleView` widget (#146)
-   Support item selection in `ListView` (#162, #163, #167)
-   Support updating a data set through a view widget (#170)
-   Support recursive updates of datasets (allowing `FilteredList` to update
    when the underlying dataset updates) (#172)
-   Add `MatrixView` (#175, #176)

### Event handling

-   Add `EditGuard::update` and pass `&mut Manager` to guards (#148)
-   Revise handlers: `TextButton`, `CheckBox`, `RadioBox` now all optional
    closures to generate an optional message (#160)
-   Reduce redraws by explicitly specifying which widgets highlight on hover (#167)
-   Remove payload from `Response::Unhandled` (#170)
-   Add `u64` payload to `Event::TimerUpdate` (#171)
-   Revise `ComboBox` handler (#171)
-   Add `Response::Select` (#174)
-   Add `WidgetExt::map_msg`, `map_msg_discard` and `map_response` for
    convenient adjustment of widget messages (#175)
-   Support arrow-key and tab navigation for `ListView` and `MatrixView` (#177)
-   Replace `Layout::spatial_range` with `spatial_nav`, allowing correct child
    navigation order for `ListView` and `MatrixView` (#177)

### Layout

-   Add ~~`size_handle`~~ `manager` parameter to `set_rect` (#147, #150)
-   Generic support for space reservations (#147, #175)
-   Add `FrameRules` type for consistent/simpler frame sizing; use in
    `SizeHandle` (#161)
-   New `text_margin` property for themes (#161)
-   Scroll regions have a drawable margin (#162)
-   Rename `StretchPolicy` to `Stretch` and revise entries (#173)
-   Add `SizeHandle::pixels_from_virtual`, `pixels_from_points`,
    `pixels_from_em` convenience methods (#175)

### Geometry types

-   Impl `Sub<Size> for Coord` and `Mul<i32> for Coord` (#148)
-   Use `i32` instead of `u32` for `Size`; add `Offset` (#152)
-   Add `set_component` method to `Size`, `Coord` and `Offset` (#162)

### Misc

-   Terminology: replace 'toolkit' with 'shell' (#144)
-   Add `ScrollComponent` abstraction (#148)
-   Make `TkAction` into a bitflags struct (from an enum) (#150)
-   Make grab handles use the "grab" cursor icon (#151)
-   Direction types `Left, Right, Down, Up` moved into new `kas::dir` module (#154)
-   Push `derive(Widget)` before its attributes (#164)
-   Move `ThemeAction` and `ThemeApi` into `kas::draw` (#172)
-   Allow calling `set_text_and_prepare`, `set_string_and_prepare` before UI is
    initialised (#176)
-   Update README and ROADMAP (#171, #174)
-   Update MSRV to 1.52.0 (currently beta) (#179)
-   Update dependencies: `winit`, `wgpu`, `wgpu_glyph`, `env_logger` (#156)
-   Update dependencies: `kas-text`, `smallvec`, `bitflags`, `bytemuck` (#180)
-   Replace `clipboard` dependency with `window_clipboard` (#180)
-   Use const generics to clean up layout solver storage types (#182)

## [0.6.0] — 2020-11-24

This release covers significant revisions to the KAS-text API along with initial
support for rich text. It also includes several fixes and quality-of-life
improvements.

### Text

-   Updates for new KAS-text API (#126, #131-133)
-   Fix drawing of edit cursor in right-hand margin (#122)
-   Add `markdown` example (#125, #139)
-   Remove `LabelString`; make `AccelString` implement
    `kas-text::parser::Parser` (#126)
-   Support underline and strikethrough (#129, #133)
-   New `SelectionHelper` struct to abstract over text-selection logic (#137)

### Graphics

-   Update to wgpu v0.6 (#121)
-   Use pre-compiled GLSL shaders; add simple opt-in build system. This removes
    the depencency on `shaderc`. (#124)
-   Fix shader precision (#128)
-   Add draw performance metrics (#131)

### Layout

Improve space allocation for grid spans (#134)
Fix size of slider handle when slider is over-sized (#134)
Remove the unwanted right-hand margin in the Mandlebrot example (#134)

### Window and event management

-   Update winit to 0.23 (#129)
-   Add `Window::on_drop` method returning a future which can be used to
    retrieve state from a dialog window on closure. This replaces window
    callbacks. (#130)
-   New `LostSelFocus` event to delay focus-clearing until another widget
    supporting selection receives focus (#137)
-   Use `LinearMap` for cleaner code in the event manager (#140)
-   Fix: avoid clearing `time_updates`, `handle_updates` and `pending` fields
    of event manager during reconfigure (#140)

### Misc

-   Remove broken `CloneTo` helper trait (#126)
-   Add `min_spec` feature for optional usage of `min_specialization`. (#132)
-   Let parent widgets store identifier range for descendents, allowing O(1)
    `is_ancestor_of` check. (#132)
-   Use binary-search in `WidgetChildren::find` and `find_mut` for `O(log n)`
    performance. (#133)
-   Add builder-style methods to `Theme` and `Toolkit` (#139)
-   Switch to US-English spellings in API (#141)

## [0.5.0] — 2020-08-14
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
-   For several methods like `set_text`, replace `w.set_text(mgr, text)` with `*mgr |= w.set_text(text)`

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
