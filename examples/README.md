KAS Examples
==========

Examples are grouped by complexity.


Simplest examples
-----------------

### Hello

A simple custom message box.

![Hello](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/hello.png)

### Counter

(Almost) the simplest interactive example possible: a counter with push-buttons.

**Variant:** `sync-counter` opens two windows with a synchronised counter.

![Counter](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/counter.png)

### Cursors

Curious what each mouse cursor available on your desktop (via winit) looks like?

![Cursors](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/cursors.png)

### Layout

Demonstration of complex layout and multi-paragraph text.

![Layout](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/layout.png)

### Splitter

Demonstrates resizable panes.

![Splitter](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/splitter.png)


More complex examples
---------------------

### Calculator

A simple calculator show-casing the grid layout and keyboard support.

![Calculator](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/calculator.png)

### Stopwatch

Ready? Set! Go!

![Stopwatch](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/stopwatch.png)

### Sync-counter

A variant of [Counter](#Counter), demonstrating multiple windows and the
`SingleView` widget (the simplest shared data widget).

### Clock

A simple clock. An example of a custom widget using mid-level draw routines and
timer updates.

![Clock](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/clock.png)

### Times-tables

A simple demonstration of the `GridView` widget.

![Times-tables](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/times-tables.png)

### Data list

This example demonstrates an interface over a list data structure of
user-defined length. It has two implementations, both with (approximately) the
same UI, but different internals:

-   `data-list` directly allocates a widget for each data entry and stores data
    within the widgets; it can scale to hundreds of entries or potentially tens
    of thousands when using release optimisations and tolerating some delays
-   `data-list-view` uses a dynamic view over a lazily-allocated data structure;
    performance is thus independent of the number of entries (though length is
    still limited by the maximum possible scroll offset; see issue #222)

![Data list](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/data-list.png)

### Proxy

Demonstrates updating the UI in response to events from a background thread.

![Async-event](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/async-event.png)


Most-complex examples
---------------------

### Gallery

A testbed demoing most widgets, animations, data models, canvas and configuration.

![Gallery](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/gallery.png)
![Gallery](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/video/gallery.apng)

### Mandlebrot

GPU-accelerated fractals via a custom embedded WGPU graphics pipeline.

![Mandlebrot](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/mandlebrot.png)


Tools
-----

### Reformat Id

A [`kas::Id`](https://docs.rs/kas/latest/kas/struct.Id.html) is a path. This tool can decode both an `Id` displayed with a leading `#` (as below) and codes converted to a `u64` (where this is a compressed full path, not where this is a hash of a longer stack-allocated path).
```
$ cargo run --example reformat-id '#1001970'
[1, 0, 0, 1, 15, 0]
```


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
