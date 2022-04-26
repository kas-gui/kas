// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Mandlebrot example
//!
//! Demonstrates use of a custom draw pipe.

use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::{include_spirv, Buffer, ShaderModule};

use kas::draw::{Draw, DrawIface, PassId};
use kas::event::{self, Command};
use kas::geom::{DVec2, Vec2, Vec3};
use kas::prelude::*;
use kas::shell::draw::{CustomPipe, CustomPipeBuilder, CustomWindow, DrawCustom, DrawPipe};
use kas::shell::Options;
use kas::widgets::adapter::ReserveP;
use kas::widgets::{Label, Slider, Window};

#[cfg(not(feature = "shader64"))]
type ShaderVec2 = Vec2;
#[cfg(feature = "shader64")]
type ShaderVec2 = DVec2;

#[cfg(not(feature = "shader64"))]
const SHADER_FLOAT64: wgpu::Features = wgpu::Features::empty();
#[cfg(feature = "shader64")]
const SHADER_FLOAT64: wgpu::Features = wgpu::Features::SHADER_FLOAT64;

#[cfg(not(feature = "shader64"))]
const FRAG_SHADER: &[u8] = include_bytes!("shader32.frag.spv");
#[cfg(feature = "shader64")]
const FRAG_SHADER: &[u8] = include_bytes!("shader64.frag.spv");

struct Shaders {
    vertex: ShaderModule,
    fragment: ShaderModule,
}

impl Shaders {
    fn new(device: &wgpu::Device) -> Self {
        let vertex = device.create_shader_module(&include_spirv!("shader.vert.spv"));
        // Note: we don't use wgpu::include_spirv since it forces validation,
        // which cannot currently deal with double precision floats (dvec2).
        let fragment = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("fragment shader"),
            source: wgpu::util::make_spirv(FRAG_SHADER),
        });

        Shaders { vertex, fragment }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct PushConstants {
    p: ShaderVec2,
    q: ShaderVec2,
    iterations: i32,
}
impl Default for PushConstants {
    fn default() -> Self {
        PushConstants {
            p: ShaderVec2::splat(0.0),
            q: ShaderVec2::splat(1.0),
            iterations: 64,
        }
    }
}
impl PushConstants {
    fn set(&mut self, p: DVec2, q: DVec2, iterations: i32) {
        #[cfg(feature = "shader64")]
        {
            self.p = p;
            self.q = q;
        }
        #[cfg(not(feature = "shader64"))]
        {
            self.p = p.cast_approx();
            self.q = q.cast_approx();
        }
        self.iterations = iterations;
    }
}
unsafe impl bytemuck::Zeroable for PushConstants {}
unsafe impl bytemuck::Pod for PushConstants {}

struct PipeBuilder;

impl CustomPipeBuilder for PipeBuilder {
    type Pipe = Pipe;

    fn device_descriptor() -> wgpu::DeviceDescriptor<'static> {
        wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::PUSH_CONSTANTS | SHADER_FLOAT64,
            limits: wgpu::Limits {
                max_push_constant_size: size_of::<PushConstants>().cast(),
                ..Default::default()
            },
        }
    }

    fn build(
        &mut self,
        device: &wgpu::Device,
        bgl_common: &wgpu::BindGroupLayout,
        tex_format: wgpu::TextureFormat,
    ) -> Self::Pipe {
        let shaders = Shaders::new(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bgl_common],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::FRAGMENT,
                range: 0..size_of::<PushConstants>().cast(),
            }],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vertex,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back), // not required
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.fragment,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: tex_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
        });

        Pipe { render_pipeline }
    }
}

struct Pipe {
    render_pipeline: wgpu::RenderPipeline,
}

struct PipeWindow {
    push_constants: PushConstants,
    passes: Vec<(Vec<Vertex>, Option<Buffer>, u32)>,
}

impl CustomPipe for Pipe {
    type Window = PipeWindow;

    fn new_window(&self, _: &wgpu::Device) -> Self::Window {
        PipeWindow {
            push_constants: Default::default(),
            passes: vec![],
        }
    }

