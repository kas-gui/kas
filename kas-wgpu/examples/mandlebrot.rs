// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Mandlebrot example
//!
//! Demonstrates use of a custom draw pipe.
#![feature(proc_macro_hygiene)]

use shaderc::{Compiler, ShaderKind};
use std::mem::size_of;
use wgpu::ShaderModule;

use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{self, Event, Manager, Response, VoidResponse};
use kas::geom::{Coord, DVec2, Rect, Size, Vec2};
use kas::layout::{AxisInfo, SizeRules, StretchPolicy};
use kas::macros::make_widget;
use kas::widget::{Label, Slider, Window};
use kas::{AlignHints, Layout, ThemeApi, Vertical, WidgetCore, WidgetId};
use kas_wgpu::draw::{CustomPipe, CustomPipeBuilder, CustomWindow, DrawCustom, DrawWindow};
use kas_wgpu::Options;

const VERTEX: &'static str = "
#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a1;

layout(location = 0) out vec2 b1;

layout(set = 0, binding = 0) uniform Locals {
    vec2 scale;
};

const vec2 offset = { 1.0, 1.0 };

void main() {
    gl_Position = vec4(scale * a_pos - offset, 0.0, 1.0);
    b1 = a1;
}
";
const FRAGMENT: &'static str = "
#version 450
#extension GL_ARB_separate_shader_objects : enable

precision highp float;

layout(location = 0) in vec2 cf;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 1) uniform Locals {
    dvec2 alpha;
    dvec2 delta;
};

layout(set = 0, binding = 2) uniform Iters {
    int iter;
};

void main() {
    dvec2 cd = cf;
    dvec2 c = dvec2(alpha.x * cd.x - alpha.y * cd.y, alpha.x * cd.y + alpha.y * cd.x) + delta;

    dvec2 z = c;
    int i;
    for(i=0; i<iter; i++) {
        double x = (z.x * z.x - z.y * z.y) + c.x;
        double y = (z.y * z.x + z.x * z.y) + c.y;

        if((x * x + y * y) > 4.0) break;
        z.x = x;
        z.y = y;
    }

    float r = (i == iter) ? 0.0 : float(i) / iter;
    float g = r * r;
    float b = g * g;
    outColor = vec4(r, g, b, 1.0);
}
";

struct Shaders {
    vertex: ShaderModule,
    fragment: ShaderModule,
}

impl Shaders {
    fn compile(device: &wgpu::Device) -> Self {
        let mut compiler = Compiler::new().unwrap();

        let artifact = compiler
            .compile_into_spirv(VERTEX, ShaderKind::Vertex, "VERTEX", "main", None)
            .unwrap();
        let vertex = device.create_shader_module(&artifact.as_binary());

        let artifact = compiler
            .compile_into_spirv(FRAGMENT, ShaderKind::Fragment, "FRAGMENT", "main", None)
            .unwrap();
        let fragment = device.create_shader_module(&artifact.as_binary());

        Shaders { vertex, fragment }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec2, Vec2);

struct PipeBuilder;

impl CustomPipeBuilder for PipeBuilder {
    type Pipe = Pipe;

    fn build(&mut self, device: &wgpu::Device, _: wgpu::TextureFormat) -> Self::Pipe {
        // Note: real apps should compile shaders once and share between windows
        let shaders = Shaders::compile(device);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &shaders.vertex,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &shaders.fragment,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: (size_of::<Vec2>()) as u64,
                        shader_location: 1,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Pipe {
            bind_group_layout,
            render_pipeline,
        }
    }
}

struct Pipe {
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

type Scale = [f32; 2];
type UnifRect = (DVec2, DVec2);
fn unif_rect_as_arr(rect: UnifRect) -> [f64; 4] {
    [(rect.0).0, (rect.0).1, (rect.1).0, (rect.1).1]
}

struct PipeWindow {
    bind_group: wgpu::BindGroup,
    scale_buf: wgpu::Buffer,
    rect_buf: wgpu::Buffer,
    iter_buf: wgpu::Buffer,
    rect: UnifRect,
    iterations: i32,
    passes: Vec<Vec<Vertex>>,
}

impl CustomPipe for Pipe {
    type Window = PipeWindow;

