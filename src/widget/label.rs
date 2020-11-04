// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::draw::TextClass;
use kas::event::{self, GrabMode, PressSource};
use kas::prelude::*;
use kas::text::format::{EditableText, FormattableText};
use kas::text::SelectionHelper;

/// A text label
///
/// This type is generic over the text type. Some aliases are available:
/// [`StrLabel`], [`StringLabel`], [`AccelLabel`].
#[handler(handle=noauto)]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label<T: FormattableText + 'static> {
    #[widget_core]
    core: CoreData,
    reserve: Option<T>,
    label: Text<T>,
    selection: SelectionHelper,
}

mod impls {
    use super::*;

    pub fn size_rules<T: FormattableText + 'static>(
        obj: &mut Label<T>,
        size_handle: &mut dyn SizeHandle,
        axis: AxisInfo,
    ) -> SizeRules {
        let mut prepared = None;
        let text = if let Some(s) = obj.reserve.take() {
            prepared = Some(Text::new_multi(s));
            prepared.as_mut().unwrap()
        } else {
            &mut obj.label
        };
        let rules = size_handle.text_bound(text, TextClass::Label, axis);
        if let Some(text) = prepared {
            obj.reserve = Some(text.take_text());
        }
        if axis.is_horizontal() {
            obj.core.rect.size.0 = rules.ideal_size();
        } else {
            obj.core.rect.size.1 = rules.ideal_size();
        }
        rules
    }

    pub fn set_rect<T: FormattableText + 'static>(
        obj: &mut Label<T>,
        rect: Rect,
        align: AlignHints,
    ) {
        obj.core.rect = rect;
        obj.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Default, Align::Centre));
        });
    }

    pub fn draw<T: FormattableText + 'static>(obj: &Label<T>, draw_handle: &mut dyn DrawHandle) {
        draw_handle.text_effects_selected(
            obj.core.rect.pos,
            Coord::ZERO,
            &obj.label,
            obj.selection.range().into(),
            TextClass::Label,
        );
    }
}

impl<T: FormattableText + 'static> Layout for Label<T> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        impls::size_rules(self, size_handle, axis)
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        impls::set_rect(self, rect, align);
    }

    #[cfg(feature = "min_spec")]
    default fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        impls::draw(self, draw_handle);
    }
    #[cfg(not(feature = "min_spec"))]
    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        impls::draw(self, draw_handle);
    }
}

#[cfg(feature = "min_spec")]
impl Layout for AccelLabel {
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, _: bool) {
        // Ignore selection for AccelLabel
        let state = mgr.show_accel_labels();
        draw_handle.text_accel(self.core.rect.pos, &self.label, state, TextClass::Label);
    }
}

// Str/String representations have no effects, so use simpler draw call
#[cfg(feature = "min_spec")]
impl<'a> Layout for Label<&'a str> {
    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        let bounds = self.label.env().bounds.into();
        draw_handle.text_selected(
            self.core.rect.pos,
            bounds,
            Coord::ZERO,
            &self.label,
            self.selection.range(),
            TextClass::Label,
        );
    }
}
#[cfg(feature = "min_spec")]
impl Layout for StringLabel {
    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState, _: bool) {
        let bounds = self.label.env().bounds.into();
        draw_handle.text_selected(
            self.core.rect.pos,
            bounds,
            Coord::ZERO,
            &self.label,
            self.selection.range(),
            TextClass::Label,
        );
    }
}

