// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utilities

use crate::geom::Coord;
use crate::{ChildIndices, Id, Tile, TileExt};
use std::{error::Error, fmt, path::Path};

enum IdentifyContents<'a> {
    Simple(&'a Id),
    Wrapping(&'a dyn Tile),
}

/// Helper to display widget identification (e.g. `MyWidget#01`)
///
/// Constructed by [`crate::Tile::identify`].
pub struct IdentifyWidget<'a>(&'a str, IdentifyContents<'a>);
impl<'a> IdentifyWidget<'a> {
    /// Construct for a simple widget
    pub fn simple(name: &'a str, id: &'a Id) -> Self {
        IdentifyWidget(name, IdentifyContents::Simple(id))
    }

    /// Construct for a wrapping widget
    pub fn wrapping(name: &'a str, inner: &'a dyn Tile) -> Self {
        IdentifyWidget(name, IdentifyContents::Wrapping(inner))
    }
}
impl<'a> fmt::Display for IdentifyWidget<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.1 {
            IdentifyContents::Simple(id) => write!(f, "{}{}", self.0, id),
            IdentifyContents::Wrapping(inner) => write!(f, "{}<{}>", self.0, inner.identify()),
        }
    }
}

struct Trail<'a> {
    parent: Option<&'a Trail<'a>>,
    trail: &'static str,
}
impl<'a> fmt::Display for Trail<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(p) = self.parent {
            p.fmt(f)?;
        }
        write!(f, "{}", self.trail)
    }
}

/// Helper to print widget heirarchy
///
/// Note: output starts with a new line.
pub struct WidgetHierarchy<'a> {
    widget: &'a dyn Tile,
    filter: Option<Id>,
    trail: Trail<'a>,
    indent: usize,
    have_next_sibling: bool,
}
impl<'a> WidgetHierarchy<'a> {
    pub fn new(widget: &'a dyn Tile, filter: Option<Id>) -> Self {
        WidgetHierarchy {
            widget,
            filter,
            trail: Trail {
                parent: None,
                trail: "",
            },
            indent: 0,
            have_next_sibling: false,
        }
    }
}
impl<'a> fmt::Display for WidgetHierarchy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let len = 51 - 2 * self.indent;
        let trail = &self.trail;
        let (hook, trail_hook) = match self.indent >= 1 {
            false => ("", ""),
            true if self.have_next_sibling => ("├ ", "│ "),
            true => ("└ ", "  "),
        };
        // Note: pre-format some items to ensure correct alignment
        let identify = format!("{}", self.widget.identify());
        let r = self.widget.rect();
        let Coord(x1, y1) = r.pos;
        let Coord(x2, y2) = r.pos + r.size;
        let xr = format!("x={x1}..{x2}");
        let xrlen = xr.len().max(12);
        write!(
            f,
            "\n{trail}{hook}{identify:<len$} {xr:<xrlen$} y={y1}..{y2}"
        )?;

        let indent = self.indent + 1;

        if let Some(id) = self.filter.as_ref()
            && let Some(index) = self.widget.find_child_index(id)
            && let Some(widget) = self.widget.get_child(index)
        {
            return write!(f, "{}", WidgetHierarchy {
                widget,
                filter: self.filter.clone(),
                trail: Trail {
                    parent: Some(trail),
                    trail: trail_hook,
                },
                indent,
                have_next_sibling: false,
            });
        }

        let mut iter = self.widget.children();
        let mut next = iter.next();
        while let Some(widget) = next {
            next = iter.next();

            if !widget.id_ref().is_valid() {
                continue;
            }

            write!(f, "{}", WidgetHierarchy {
                widget,
                filter: None,
                trail: Trail {
                    parent: Some(trail),
                    trail: trail_hook,
                },
                indent,
                have_next_sibling: next.is_some(),
            })?;
        }
        Ok(())
    }
}

/// Format for types supporting Debug
///
/// This requires the "spec" feature and nightly rustc to be useful.
pub struct TryFormat<'a, T: ?Sized>(pub &'a T);

#[cfg(not(feature = "spec"))]
impl<'a, T: ?Sized> fmt::Debug for TryFormat<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{}}}", std::any::type_name::<T>())
    }
}

#[cfg(feature = "spec")]
impl<'a, T: ?Sized> fmt::Debug for TryFormat<'a, T> {
    default fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{}}}", std::any::type_name::<T>())
    }
}

#[cfg(feature = "spec")]
impl<'a, T: fmt::Debug + ?Sized> fmt::Debug for TryFormat<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Generic implementation of [`Tile::nav_next`]
pub fn nav_next(reverse: bool, from: Option<usize>, indices: ChildIndices) -> Option<usize> {
    let range = indices.as_range();
    if range.is_empty() {
        return None;
    }
    let (first, last) = (range.start, range.end - 1);

    if let Some(index) = from {
        match reverse {
            false if index < last => Some(index + 1),
            true if first < index => Some(index - 1),
            _ => None,
        }
    } else {
        match reverse {
            false => Some(first),
            true => Some(last),
        }
    }
}

/// Log a warning regarding an error message
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub fn warn_about_error(msg: &str, mut error: &dyn Error) {
    log::warn!("{msg}: {error}");
    while let Some(source) = error.source() {
        log::warn!("Source: {source}");
        error = source;
    }
}

/// Log a warning regarding an error message with a path
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub fn warn_about_error_with_path(msg: &str, error: &dyn Error, path: &Path) {
    warn_about_error(msg, error);
    log::warn!("Path: {}", path.display());
}
