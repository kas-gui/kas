// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom draw pipes

use super::DrawPipe;
use kas::draw::Region;
use kas::geom::{Rect, Size};

/// Abstraction allowing custom draw implementation
pub trait DrawCustom<C: CustomPipe> {
    /// Call a custom draw pipe
    fn custom(&mut self, region: Region, rect: Rect, param: C::Param);
}

/// A custom draw pipe
///
/// A "draw pipe" consists of draw primitives (usually triangles), resources
/// (textures), shaders, and pipe configuration (e.g. blending mode).
/// A custom pipe allows direct use of the WebGPU graphics stack.
///
/// Note that `kas-wgpu` accepts only a single custom pipe. To use more than
/// one, you will have to implement your own multiplexer (presumably using an
/// enum for the `Param` type).
pub trait CustomPipe: Clone {
    /// User parameter type
    type Param;

    /// Initialisation routines
    fn init(&mut self, device: &wgpu::Device, size: Size);

    /// Called whenever the window is resized
    fn resize(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, size: Size);

    /// Invoke user-defined custom routine
    ///
    /// Custom add-primitives / update function called from user code by
    /// [`DrawCustom::custom`].
    fn invoke(&mut self, pass: usize, rect: Rect, param: Self::Param);

    /// Do a render pass.
    ///
    /// Rendering uses one pass per region, where each region has its own
    /// scissor rect. This method may be called multiple times per frame.
    /// Each widget invoking this pipe will give the correct `pass` number for
    /// the widget in [`CustomPipe::invoke`]; multiple widgets may use the same
    /// `pass`.
    fn render(&mut self, device: &wgpu::Device, pass: usize, rpass: &mut wgpu::RenderPass);
}

pub enum Void {}

/// A dummy implementation (does nothing)
impl CustomPipe for () {
    type Param = Void;
    fn init(&mut self, _: &wgpu::Device, _: Size) {}
    fn resize(&mut self, _: &wgpu::Device, _: &mut wgpu::CommandEncoder, _: Size) {}
    fn invoke(&mut self, _: usize, _: Rect, _: Self::Param) {}
    fn render(&mut self, _: &wgpu::Device, _: usize, _: &mut wgpu::RenderPass) {}
}

impl<C: CustomPipe> DrawCustom<C> for DrawPipe<C> {
    fn custom(&mut self, region: Region, rect: Rect, param: C::Param) {
        self.custom.invoke(region.0, rect, param);
    }
}
