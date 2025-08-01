// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Mandlebrot example
//!
//! Demonstrates use of a custom draw pipe.

use kas::decorations::{Decorations, TitleBarButtons};
use kas::draw::{Draw, DrawIface, PassId};
use kas::event::{self, Command};
use kas::geom::{Affine, DVec2, Linear, Vec2, Vec3};
use kas::prelude::*;
use kas::widgets::adapt::Reserve;
use kas::widgets::{Label, Slider, Text, format_value};
use kas_wgpu::draw::{CustomPipe, CustomPipeBuilder, CustomWindow, DrawCustom, DrawPipe};
use kas_wgpu::wgpu;
use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, ShaderModule, include_spirv};

#[cfg(not(feature = "shader64"))]
type ShaderVec2 = Vec2;
#[cfg(feature = "shader64")]
type ShaderVec2 = DVec2;

#[cfg(not(feature = "shader64"))]
const SHADER_FLOAT64: wgpu::Features = wgpu::Features::empty();
#[cfg(feature = "shader64")]
const SHADER_FLOAT64: wgpu::Features = wgpu::Features::SHADER_F64;

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
        let vertex = device.create_shader_module(include_spirv!("shader.vert.spv"));
        // Note: we don't use wgpu::include_spirv since it forces validation,
        // which cannot currently deal with double precision floats (dvec2).
        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
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
            required_features: wgpu::Features::PUSH_CONSTANTS | SHADER_FLOAT64,
            required_limits: wgpu::Limits {
                max_push_constant_size: size_of::<PushConstants>().cast(),
                ..Default::default()
            },
            ..Default::default()
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
                entry_point: Some("main"),
                compilation_options: Default::default(),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: tex_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Pipe { render_pipeline }
    }
}

struct Pipe {
    render_pipeline: wgpu::RenderPipeline,
}

#[derive(Default)]
struct PipeWindow {
    push_constants: PushConstants,
    passes: Vec<(Vec<Vertex>, Option<Buffer>, u32)>,
}

impl CustomPipe for Pipe {
    type Window = PipeWindow;

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
        if let Some(tuple) = window.passes.get(pass)
            && let Some(buffer) = tuple.1.as_ref()
        {
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

#[derive(Debug)]
struct ViewUpdate(String);

const INITIAL_OFFSET: DVec2 = DVec2(-0.5, 0.0);

#[impl_self]
mod Mandlebrot {
    #[widget]
    struct Mandlebrot {
        core: widget_core!(),
        transform: Affine,
        view_delta: DVec2,
        view_alpha: f64,
        rel_width: f32,
        iters: i32,
    }

    impl Mandlebrot {
        fn new() -> Self {
            Mandlebrot {
                core: Default::default(),
                transform: Affine::translate(INITIAL_OFFSET),
                view_delta: DVec2::ZERO,
                view_alpha: 0.0,
                rel_width: 0.0,
                iters: 64,
            }
        }

        fn reset_view(&mut self) {
            self.transform = Affine::translate(INITIAL_OFFSET);
        }

        fn loc(&self) -> String {
            let delta = self.transform.delta();
            let d0 = delta.0;
            let op = if delta.1 < 0.0 { "−" } else { "+" };
            let d1 = delta.1.abs();
            let s = self.transform.alpha().get_scale().ln();
            #[cfg(not(feature = "shader64"))]
            {
                format!("Location: {d0:.7} {op} {d1:.7}i; scale: {s:.2}")
            }
            #[cfg(feature = "shader64")]
            {
                format!("Location: {d0:.15} {op} {d1:.15}i; scale: {s:.2}")
            }
        }
    }

    impl Layout for Mandlebrot {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            kas::layout::LogicalSize(320.0, 240.0)
                .to_rules_with_factor(axis, sizer.scale_factor(), 4.0)
                .with_stretch(Stretch::High)
        }

        fn set_rect(&mut self, _: &mut ConfigCx, rect: Rect, _: AlignHints) {
            widget_set_rect!(rect);
            let size = DVec2::conv(rect.size);
            let rel_width = DVec2(size.0 / size.1, 1.0);
            self.view_alpha = 2.0 / size.1;
            self.view_delta = -(DVec2::conv(rect.pos) * 2.0 + size) / size.1;
            self.rel_width = rel_width.0 as f32;
        }

        fn draw(&self, mut draw: DrawCx) {
            let draw = draw.draw_device();
            let draw = DrawIface::<DrawPipe<Pipe>>::downcast_from(draw).unwrap();
            let alpha = self.transform.alpha().get_vec2();
            let delta = self.transform.delta();
            let p = (alpha, delta, self.rel_width, self.iters);
            draw.draw.custom(draw.get_pass(), self.rect(), p);
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Unknown
        }

        fn navigable(&self) -> bool {
            true
        }
    }

    impl Events for Mandlebrot {
        type Data = i32;

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn update(&mut self, _: &mut ConfigCx, data: &i32) {
            self.iters = *data;
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &i32, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, _) => match cmd {
                    Command::Home | Command::End => self.reset_view(),
                    Command::PageUp => self.transform /= Linear::scale(2f64.sqrt()),
                    Command::PageDown => self.transform *= Linear::scale(2f64.sqrt()),
                    cmd => {
                        let d = 0.2;
                        let delta = match cmd {
                            Command::Up => DVec2(0.0, -d),
                            Command::Down => DVec2(0.0, d),
                            Command::Left => DVec2(-d, 0.0),
                            Command::Right => DVec2(d, 0.0),
                            _ => return Unused,
                        };
                        self.transform += self.transform.alpha() * delta;
                    }
                },
                Event::Scroll(delta) => match delta.as_factor_or_offset(cx) {
                    Ok(factor) => self.transform *= Linear::scale(2f64.powf(factor)),
                    Err(offset) => {
                        self.transform -=
                            self.transform.alpha() * self.view_alpha * DVec2::conv(offset);
                    }
                },
                Event::Pan(t) => {
                    // Our full transform (from screen coordinates to world coordinates) is:
                    // f(p) = α_w * α_v * p + α_w * δ_v + δ_w
                    // where self.transform (α_w, δ_w) is the world transform
                    // and α_v, δ_v are view transforms (see notes in PipeWindow::invoke).
                    //
                    // To adjust the world offset (in reverse), we use the following formulae:
                    // α_w' = α_w / α
                    // δ_w' = δ_w - α_w' * α_v * δ + (α_w - α_w') δ_v
                    // where (α, δ) is t and x' is the "new x".
                    let new_alpha = self.transform.alpha() / t.alpha();
                    let new_delta = self.transform.delta()
                        - new_alpha * self.view_alpha * t.delta()
                        + (self.transform.alpha() - new_alpha) * self.view_delta;
                    self.transform = Affine::new(new_alpha, new_delta);
                }
                Event::PressStart { press } => {
                    return press
                        .grab(self.id(), event::GrabMode::PAN_FULL)
                        .with_icon(event::CursorIcon::Grabbing)
                        .complete(cx);
                }
                _ => return Unused,
            }

            cx.redraw(self.id());
            cx.push(ViewUpdate(self.loc()));
            Used
        }
    }
}