    fn prepare(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        _: &mut wgpu::util::StagingBelt,
        _: &mut wgpu::CommandEncoder,
    ) {
        // Upload per-pass render data to buffers. NOTE: we could use a single
        // buffer for all passes like in kas-wgpu/src/draw/common.rs.
        for pass in &mut window.passes {
            if !pass.0.is_empty() {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&pass.0),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.1 = Some(buffer);
                pass.2 = pass.0.len().cast();
                pass.0.clear();
            } else {
                pass.1 = None;
            }
        }
    }

    fn render_pass<'a>(
        &'a self,
        window: &'a mut Self::Window,
        _: &wgpu::Device,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        if let Some(tuple) = window.passes.get(pass) {
            if let Some(buffer) = tuple.1.as_ref() {
                rpass.set_pipeline(&self.render_pipeline);
                rpass.set_push_constants(
                    wgpu::ShaderStages::FRAGMENT,
                    0,
                    bytemuck::bytes_of(&window.push_constants),
                );
                rpass.set_bind_group(0, bg_common, &[]);
                rpass.set_vertex_buffer(0, buffer.slice(..));
                rpass.draw(0..tuple.2, 0..1);
            }
        }
    }
}

impl CustomWindow for PipeWindow {
    type Param = (DVec2, DVec2, f32, i32);

    fn invoke(&mut self, pass: PassId, rect: Rect, p: Self::Param) {
        self.push_constants.set(p.0, p.1, p.3);
        let rel_width = p.2;

        let aa = Vec2::conv(rect.pos);
        let bb = aa + Vec2::conv(rect.size);

        let depth = pass.depth();
        let ab = Vec3(aa.0, bb.1, depth);
        let ba = Vec3(bb.0, aa.1, depth);
        let aa = Vec3::from2(aa, depth);
        let bb = Vec3::from2(bb, depth);

        // Fix height to 2 here; width is relative:
        let cbb = Vec2(rel_width, 1.0);
        let caa = -cbb;
        let cab = Vec2(caa.0, cbb.1);
        let cba = Vec2(cbb.0, caa.1);

        // Effectively, this gives us the following "view transform":
        // α_v * p + δ_v = 2 * (p - rect.pos) * (rel_width / width, 1 / height) - (rel_width, 1)
        //               = 2 * (p - rect.pos) / height - (width, height) / height
        //               = 2p / height - (2*rect.pos + rect.size) / height
        // Or: α_v = 2 / height, δ_v = -(2*rect.pos + rect.size) / height
        // This is used to define view_alpha and view_delta (in Mandlebrot::set_rect).

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            Vertex(aa, caa), Vertex(ba, cba), Vertex(ab, cab),
            Vertex(ab, cab), Vertex(ba, cba), Vertex(bb, cbb),
        ]);
    }
}

impl PipeWindow {
    fn add_vertices(&mut self, pass: usize, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize_with(pass + 8, Default::default);
        }

        self.passes[pass].0.extend_from_slice(slice);
    }
}

#[derive(Clone, Debug)]
struct ViewUpdate;

