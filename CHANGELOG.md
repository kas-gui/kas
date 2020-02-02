# Changelog
The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2020-02-02


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
