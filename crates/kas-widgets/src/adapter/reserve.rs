// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size reservation

use kas::{event, prelude::*};
use std::fmt::{self, Debug};

/// Parameterisation of [`Reserve`] using a function pointer
///
/// Since it is impossible to name closures, using [`Reserve`] where a type is
/// required (e.g. in a struct field) is only possible by making the containing
/// struct generic over this field, which may be undesirable. As an alternative
/// a function pointer may be preferred.
pub type ReserveP<W> = Reserve<W, fn(&mut dyn SizeHandle, AxisInfo) -> SizeRules>;

/// A generic widget for size reservations
///
/// In a few cases it is desirable to reserve more space for a widget than
/// required for the current content, e.g. if a label's text may change. This
/// widget can be used for this by wrapping the base widget.
#[derive(Clone, Default, Widget)]
#[handler(msg = <W as Handler>::Msg)]
pub struct Reserve<W: Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> {
    #[widget_core]
    core: CoreData,
    #[widget]
    pub inner: W,
    reserve: R,
}

impl<W: Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> Reserve<W, R> {
    /// Construct a reserve
    ///
    /// The closure `reserve` should generate `SizeRules` on request, just like
    /// [`Layout::size_rules`]. This can be done by instantiating a temporary
    /// widget, for example:
    ///```
    /// use kas_widgets::adapter::Reserve;
    /// use kas_widgets::Label;
    /// use kas::prelude::*;
    ///
    /// let label = Reserve::new(Label::new("0"), |size_handle, axis| {
    ///     Label::new("00000").size_rules(size_handle, axis)
    /// });
    ///```
    /// Alternatively one may use virtual pixels:
    ///```
    /// use kas_widgets::adapter::Reserve;
    /// use kas_widgets::Filler;
    /// use kas::prelude::*;
    ///
    /// let label = Reserve::new(Filler::new(), |size_handle, axis| {
    ///     let size = i32::conv_ceil(size_handle.scale_factor() * 100.0);
    ///     SizeRules::fixed(size, (0, 0))
    /// });
    ///```
    /// The resulting `SizeRules` will be the max of those for the inner widget
    /// and the result of the `reserve` closure.
    #[inline]
    pub fn new(inner: W, reserve: R) -> Self {
        Reserve {
            core: Default::default(),
            inner,
            reserve,
        }
    }
}

impl<W: Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> Layout
    for Reserve<W, R>
{
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let inner_rules = self.inner.size_rules(size_handle, axis);
        let reserve_rules = (self.reserve)(size_handle, axis);
        inner_rules.max(reserve_rules)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.inner.set_rect(mgr, rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        self.inner.find_id(coord).or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        self.inner.draw(draw_handle, mgr, disabled);
    }
}

impl<W: HasBool + Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> HasBool
    for Reserve<W, R>
{
    fn get_bool(&self) -> bool {
        self.inner.get_bool()
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.inner.set_bool(state)
    }
}

impl<W: HasStr + Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> HasStr
    for Reserve<W, R>
{
    fn get_str(&self) -> &str {
        self.inner.get_str()
    }
}

impl<W: HasString + Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static>
    HasString for Reserve<W, R>
{
    fn set_string(&mut self, text: String) -> TkAction {
        self.inner.set_string(text)
    }
}

// TODO: HasFormatted

impl<W: SetAccel + Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> SetAccel
    for Reserve<W, R>
{
    fn set_accel_string(&mut self, accel: AccelString) -> TkAction {
        self.inner.set_accel_string(accel)
    }
}

impl<W: Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> Debug
    for Reserve<W, R>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Reserve")
            .field("core", &self.core)
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<W: Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> std::ops::Deref
    for Reserve<W, R>
{
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: Widget, R: FnMut(&mut dyn SizeHandle, AxisInfo) -> SizeRules + 'static> std::ops::DerefMut
    for Reserve<W, R>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