    fn new_window(&self, device: &wgpu::Device, size: Size) -> Self::Window {
        let usage = wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST;

        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let scale_buf = device
            .create_buffer_mapped(scale_factor.len(), usage)
            .fill_from_slice(&scale_factor);

        let rect: UnifRect = (DVec2::splat(0.0), DVec2::splat(1.0));
        let rect_arr = unif_rect_as_arr(rect);
        let rect_buf = device
            .create_buffer_mapped(rect_arr.len(), usage)
            .fill_from_slice(&rect_arr);

        let iter = [64];
        let iter_buf = device
            .create_buffer_mapped(iter.len(), usage)
            .fill_from_slice(&iter);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &scale_buf,
                        range: 0..(size_of::<Scale>() as u64),
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &rect_buf,
                        range: 0..(size_of::<UnifRect>() as u64),
                    },
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &iter_buf,
                        range: 0..(size_of::<i32>() as u64),
                    },
                },
            ],
        });

        PipeWindow {
            bind_group,
            scale_buf,
            rect_buf,
            iter_buf,
            rect,
            iterations: iter[0],
            passes: vec![],
        }
    }

    fn resize(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        size: Size,
    ) {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let scale_buf = device
            .create_buffer_mapped(scale_factor.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&scale_factor);

        let byte_len = size_of::<Scale>() as u64;
        encoder.copy_buffer_to_buffer(&scale_buf, 0, &window.scale_buf, 0, byte_len);
    }

    fn update(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let rect_arr = unif_rect_as_arr(window.rect);
        let rect_buf = device
            .create_buffer_mapped(rect_arr.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&rect_arr);

        let byte_len = size_of::<UnifRect>() as u64;
        encoder.copy_buffer_to_buffer(&rect_buf, 0, &window.rect_buf, 0, byte_len);

        let iter = [window.iterations];
        let iter_buf = device
            .create_buffer_mapped(iter.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&iter);

        let byte_len = size_of::<i32>() as u64;
        encoder.copy_buffer_to_buffer(&iter_buf, 0, &window.iter_buf, 0, byte_len);
    }

    fn render(
        &self,
        window: &mut Self::Window,
        device: &wgpu::Device,
        pass: usize,
        rpass: &mut wgpu::RenderPass,
    ) {
        if pass >= window.passes.len() {
            return;
        }
        let v = &mut window.passes[pass];

        let buffer = device
            .create_buffer_mapped(v.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&v);
        let count = v.len() as u32;

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &window.bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&buffer, 0)]);
        rpass.draw(0..count, 0..1);

        v.clear();
    }
}

impl CustomWindow for PipeWindow {
    type Param = (DVec2, DVec2, f32, i32);

    fn invoke(&mut self, pass: usize, rect: Rect, p: Self::Param) {
        self.rect = (p.0, p.1);
        let rel_width = p.2;
        self.iterations = p.3;

        let aa = Vec2::from(rect.pos);
        let bb = aa + Vec2::from(rect.size);

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);

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
        self.add_vertices(pass, &[
            Vertex(aa, caa), Vertex(ba, cba), Vertex(ab, cab),
            Vertex(ab, cab), Vertex(ba, cba), Vertex(bb, cbb),
        ]);
    }
}

impl PipeWindow {
    fn add_vertices(&mut self, pass: usize, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, vec![]);
        }

        self.passes[pass].extend_from_slice(slice);
    }
}

#[widget]
#[derive(Clone, Debug, kas :: macros :: Widget)]
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

impl event::Handler for Mandlebrot {
    type Msg = ();
}