impl_scope! {
    #[derive(Clone, Debug)]
    #[widget]
    struct Mandlebrot {
        #[widget_core]
        core: kas::CoreData,
        alpha: DVec2,
        delta: DVec2,
        view_delta: DVec2,
        view_alpha: f64,
        rel_width: f32,
        iter: i32,
    }

    impl Mandlebrot {
        fn new() -> Self {
            Mandlebrot {
                core: Default::default(),
                alpha: DVec2(1.0, 0.0),
                delta: DVec2(-0.5, 0.0),
                view_delta: DVec2::ZERO,
                view_alpha: 0.0,
                rel_width: 0.0,
                iter: 64,
            }
        }

        fn reset_view(&mut self) {
            self.alpha = DVec2(1.0, 0.0);
            self.delta = DVec2(-0.5, 0.0);
        }

        fn loc(&self) -> String {
            let op = if self.delta.1 < 0.0 { "−" } else { "+" };
            format!(
                "Location: {} {} {}i; scale: {}",
                self.delta.0,
                op,
                self.delta.1.abs(),
                self.alpha.sum_square().sqrt()
            )
        }
    }

    impl WidgetConfig for Mandlebrot {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.register_nav_fallback(self.id());
        }

        fn key_nav(&self) -> bool {
            true
        }
    }

    impl Layout for Mandlebrot {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            // We use a reasonable minimum size of 300x200 and a large ideal
            // size of 3000x2000: the initial size should fill the screen.
            LogicalSize(300.0, 200.0)
                .to_rules_with_factor(axis, mgr.scale_factor(), 10.0)
                .with_stretch(Stretch::High)
        }

        #[inline]
        fn set_rect(&mut self, _: &mut SetRectMgr, rect: Rect, _: AlignHints) {
            self.core.rect = rect;
            let size = DVec2::conv(rect.size);
            let rel_width = DVec2(size.0 / size.1, 1.0);
            self.view_alpha = 2.0 / size.1;
            self.view_delta = -(DVec2::conv(rect.pos) * 2.0 + size) / size.1;
            self.rel_width = rel_width.0 as f32;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let draw = draw.draw_device();
            let draw = DrawIface::<DrawPipe<Pipe>>::downcast_from(draw).unwrap();
            let p = (self.alpha, self.delta, self.rel_width, self.iter);
            draw.draw.custom(draw.get_pass(), self.core.rect, p);
        }
    }

    impl event::Handler for Mandlebrot {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(cmd, _) => {
                    match cmd {
                        Command::Home | Command::End => self.reset_view(),
                        Command::PageUp => self.alpha = self.alpha / 2f64.sqrt(),
                        Command::PageDown => self.alpha = self.alpha * 2f64.sqrt(),
                        cmd => {
                            let d = 0.2;
                            let delta = match cmd {
                                Command::Up => DVec2(0.0, -d),
                                Command::Down => DVec2(0.0, d),
                                Command::Left => DVec2(-d, 0.0),
                                Command::Right => DVec2(d, 0.0),
                                _ => return Response::Unused,
                            };
                            self.delta += self.alpha.complex_mul(delta);
                        }
                    }
                    mgr.push_msg(ViewUpdate);
                }
                Event::Scroll(delta) => {
                    let factor = match delta {
                        event::ScrollDelta::LineDelta(_, y) => -0.5 * y as f64,
                        event::ScrollDelta::PixelDelta(coord) => -0.01 * coord.1 as f64,
                    };
                    self.alpha = self.alpha * 2f64.powf(factor);
                    mgr.push_msg(ViewUpdate);
                }
                Event::Pan { alpha, delta } => {
                    // Our full transform (from screen coordinates to world coordinates) is:
                    // f(p) = α_w * α_v * p + α_w * δ_v + δ_w
                    // where _w indicate world transforms (self.alpha, self.delta)
                    // and _v indicate view transforms (see notes in PipeWindow::invoke).
                    //
                    // To adjust the world offset (in reverse), we use the following formulae:
                    // α_w' = (1/α) * α_w
                    // δ_w' = δ_w - α_w' * α_v * δ + (α_w - α_w') δ_v
                    // where x' is the "new x".
                    let new_alpha = self.alpha.complex_div(alpha);
                    self.delta = self.delta - new_alpha.complex_mul(delta) * self.view_alpha
                        + (self.alpha - new_alpha).complex_mul(self.view_delta);
                    self.alpha = new_alpha;

                    mgr.push_msg(ViewUpdate);
                }
                Event::PressStart { source, coord, .. } => {
                    mgr.grab_press(
                        self.id(),
                        source,
                        coord,
                        event::GrabMode::PanFull,
                        Some(event::CursorIcon::Grabbing),
                    );
                }
                _ => return Response::Unused,
            }
            Response::Used
        }
    }
}

impl_scope! {
    #[derive(Debug)]
    #[widget{
        layout = grid: {
            0..2, 0: self.label;
            0, 1: align(center): self.iters;
            0, 2: self.slider;
            1..3, 1..3: self.mbrot;
        };
    }]
    struct MandlebrotWindow {
        #[widget_core] core: CoreData,
        #[widget] label: Label<String>,
        #[widget] iters: ReserveP<Label<String>>,
        #[widget] slider: Slider<i32, kas::dir::Up>,
        // extra col span allows use of Label's margin
        #[widget] mbrot: Mandlebrot,
    }

    impl MandlebrotWindow {
        fn new_window() -> Window<MandlebrotWindow> {
            let slider = Slider::new(0, 256, 1).with_value(64);
            let mbrot = Mandlebrot::new();
            let w = MandlebrotWindow {
                core: Default::default(),
                label: Label::new(mbrot.loc()),
                iters: ReserveP::new(Label::from("64"), |size_mgr, axis| {
                    Label::new("000").size_rules(size_mgr, axis)
                }),
                slider,
                mbrot,
            };
            Window::new("Mandlebrot", w)
        }
    }
    impl Handler for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(iter) = mgr.try_pop_msg() {
                self.mbrot.iter = iter;
                *mgr |= self.iters.set_string(format!("{}", iter));
            } else if let Some(ViewUpdate) = mgr.try_pop_msg() {
                mgr.redraw(self.mbrot.id());
                *mgr |= self.label.set_string(self.mbrot.loc());
            }
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas::theme::FlatTheme::new().with_colours("dark");

    kas::shell::Toolkit::new_custom(PipeBuilder, theme, Options::from_env())?
        .with(MandlebrotWindow::new_window())?
        .run()
}
