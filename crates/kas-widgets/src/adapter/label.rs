// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper adding a label

use kas::draw::TextClass;
use kas::layout::{RulesSetter, RulesSolver};
use kas::text::util::set_text_and_prepare;
use kas::{event, layout, prelude::*};

widget! {
    /// A wrapper widget with a label
    #[derive(Clone, Default, Debug)]
    #[handler(msg = W::Msg)]
    pub struct WithLabel<W: Widget, D: Directional> {
        #[widget_core]
        core: CoreData,
        layout_data: layout::FixedRowStorage<2>,
        dir: D,
        #[widget]
        inner: W,
        label_pos: Coord,
        label: Text<AccelString>,
    }
}

impl<W: Widget, D: Directional + Default> WithLabel<W, D> {
    /// Construct from `inner` widget and `label`
    #[inline]
    pub fn new<T: Into<AccelString>>(inner: W, label: T) -> Self {
        Self::new_with_direction(D::default(), inner, label)
    }
}

impl<W: Widget, D: Directional> WithLabel<W, D> {
    /// Construct from `direction`, `inner` widget and `label`
    #[inline]
    pub fn new_with_direction<T: Into<AccelString>>(direction: D, inner: W, label: T) -> Self {
        WithLabel {
            core: Default::default(),
            layout_data: Default::default(),
            dir: direction,
            inner,
            label_pos: Default::default(),
            label: Text::new_multi(label.into()),
        }
    }

    /// Set text in an existing `Label`
    ///
    /// Note: this must not be called before fonts have been initialised
    /// (usually done by the theme when the main loop starts).
    pub fn set_text<T: Into<AccelString>>(&mut self, text: T) -> TkAction {
        set_text_and_prepare(&mut self.label, text.into(), self.core.rect.size)
    }

    /// Get the accelerator keys
    pub fn keys(&self) -> &[event::VirtualKeyCode] {
        self.label.text().keys()
    }
}

impl<W: Widget, D: Directional> Layout for WithLabel<W, D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut solver = layout::RowSolver::new(axis, (self.dir, 2), &mut self.layout_data);
        let child = &mut self.inner;
        solver.for_child(&mut self.layout_data, 0usize, |axis| {
            child.size_rules(size_handle, axis)
        });
        let label = &mut self.label;
        solver.for_child(&mut self.layout_data, 1usize, |axis| {
            size_handle.text_bound(label, TextClass::Label, axis)
        });
        solver.finish(&mut self.layout_data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let mut setter = layout::RowSetter::<_, [i32; 2], _>::new(
            rect,
            (self.dir, 2),
            align,
            &mut self.layout_data,
        );
        let rect = setter.child_rect(&mut self.layout_data, 0);
        self.inner.set_rect(mgr, rect, align);
        let rect = setter.child_rect(&mut self.layout_data, 1);
        self.label_pos = rect.pos;
        self.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Default, Align::Centre));
        });
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        self.inner.find_id(coord).or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        self.inner.draw(draw_handle, mgr, disabled);
        let accel = mgr.show_accel_labels();
        let state = self.input_state(mgr, disabled);
        draw_handle.text_accel(self.label_pos, &self.label, accel, TextClass::Label, state);
    }
}

impl<W: Widget, D: Directional> HasStr for WithLabel<W, D> {
    fn get_str(&self) -> &str {
        self.label.as_str()
    }
}

impl<W: Widget, D: Directional> SetAccel for WithLabel<W, D> {
    fn set_accel_string(&mut self, string: AccelString) -> TkAction {
        let mut action = TkAction::empty();
        if self.label.text().keys() != string.keys() {
            action |= TkAction::RECONFIGURE;
        }
        action | set_text_and_prepare(&mut self.label, string, self.core.rect.size)
    }
}

impl<W: Widget, D: Directional> std::ops::Deref for WithLabel<W, D> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: Widget, D: Directional> std::ops::DerefMut for WithLabel<W, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