impl Layout for Mandlebrot {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, a: AxisInfo) -> SizeRules {
        let size = (match a.is_horizontal() {
            true => 300.0,
            false => 200.0,
        } * size_handle.scale_factor())
        .round() as u32;
        SizeRules::new(size, size * 3, (0, 0), StretchPolicy::Maximise)
    }

    #[inline]
    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, _align: AlignHints) {
        self.core.rect = rect;
        let size = DVec2::from(rect.size);
        let rel_width = DVec2(size.0 / size.1, 1.0);
        self.view_alpha = 2.0 / size.1;
        self.view_delta = -(DVec2::from(rect.pos) * 2.0 + size) / size.1;
        self.rel_width = rel_width.0 as f32;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &event::ManagerState) {
        let (region, offset, draw) = draw_handle.draw_device();
        // TODO: our view transform assumes that offset = 0.
        // Here it is but in general we should be able to handle an offset here!
        assert_eq!(offset, Coord::ZERO, "view transform assumption violated");

        let draw = draw
            .as_any_mut()
            .downcast_mut::<DrawWindow<PipeWindow>>()
            .unwrap();
        let p = (self.alpha, self.delta, self.rel_width, self.iter);
        draw.custom(region, self.core.rect + offset, p);
    }

    fn event(&mut self, mgr: &mut Manager, _: WidgetId, event: Event) -> Response<Self::Msg> {
        match event {
            Event::Action(event::Action::Scroll(delta)) => {
                let factor = match delta {
                    event::ScrollDelta::LineDelta(_, y) => -0.5 * y as f64,
                    event::ScrollDelta::PixelDelta(coord) => -0.01 * coord.1 as f64,
                };
                self.alpha = self.alpha * 2f64.powf(factor);
                mgr.redraw(self.id());
                Response::Msg(())
            }
            Event::Action(event::Action::Pan { alpha, delta }) => {
                // Our full transform (from screen coordinates to world coordinates) is:
                // f(p) = α_w * α_v * p + α_w * δ_v + δ_w
                // where _w indicate world transforms (self.alpha, self.delta)
                // and _v indicate view transforms (see notes in PipeWindow::invoke).
                //
                // To adjust the world offset (in reverse), we use the following formulae:
                // α_w' = (1/α) * α_w
                // δ_w' = δ_w - α_w' * α_v * δ + (α_w - α_w') δ_v
                // where x' is the "new x".
                let new_alpha = self.alpha.complex_div(alpha.into());
                self.delta = self.delta - new_alpha.complex_mul(delta.into()) * self.view_alpha
                    + (self.alpha - new_alpha).complex_mul(self.view_delta);
                self.alpha = new_alpha;

                mgr.redraw(self.id());
                Response::Msg(())
            }
            Event::PressStart { source, coord } => {
                mgr.request_grab(
                    self.id(),
                    source,
                    coord,
                    event::GrabMode::PanFull,
                    Some(event::CursorIcon::Grabbing),
                );
                Response::None
            }
            _ => Response::None,
        }
    }
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

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let mbrot = Mandlebrot::new();
    let slider = Slider::new(0, 256).with_value(64);

    let window = make_widget! {
        #[widget]
        #[layout(grid)]
        #[handler(msg = event::VoidMsg)]
        struct {
            #[widget(cspan=2)] label: Label = Label::new(mbrot.loc()),
            #[widget(row=1, halign=centre)] iters: Label = Label::new("64").reserve("000"),
            #[widget(row=2, handler = iter)] _: Slider<i32, Vertical> = slider,
            #[widget(col=1, row=1, rspan=2, handler = mbrot)] mbrot: Mandlebrot = mbrot,
        }
        impl {
            fn iter(&mut self, mgr: &mut Manager, iter: i32) -> VoidResponse {
                self.mbrot.iter = iter;
                self.label.set_text(mgr, self.mbrot.loc());
                self.iters.set_text(mgr, format!("{}", iter));
                Response::None
            }
            fn mbrot(&mut self, mgr: &mut Manager, _: ()) -> VoidResponse {
                self.label.set_text(mgr, self.mbrot.loc());
                Response::None
            }
        }
    };
    let window = Window::new("Mandlebrot", window);

    let mut theme = kas_theme::FlatTheme::new();
    theme.set_colours("dark");
    let mut toolkit = kas_wgpu::Toolkit::new_custom(PipeBuilder, theme, Options::from_env())?;
    toolkit.add(window)?;
    toolkit.run()
}