#[impl_self]
mod MandlebrotUI {
    #[widget]
    #[layout(grid! {
        (1, 0) => self.label,
        (2, 0) => self.title,
        (3, 0) => self.buttons,
        (0, 1) => self.iters_label.align(AlignHints::CENTER),
        (0, 2) => self.slider,
        // extra col span allows use of Label's margin
        (1..5, 1..4) => self.mbrot,
    })]
    struct MandlebrotUI {
        core: widget_core!(),
        #[widget(&self.loc)]
        label: Text<String, String>,
        #[widget]
        title: Label<&'static str>,
        #[widget]
        buttons: TitleBarButtons,
        #[widget(&self.iters)]
        iters_label: Reserve<Text<i32, String>>,
        #[widget(&self.iters)]
        slider: Slider<i32, i32, kas::dir::Up>,
        #[widget(&self.iters)]
        mbrot: Mandlebrot,
        iters: i32,
        loc: String,
    }

    impl MandlebrotUI {
        fn new() -> MandlebrotUI {
            let mbrot = Mandlebrot::new();
            let loc = mbrot.loc();
            MandlebrotUI {
                core: Default::default(),
                label: format_value!("{}"),
                title: Label::new("Mandlebrot"),
                buttons: Default::default(),
                iters_label: format_value!("{}").with_min_size_em(3.0, 0.0),
                slider: Slider::up(0..=256, |_, iters| *iters).with_msg(|iters| iters),
                mbrot,
                iters: 64,
                loc,
            }
        }
    }
    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, data: &()) {
            if let Some(iters) = cx.try_pop() {
                self.iters = iters;
            } else if let Some(ViewUpdate(loc)) = cx.try_pop() {
                self.loc = loc;
            } else {
                return;
            }
            cx.update(self.as_node(data));
        }
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let window =
        Window::new(MandlebrotUI::new(), "Mandlebrot").with_decorations(Decorations::Border);
    let mut runner = kas::runner::Runner::with_wgpu_pipe(PipeBuilder)
        .with_theme(kas::theme::FlatTheme::new())
        .build(())?;
    let _ = runner.config_mut().theme.set_active_scheme("dark");
    runner.with(window).run()
}
