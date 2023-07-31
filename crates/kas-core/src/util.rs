// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utilities

use crate::geom::Coord;
use crate::{Layout, LayoutExt, WidgetId};
use std::fmt;

/// Helper to display widget identification (e.g. `MyWidget#01`)
///
/// Constructed by [`crate::LayoutExt::identify`].
pub struct IdentifyWidget<'a>(pub(crate) &'static str, pub(crate) &'a WidgetId);
impl<'a> fmt::Display for IdentifyWidget<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}{}", self.0, self.1)
    }
}

/// Helper to print widget heirarchy
///
/// Note: output starts with a new line.
pub struct WidgetHierarchy<'a> {
    widget: &'a dyn Layout,
    indent: usize,
}
impl<'a> WidgetHierarchy<'a> {
    pub fn new(widget: &'a dyn Layout) -> Self {
        WidgetHierarchy { widget, indent: 0 }
    }
}
impl<'a> fmt::Display for WidgetHierarchy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let len = 43 - 2 * self.indent;
        let trail = "| ".repeat(self.indent);
        // Note: pre-format some items to ensure correct alignment
        let identify = format!("{}", self.widget.identify());
        let r = self.widget.rect();
        let Coord(x1, y1) = r.pos;
        let Coord(x2, y2) = r.pos + r.size;
        let xr = format!("x={x1}..{x2}");
        let xrlen = xr.len().max(12);
        write!(f, "\n{trail}{identify:<len$} {xr:<xrlen$} y={y1}..{y2}")?;

        let indent = self.indent + 1;
        self.widget
            .for_children_try(|w| write!(f, "{}", WidgetHierarchy { widget: w, indent }))?;
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

/// Generic implementation of [`Layout::nav_next`](crate::Layout::nav_next)
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
