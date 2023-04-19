#!/bin/sh
# Check all examples not using view widgets:
cargo check --example async-event --example calculator --example clock --example counter --example cursors --example data-list --example hello --example layout --example splitter --example stopwatch
cargo check --manifest-path examples/mandlebrot/Cargo.toml
