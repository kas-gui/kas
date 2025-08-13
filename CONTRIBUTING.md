Contributing
========

Contributions to KAS are welcome. It will be assumed that all
contributions are made available under the terms of the Apache License v2.0.

For very small changes, you may simply open a pull request; in other cases it
is recommended to open an issue first. This project does not currently make use
of third-party message boards.


Code style
---------

Code style is more art than science, but should:

1.  Use `cargo fmt`
2.  Make code clear and readable
3.  Aim to reduce the line count, when this doesn't conflict with the above

Sometimes introducing an extra `let` binding helps. Sometimes it makes sense
to wrap a `match`. Sometimes it makes sense to use `return`.

### Spelling

As is industry standard, APIs should use US-English spellings.
This rule is not enforced for documentation or local variables,
so long readability is not significantly impaired.

Notes:

-   'Config' is used as an abbreviation for 'configuration' (noun), *not* for 'configure' (verb)

### Unsafe

Usage of `unsafe` is allowed (and required in several cases, e.g. C APIs and GPU communication) but requires additional justification and review. Read [The Rustonomicon] and avoid [Undefined Behaviour].

Note that the `unsafe_node` feature is dependent on unspecified behaviour but (as far as I am aware) not [Undefined Behaviour] (which has a distinct meaning). For this reason the feature is not enabled by default.

[The Rustonomicon]: https://doc.rust-lang.org/nomicon/index.html
[Undefined Behaviour]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
