// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom draw pipes

use super::DrawWindow;
use kas::draw::PassId;
use kas::geom::{Rect, Size};

/// Allows use of the low-level graphics API
///
/// To use this, write an implementation of [`CustomPipe`], then pass the
/// corresponding [`CustomPipeBuilder`] to [`crate::Toolkit::new_custom`].
pub trait DrawCustom<CW: CustomWindow> {
    /// Call a custom draw pipe
    fn custom(&mut self, pass: PassId, rect: Rect, param: CW::Param);
}

/// Builder for a [`CustomPipe`]
pub trait CustomPipeBuilder {
    type Pipe: CustomPipe;

    /// Request a device supporting these features and limits
    ///
    /// See [`wgpu::Adapter::request_device`] and [`wgpu::DeviceDescriptor`] doc.
    fn device_descriptor() -> wgpu::DeviceDescriptor<'static> {
        Default::default()
    }

    /// Build a pipe
    ///
    /// A "common" bind group with layout `bgl_common` is available, supplying
    /// window sizing and theme lighting information (refer to existing pipes
    /// and shaders for usage). Usage is optional.
    ///
    /// The given texture format should be used to construct a
    /// compatible [`wgpu::RenderPipeline`].
    fn build(
        &mut self,
        device: &wgpu::Device,
        bgl_common: &wgpu::BindGroupLayout,
        tex_format: wgpu::TextureFormat,
    ) -> Self::Pipe;
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
/// one custom graphics pipeline, you must implement your own multiplexer.
pub trait CustomPipe: 'static {
    /// Associated per-window state for the custom pipe
    type Window: CustomWindow;

    /// Construct a window associated with this pipeline
    fn new_window(&self, device: &wgpu::Device, size: Size) -> Self::Window;

    /// Called whenever the window is resized
    fn resize(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: Size,
    ) {
        let _ = (window, device, queue, size);
    }

    /// Per-frame updates
    ///
    /// This is called once per frame before rendering operations, and may for
    /// example be used to prepare uniform and buffers.
    ///
    /// This method is optional; by default it does nothing.
    fn prepare(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let _ = (window, device, staging_belt, encoder);
    }

    /// Render (pass)
    ///
    /// Each item drawn is associated with a clip region, and each of these
    /// with a pass. This method will be called once for each clip region in use
    /// (possibly also for other clip regions). Drawing uses an existing texture
    /// and occurs after most other draw operations, but before text.
    ///
    /// The "common" bind group supplies window scaling and theme lighting
    /// information may optionally be set (see [`CustomPipeBuilder::build`]).
    ///
    /// This method is optional; by default it does nothing.
    #[allow(unused)]
    fn render_pass<'a>(
        &'a self,
        window: &'a mut Self::Window,
        device: &wgpu::Device,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
    }

    /// Render (final)
    ///
    /// This method is the last step in drawing a frame except for text
    /// rendering. Depending on the application, it may make more sense to draw
    /// in [`CustomPipe::render_pass`] or in this method.
    ///
    /// This method is optional; by default it does nothing.
    #[allow(unused)]
    fn render_final<'a>(
        &'a self,
        window: &'a mut Self::Window,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        size: Size,
    ) {
    }
}

/// Per-window state for a custom draw pipe
///
/// One instance is constructed per window. Since the [`CustomPipe`] is not
/// accessible during a widget's [`kas::Layout::draw`] calls, this struct must
/// batch per-frame draw data.
pub trait CustomWindow: 'static {
    /// User parameter type
    type Param;

    /// Invoke user-defined custom routine
    ///
    /// Custom add-primitives / update function called from user code by
    /// [`DrawCustom::custom`].
    fn invoke(&mut self, pass: PassId, rect: Rect, param: Self::Param);
}

/// A dummy implementation (does nothing)
impl CustomPipeBuilder for () {
    type Pipe = ();
    fn build(
        &mut self,
        _: &wgpu::Device,
        _: &wgpu::BindGroupLayout,
        _: wgpu::TextureFormat,
    ) -> Self::Pipe {
    }
}

pub enum Void {}

/// A dummy implementation (does nothing)
impl CustomPipe for () {
    type Window = ();
    fn new_window(&self, _: &wgpu::Device, _: Size) -> Self::Window {}
    fn resize(&self, _: &mut Self::Window, _: &wgpu::Device, _: &wgpu::Queue, _: Size) {}
}

/// A dummy implementation (does nothing)
impl CustomWindow for () {
    type Param = Void;
    fn invoke(&mut self, _: PassId, _: Rect, _: Self::Param) {}
}

impl<CW: CustomWindow> DrawCustom<CW> for DrawWindow<CW> {
    fn custom(&mut self, pass: PassId, rect: Rect, param: CW::Param) {
        self.custom.invoke(pass, rect, param);
    }
}
