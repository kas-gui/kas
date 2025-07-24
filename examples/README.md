KAS Examples
==========

Examples are listed below in alphabetical order.

If learning KAS, the Hello, Counter, Calculator, and perhaps Filter-list
examples may be the most accessible. Also see
[the tutorials](https://kas-gui.github.io/tutorials/).

### Async event

Demonstrates updating the UI in response to events from a background thread.

![Async-event](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/async-event.png)

### Calculator

A simple calculator show-casing the grid layout and keyboard support.

![Calculator](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/calculator.png)

### Clock

A simple clock. An example of a custom widget using mid-level draw routines and
timer updates.

![Clock](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/clock.png)

### Counter

(Almost) the simplest interactive example possible: a counter with push-buttons.

**Variant:** `sync-counter` opens two windows with a synchronised counter.

![Counter](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/counter.png)

### Cursors

Curious what each mouse cursor available on your desktop (via winit) looks like?

![Cursors](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/cursors.png)

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

### Gallery

A testbed demoing most widgets, animations, data models, canvas and configuration.

![Gallery](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/gallery.png)
![Gallery](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/video/gallery.apng)

### Hello

A message box.

![Hello](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/hello.png)

### Layout

Demonstration of complex layout and multi-paragraph text.

![Layout](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/layout.png)

### Mandlebrot

GPU-accelerated fractals via a custom embedded WGPU graphics pipeline.

![Mandlebrot](https://github.com/kas-gui/data-dump/blob/master/kas_0_9/image/mandlebrot.png)

### Splitter

Demonstrates resizable panes.

![Splitter](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/splitter.png)

### Stopwatch

Ready? Set! Go!

![Stopwatch](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/stopwatch.png)

### Sync-counter

A variant of [Counter](#Counter), demonstrating multiple windows and the
`SingleView` widget (the simplest shared data widget).

### Times-tables

A simple demonstration of the `GridView` widget.

![Times-tables](https://github.com/kas-gui/data-dump/blob/master/kas_0_11/image/times-tables.png)


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
