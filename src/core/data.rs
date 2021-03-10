// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget data types

use std::convert::TryFrom;
use std::fmt;
use std::num::NonZeroU32;

#[allow(unused)]
use super::Layout;
use super::Widget;
use crate::event::{self, Manager};
use crate::geom::Rect;
use crate::{dir::Direction, layout, WindowId};

/// Widget identifier
///
/// All widgets are assigned an identifier which is unique within the window.
/// This type may be tested for equality and order.
///
/// This type is small and cheap to copy. Internally it is "NonZero", thus
/// `Option<WidgetId>` is a free extension (requires no extra memory).
///
/// Identifiers are assigned when configured and when re-configured
/// (via [`kas::TkAction::RECONFIGURE`]). Since user-code is not notified of a
/// re-configure, user-code should not store a `WidgetId`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetId(NonZeroU32);

impl WidgetId {
    pub(crate) const FIRST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU32::new(self.0.get() + 1).unwrap())
    }
}

impl TryFrom<u32> for WidgetId {
    type Error = ();
    fn try_from(x: u32) -> Result<WidgetId, ()> {
        NonZeroU32::new(x).map(|n| WidgetId(n)).ok_or(())
    }
}

impl TryFrom<u64> for WidgetId {
    type Error = ();
    fn try_from(x: u64) -> Result<WidgetId, ()> {
        if let Ok(x) = u32::try_from(x) {
            if let Some(nz) = NonZeroU32::new(x) {
                return Ok(WidgetId(nz));
            }
        }
        Err(())
    }
}

impl From<WidgetId> for u32 {
    #[inline]
    fn from(id: WidgetId) -> u32 {
        id.0.get()
    }
}

impl From<WidgetId> for u64 {
    #[inline]
    fn from(id: WidgetId) -> u64 {
        id.0.get().into()
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        WidgetId::LAST
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "#{}", self.0)
    }
}

#[test]
fn size_of_option_widget_id() {
    use std::mem::size_of;
    assert_eq!(size_of::<WidgetId>(), size_of::<Option<WidgetId>>());
}

/// Common widget data
///
/// All widgets should embed a `#[widget_core] core: CoreData` field.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
    pub disabled: bool,
}

/// Trait to describe the type needed by the layout implementation.
///
/// The (non-trivial) [`layout`] engines require a storage field within their
/// widget. For manual [`Layout`] implementations this may be specified
/// directly, but to allow the `derive(Widget)` macro to specify the appropriate
/// data type, a widget should include a field of the following form:
/// ```none
/// #[layout_data] layout_data: <Self as kas::LayoutData>::Data,
/// ```
///
/// Ideally we would use an inherent associated type on the struct in question,
/// but until rust-lang#8995 is implemented that is not possible. We also cannot
/// place this associated type on the [`Widget`] trait itself, since then uses
/// of the trait would require parameterisation. Thus, this trait.
pub trait LayoutData {
    type Data: Clone + fmt::Debug + Default;
    type Solver: layout::RulesSolver;
    type Setter: layout::RulesSetter;
}

/// A widget which escapes its parent's rect
///
/// A pop-up is a special widget drawn either as a layer over the existing
/// window or in a new borderless window. It should be precisely positioned
/// *next to* it's `parent`'s `rect`, in the specified `direction` (or, if not
/// possible, in the opposite direction).
///
/// A pop-up is in some ways an ordinary child widget and in some ways not.
/// The pop-up widget should be a permanent child of its parent, but is not
/// visible until [`Manager::add_popup`] is called.
///
/// A pop-up widget's rect is not contained by its parent, therefore the parent
/// must not call any [`Layout`] methods on the pop-up (whether or not it is
/// visible). The window is responsible for calling these methods.
///
/// Other methods on the pop-up, including event handlers, should be called
/// normally, with one exception: after calling an event handler on the pop-up,
/// the parent should invoke [`Manager::pop_action`] and handle the action
/// itself, where possible (using [`Manager::close_window`] to close it).
/// Remaining actions should be added back to the [`Manager`].
//
// NOTE: it's tempting to include a pointer to the widget here. There are two
// options: (a) an unsafe aliased pointer or (b) Rc<RefCell<dyn WidgetConfig>>.
// Option (a) should work but is an unnecessary performance hack; (b) could in
// theory work but requires adjusting WidgetChildren::get, find etc. to take a
// closure instead of returning a reference, causing *significant* complication.
#[derive(Clone, Debug)]
pub struct Popup {
    pub id: WidgetId,
    pub parent: WidgetId,
    pub direction: Direction,
}

/// Functionality required by a window
pub trait Window: Widget<Msg = event::VoidMsg> {
    /// Get the window title
    fn title(&self) -> &str;

    /// Whether to limit the maximum size of a window
    ///
    /// All widgets' size rules allow calculation of two sizes: the minimum
    /// size and the ideal size. Windows are initially sized to the ideal size.
    /// This option controls whether the window size is restricted by the
    /// calculated minimum size and by the ideal size.
    ///
    /// Return value is `(restrict_min, restrict_max)`. Suggested is to use
    /// `(true, true)` for simple dialog boxes and `(true, false)` for complex
    /// windows.
    fn restrict_dimensions(&self) -> (bool, bool);

    /// Add a pop-up as a layer in the current window
    ///
    /// Each [`Popup`] is assigned a [`WindowId`]; both are passed.
    fn add_popup(&mut self, mgr: &mut Manager, id: WindowId, popup: Popup);

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    fn resize_popups(&mut self, mgr: &mut Manager);

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    fn remove_popup(&mut self, mgr: &mut Manager, id: WindowId);

    /// Handle closure of self
    ///
    /// This allows for actions on destruction, but doesn't need to do anything.
    fn handle_closure(&mut self, _mgr: &mut Manager) {}
}
