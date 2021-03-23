KAS Examples
==========

Exmaples demonstrate various features of KAS.

Note that screenshots may be outdated since themes, draw routines and layout code has been updated quite frequently.

### Gallery

A testbed demoing most widgets along with theme and colour switching.

![Gallery](../../screenshots/gallery.png)
![Gallery](https://github.com/kas-gui/data-dump/blob/master/video/gallery.png)

### Counter

(Almost) the simplest interactive example possible: a counter with push-buttons.

**Variant:** `sync-counter` opens two windows with a synchronised counter.

![Counter](../../screenshots/counter.png)

### Calculator

A simple calculator, with (limited) keyboard support.

![Calculator](../../screenshots/calculator.png)

### Clock

A simple clock. An example of a custom widget using lower-level draw routines.

![Clock](../../screenshots/clock.png)

### Splitter

Demonstrates resizable panes.

![Splitter](../../screenshots/splitter.gif)

### Hello

A message box.

![Hello](../../screenshots/hello.png)

### Stopwatch

Ready? Set! Go!

![Stopwatch](../../screenshots/stopwatch.png)

### Data list

This example demonstrates an interface over a list data structure of
user-defined length. It has two implementations, both with (approximately) the
same UI, but different internals:

-   `data-list` directly allocates a widget for each data entry and stores data
    within the widgets; it can scale to hundreds of entries or potentially tens
    of thousands when using release optimisations and tolerating some delays
-   `data-list-view` uses a dynamic view over a lazily-allocated data structure;
    performance is thus independent of the number of entries, but the size type
    used to calculate the maximum scroll offset overflows with a few tens of
    millions of entries (depending on item widget and scale factor)

![Data list](../../screenshots/data-list.png)

### Layout

Demonstration of complex layout setting. Resizing is a little slow due to the
text layout engine (may be improved in the future).

![Layout](../../screenshots/layout.png)

### Markdown

Markdown parsing and rich-text rendering. This is a demonstrator for
[KAS-text](https://github.com/kas-gui/kas-text/)'s rich text support, using
Markdown as an input language. Note that full support for Markdown requires a
considerably more complex text layout engine as well as an HTML parser and
potentially even CSS support; this is beyond the (immediate) goals of this feature.

![Markdown](../../screenshots/markdown.png)

### Custom theme

An example demonstrating a custom theme implementation.
Pretty useless, but hey, funky background colours!

![Custom theme](../../screenshots/theme.png)

### Mandlebrot

An example demonstrating a custom draw pipe.

![Mandlebrot](../../screenshots/mandlebrot.png)

### Async event

Demonstrates how to send data from another thread.

### Cursors

Gallery of available mouse cursors.


Copyright and Licence
-------

The <COPYRIGHT> file includes a list of contributors who claim copyright on this
project. This list may be incomplete; new contributors may optionally add
themselves to this list.

The KAS library is published under the terms of the Apache License, Version 2.0.
You may obtain a copy of this licence from the <LICENSE-APACHE> file or on
the following webpage: <https://www.apache.org/licenses/LICENSE-2.0>
