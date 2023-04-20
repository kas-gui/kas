#!/bin/sh
# Check all examples not using view widgets except these:
# --example data-list --example stopwatch --example layout
cargo check --example async-event --example calculator --example clock --example counter --example cursors  --example hello --example splitter \
&& cargo check --manifest-path examples/mandlebrot/Cargo.toml
