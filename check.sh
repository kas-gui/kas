#!/bin/sh
# Check all examples not using view widgets:
cargo check --example async-event --example calculator --example clock --example counter --example cursors  --example hello --example splitter
# Except these:
# --example data-list --example stopwatch --example layout
cargo check --manifest-path examples/mandlebrot/Cargo.toml
