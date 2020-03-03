// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom draw pipes

use super::DrawWindow;
use kas::draw::Region;
use kas::geom::{Rect, Size};

/// Allows use of the low-level graphics API
///
/// To use this, write an implementation of [`CustomPipe`], then pass the
/// corresponding [`CustomPipeBuilder`] to [`crate::Toolkit::new_custom`].
pub trait DrawCustom<CW: CustomWindow> {
    /// Call a custom draw pipe
    fn custom(&mut self, region: Region, rect: Rect, param: CW::Param);
}

/// Builder for a [`CustomPipe`]
pub trait CustomPipeBuilder {
    type Pipe: CustomPipe;

    /// Build a pipe
    fn build(&mut self, device: &wgpu::Device, tex_format: wgpu::TextureFormat) -> Self::Pipe;
}

/// A custom draw pipe
///
/// A "draw pipe" consists of draw primitives (usually triangles), resources
/// (textures), shaders, and pipe configuration (e.g. blending mode).
/// A custom pipe allows direct use of the WebGPU graphics stack.
///
/// To use this, pass the corresponding [`CustomPipeBuilder`] to
/// [`crate::Toolkit::new_custom`].
///
/// Note that `kas-wgpu` accepts only a single custom pipe. To use more than
/// one, you will have to implement your own multiplexer (presumably using an
/// enum for the `Param` type).
pub trait CustomPipe {
    /// Type for per-window data
    type Window: CustomWindow + 'static;

    /// Construct a window associated with this pipeline
    fn new_window(&self, device: &wgpu::Device, size: Size) -> Self::Window;

    /// Do a render pass.
    ///
    /// Rendering uses one pass per region, where each region has its own
    /// scissor rect. This method may be called multiple times per frame.
    /// Each widget invoking this pipe will give the correct `pass` number for
    /// the widget in [`CustomWindow::invoke`]; multiple widgets may use the same
    /// `pass`.
    fn render(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        pass: usize,
        rpass: &mut wgpu::RenderPass,
    );
}

pub trait CustomWindow {
    /// User parameter type
    type Param;

    /// Called whenever the window is resized
    fn resize(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, size: Size);

    /// Invoke user-defined custom routine
    ///
    /// Custom add-primitives / update function called from user code by
    /// [`DrawCustom::custom`].
    fn invoke(&mut self, pass: usize, rect: Rect, param: Self::Param);
}

/// A dummy implementation (does nothing)
impl CustomPipeBuilder for () {
    type Pipe = ();
    fn build(&mut self, _: &wgpu::Device, _: wgpu::TextureFormat) -> Self::Pipe {
        ()
    }
}

pub enum Void {}

/// A dummy implementation (does nothing)
impl CustomPipe for () {
    type Window = ();
    fn new_window(&self, _: &wgpu::Device, _: Size) -> Self::Window {
        ()
    }
    fn render(&self, _: &mut Self::Window, _: &wgpu::Device, _: usize, _: &mut wgpu::RenderPass) {}
}

/// A dummy implementation (does nothing)
impl CustomWindow for () {
    type Param = Void;
    fn resize(&mut self, _: &wgpu::Device, _: &mut wgpu::CommandEncoder, _: Size) {}
    fn invoke(&mut self, _: usize, _: Rect, _: Self::Param) {}
}

impl<CW: CustomWindow> DrawCustom<CW> for DrawWindow<CW> {
    fn custom(&mut self, region: Region, rect: Rect, param: CW::Param) {
        self.custom.invoke(region.0, rect, param);
    }
}
