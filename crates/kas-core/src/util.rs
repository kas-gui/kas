// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utilities

use crate::geom::Coord;
use crate::{Node, WidgetId};
use std::cell::RefCell;
use std::fmt;

/// Helper to display widget identification (e.g. `MyWidget#01`)
///
/// Constructed by [`crate::WidgetExt::identify`].
pub struct IdentifyWidget(pub(crate) &'static str, pub(crate) WidgetId);
impl fmt::Display for IdentifyWidget {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}{}", self.0, self.1)
    }
}

/// Helper to print widget heirarchy
///
/// Note: output starts with a new line.
pub struct WidgetHierarchy<'a> {
    // We use RefCell since Display takes &self but Node API requires &mut self.
    node: RefCell<Node<'a>>,
    indent: usize,
}
impl<'a> WidgetHierarchy<'a> {
    pub fn new(node: Node<'a>) -> Self {
        WidgetHierarchy {
            node: RefCell::new(node),
            indent: 0,
        }
    }
}
impl<'a> fmt::Display for WidgetHierarchy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let len = 43 - 2 * self.indent;
        let trail = "| ".repeat(self.indent);
        let mut widget = self.node.borrow_mut();
        // Note: pre-format some items to ensure correct alignment
        let identify = format!("{}", widget.identify());
        let r = widget.rect();
        let Coord(x1, y1) = r.pos;
        let Coord(x2, y2) = r.pos + r.size;
        let xr = format!("x={x1}..{x2}");
        let xrlen = xr.len().max(12);
        write!(f, "\n{trail}{identify:<len$} {xr:<xrlen$} y={y1}..{y2}")?;

        let indent = self.indent + 1;
        widget.for_children_try(|node| {
            write!(f, "{}", WidgetHierarchy {
                node: RefCell::new(node),
                indent
            })
        })
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

/// Generic implementation of [`crate::Widget::nav_next`]
pub fn nav_next(reverse: bool, from: Option<usize>, len: usize) -> Option<usize> {
    let last = len.wrapping_sub(1);
    if last == usize::MAX {
        return None;
    }

    if let Some(index) = from {
        match reverse {
            false if index < last => Some(index + 1),
            true if 0 < index => Some(index - 1),
            _ => None,
        }
    } else {
        match reverse {
            false => Some(0),
            true => Some(last),
        }
    }
}

/// Load a window icon from a path
#[cfg(feature = "image")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "image")))]
pub fn load_icon_from_path<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<Icon, Box<dyn std::error::Error>> {
    // TODO(opt): image loading could be de-duplicated with
    // DrawShared::image_from_path, but this may not be worthwhile.
    let im = image::io::Reader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgba8();
    let (w, h) = im.dimensions();
    Ok(Icon::from_rgba(im.into_vec(), w, h)?)
}

/// Log a warning regarding an error message
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub fn warn_about_error(msg: &str, mut error: &dyn std::error::Error) {
    log::warn!("{msg}: {error}");
    while let Some(source) = error.source() {
        log::warn!("Source: {source}");
        error = source;
    }
}
