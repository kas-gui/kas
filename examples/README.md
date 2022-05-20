KAS Examples
==========

Examples are listed below in alphabetical order.

If learning KAS, the Hello, Counter, Calculator, and perhaps Filter-list
examples may be the most accessible. Also see
[the tutorials](https://kas-gui.github.io/tutorials/).

### Async event

Demonstrates updating the UI in response to events from a background thread.

![Async-event](https://github.com/kas-gui/data-dump/blob/master/screenshots/async-event.png)

### Calculator

A simple calculator show-casing the grid layout and keyboard support.

![Calculator](https://github.com/kas-gui/data-dump/blob/master/screenshots/calculator.png)

### Clock

A simple clock. An example of a custom widget using mid-level draw routines and
timer updates.

![Clock](https://github.com/kas-gui/data-dump/blob/master/screenshots/clock.png)

### Counter

(Almost) the simplest interactive example possible: a counter with push-buttons.

**Variant:** `sync-counter` opens two windows with a synchronised counter.

![Counter](https://github.com/kas-gui/data-dump/blob/master/screenshots/counter.png)

### Cursors

Curious what each mouse cursor available on your desktop (via winit) looks like?

![[Cursors](https://github.com/kas-gui/data-dump/blob/master/screenshots/cursors.png)

### Custom theme

An example demonstrating a custom theme implementation.
Pretty useless, but hey, funky background colours!

![Custom theme](https://github.com/kas-gui/data-dump/blob/master/screenshots/theme.png)

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

![Data list](https://github.com/kas-gui/data-dump/blob/master/screenshots/data-list.png)

### Gallery

A testbed demoing most widgets along with theme and colour switching.

Note: the video demo below is quite dated.

![Gallery](https://github.com/kas-gui/data-dump/blob/master/screenshots/gallery.png)
![Gallery](https://github.com/kas-gui/data-dump/blob/master/video/gallery.png)

### Hello

A message box.

![Hello](https://github.com/kas-gui/data-dump/blob/master/screenshots/hello.png)

### Layout

Demonstration of complex layout and multi-paragraph text.

![Layout](https://github.com/kas-gui/data-dump/blob/master/screenshots/layout.png)

### Mandlebrot

GPU-accelerated fractals via a custom embedded WGPU graphics pipeline.

![Mandlebrot](https://github.com/kas-gui/data-dump/blob/master/screenshots/mandlebrot.png)

### Splitter

Demonstrates resizable panes.

![Splitter](https://github.com/kas-gui/data-dump/blob/master/screenshots/splitter.gif)

### Stopwatch

Ready? Set! Go!

![Stopwatch](https://github.com/kas-gui/data-dump/blob/master/screenshots/stopwatch.png)

### Sync-counter

A variant of [Counter](#Counter), demonstrating multiple windows and the
`SingleView` widget (the simplest shared data widget).

### Times-tables

A simple demonstration of the `MatrixView` widget.


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