impl<T: FormattableText + 'static> event::Handler for Label<T> {
    type Msg = event::VoidMsg;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::LostSelFocus => {
                self.selection.set_empty();
                mgr.redraw(self.id());
                Response::None
            }
            Event::PressStart { source, coord, .. }
                if source.is_primary() && !mgr.modifiers().ctrl() =>
            {
                // TODO: this should handle touch on a timer (like EditBox) but
                // without blocking panning if parent uses unhandled events
                if let PressSource::Mouse(_, repeats) = source {
                    // With Ctrl held, we let parent handle for scrolling
                    self.set_edit_pos_from_coord(mgr, coord);
                    if !mgr.modifiers().shift() {
                        self.selection.set_empty();
                    }
                    self.selection.set_anchor();
                    if repeats > 1 {
                        self.selection.expand(&self.label, repeats);
                    }
                    mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                }
                Response::None
            }
            Event::PressMove { source, coord, .. } => {
                // TODO: if mgr.modifiers().ctrl() then this should pan, not select!
                if let PressSource::Mouse(_, repeats) = source {
                    self.set_edit_pos_from_coord(mgr, coord);
                    if repeats > 1 {
                        self.selection.expand(&self.label, repeats);
                    }
                }
                Response::None
            }
            Event::PressEnd { .. } => Response::None,
            event => Response::Unhandled(event),
        }
    }
}

/* TODO(specialization): can we support this? min_specialization is not enough.
impl<U, T: From<U> + FormattableText + 'static> From<U> for Label<T> {
    default fn from(text: U) -> Self {
        let text = T::from(text);
        Label::new(text)
    }
}*/

impl<T: FormattableText + 'static> From<T> for Label<T> {
    fn from(label: T) -> Self {
        Label::new(label)
    }
}

impl<'a> From<&'a str> for Label<String> {
    fn from(label: &'a str) -> Self {
        Label::new(label.to_string())
    }
}

impl<T: FormattableText + 'static> Label<T> {
    /// Construct from `label`
    pub fn new(label: T) -> Self {
        Label {
            core: Default::default(),
            reserve: None,
            label: Text::new_multi(label),
            selection: SelectionHelper::new(0, 0),
        }
    }

    /// Reserve sufficient room for the given text
    ///
    /// If this option is used, the label will be sized to fit this text, not
    /// the actual text.
    pub fn with_reserve(mut self, text: T) -> Self {
        self.reserve = Some(text);
        self
    }

    /// Set text in an existing `Label`
    ///
    /// Note: this must not be called before fonts have been initialised
    /// (usually done by the theme when the main loop starts).
    pub fn set_text(&mut self, text: T) -> TkAction {
        kas::text::util::set_text_and_prepare(&mut self.label, text)
    }

    fn set_edit_pos_from_coord(&mut self, mgr: &mut Manager, coord: Coord) {
        let rel_pos = (coord - self.core.rect.pos).into();
        self.selection
            .set_edit_pos(self.label.text_index_nearest(rel_pos));
        mgr.redraw(self.id());
    }
}

impl<T: FormattableText + 'static> HasStr for Label<T> {
    fn get_str(&self) -> &str {
        self.label.as_str()
    }
}

impl<T: FormattableText + EditableText + 'static> HasString for Label<T> {
    fn set_string(&mut self, string: String) -> TkAction {
        kas::text::util::set_string_and_prepare(&mut self.label, string)
    }
}

/// Label with `&'static str` as backing type
///
/// Warning: this type does not support [`HasString`]. Assignment is possible
/// via [`Label::set_text`], but only for `&'static str`, so most of the time
/// [`StringLabel`] will be preferred when assignment is required.
/// (Also note that the overhead of allocating and copying a `String` is
/// irrelevant considering those used for text layout and drawing.)
pub type StrLabel = Label<&'static str>;

/// Label with `String` as backing type
pub type StringLabel = Label<String>;

/// A label supporting an accelerator key
///
/// Accelerator keys are not useful on plain labels, but this widget may be
/// embedded within a parent (e.g. `CheckBox` uses this).
// TODO: we probably don't need `reserve` for AccelLabel
pub type AccelLabel = Label<AccelString>;

impl AccelLabel {
    /// Get the accelerator keys
    pub fn keys(&self) -> &[event::VirtualKeyCode] {
        &self.label.text().keys()
    }
}

impl SetAccel for AccelLabel {
    fn set_accel_string(&mut self, string: AccelString) -> TkAction {
        kas::text::util::set_text_and_prepare(&mut self.label, string)
    }
}
