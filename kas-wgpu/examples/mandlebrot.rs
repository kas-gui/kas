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

use kas::adapter::ReserveP;
use kas::draw::Pass;
use kas::event::{self, Command};
use kas::geom::{DVec2, Vec2, Vec3};
use kas::prelude::*;
use kas::widget::{Label, Slider, Window};
use kas_wgpu::draw::{CustomPipe, CustomPipeBuilder, CustomWindow, DrawCustom, DrawWindow};
use kas_wgpu::Options;

struct Shaders {
    vertex: ShaderModule,
    fragment: ShaderModule,
}

impl Shaders {
    fn new(device: &wgpu::Device) -> Self {
        let vertex = device.create_shader_module(&include_spirv!("mandlebrot/shader.vert.spv"));
        let fragment = device.create_shader_module(&include_spirv!("mandlebrot/shader.frag.spv"));

        Shaders { vertex, fragment }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

struct PipeBuilder;

impl CustomPipeBuilder for PipeBuilder {
    type Pipe = Pipe;

    fn build(
        &mut self,
        device: &wgpu::Device,
        tex_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self::Pipe {
        // Note: real apps should compile shaders once and share between windows
        let shaders = Shaders::new(device);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vertex,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float2],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
                clamp_depth: false,
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.fragment,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: tex_format,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
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
    passes: Vec<(Vec<Vertex>, Option<Buffer>, u32)>,
}

impl CustomPipe for Pipe {
    type Window = PipeWindow;

    fn new_window(&self, device: &wgpu::Device, size: Size) -> Self::Window {
        let scale_factor: Scale = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        let scale_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&scale_factor),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let rect: UnifRect = (DVec2::splat(0.0), DVec2::splat(1.0));
        let rect_arr = unif_rect_as_arr(rect);
        let rect_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&rect_arr),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let iter = [64];
        let iter_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&iter),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &scale_buf,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &rect_buf,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &iter_buf,
                        offset: 0,
                        size: None,
                    },
                },
            ],
            label: None,
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

    fn resize(&self, window: &mut Self::Window, _: &wgpu::Device, queue: &wgpu::Queue, size: Size) {
        let scale_factor = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        queue.write_buffer(&window.scale_buf, 0, bytemuck::cast_slice(&scale_factor));
    }

    fn update(&self, window: &mut Self::Window, device: &wgpu::Device, queue: &wgpu::Queue) {
        let rect_arr = unif_rect_as_arr(window.rect);
        let iter = [window.iterations];
        queue.write_buffer(&window.rect_buf, 0, bytemuck::cast_slice(&rect_arr));
        queue.write_buffer(&window.iter_buf, 0, bytemuck::cast_slice(&iter));

        // NOTE: we prepare vertex buffers here. Due to lifetime restrictions on
        // RenderPass we cannot currently create buffers in render().
        // See https://github.com/gfx-rs/wgpu-rs/issues/188
        for pass in &mut window.passes {
            if pass.0.len() > 0 {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&pass.0),
                    usage: wgpu::BufferUsage::VERTEX,
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
    ) {
        if let Some(tuple) = window.passes.get(pass) {
            if let Some(buffer) = tuple.1.as_ref() {
                rpass.set_pipeline(&self.render_pipeline);
                rpass.set_bind_group(0, &window.bind_group, &[]);
                rpass.set_vertex_buffer(0, buffer.slice(..));
                rpass.draw(0..tuple.2, 0..1);
            }
        }
    }
}

impl CustomWindow for PipeWindow {
    type Param = (DVec2, DVec2, f32, i32);

    fn invoke(&mut self, pass: Pass, rect: Rect, p: Self::Param) {
        self.rect = (p.0, p.1);
        let rel_width = p.2;
        self.iterations = p.3;

        let aa = Vec2::from(rect.pos);
        let bb = aa + Vec2::from(rect.size);

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

#[derive(Clone, Debug, kas :: macros :: Widget)]
#[widget(config=noauto)]
#[handler(handle=noauto)]
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
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.register_nav_fallback(self.id());
    }
}

impl Layout for Mandlebrot {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, a: AxisInfo) -> SizeRules {
        let min = if a.is_horizontal() { 300.0 } else { 200.0 };
        let ideal = min * 10.0; // prefer big but not larger than screen size
        let sf = size_handle.scale_factor();
        SizeRules::new_scaled(min, ideal, 0.0, Stretch::High, sf)
    }

    #[inline]
    fn set_rect(&mut self, _: &mut Manager, rect: Rect, _: AlignHints) {
        self.core.rect = rect;
        let size = DVec2::from(rect.size);
        let rel_width = DVec2(size.0 / size.1, 1.0);
        self.view_alpha = 2.0 / size.1;
        self.view_delta = -(DVec2::from(rect.pos) * 2.0 + size) / size.1;
        self.rel_width = rel_width.0 as f32;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        let (pass, offset, draw) = draw_handle.draw_device();
        // TODO: our view transform assumes that offset = 0.
        // Here it is but in general we should be able to handle an offset here!
        assert_eq!(offset, Offset::ZERO, "view transform assumption violated");

        let draw = draw
            .as_any_mut()
            .downcast_mut::<DrawWindow<PipeWindow>>()
            .unwrap();
        let p = (self.alpha, self.delta, self.rel_width, self.iter);
        draw.custom(pass, self.core.rect + offset, p);
    }
}

impl event::Handler for Mandlebrot {
    type Msg = ();

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
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
                            _ => return Response::Unhandled,
                        };
                        self.delta = self.delta + self.alpha.complex_mul(delta);
                    }
                }
                mgr.redraw(self.id());
                Response::Msg(())
            }
            Event::Scroll(delta) => {
                let factor = match delta {
                    event::ScrollDelta::LineDelta(_, y) => -0.5 * y as f64,
                    event::ScrollDelta::PixelDelta(coord) => -0.01 * coord.1 as f64,
                };
                self.alpha = self.alpha * 2f64.powf(factor);
                mgr.redraw(self.id());
                Response::Msg(())
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
                let new_alpha = self.alpha.complex_div(alpha.into());
                self.delta = self.delta - new_alpha.complex_mul(delta.into()) * self.view_alpha
                    + (self.alpha - new_alpha).complex_mul(self.view_delta);
                self.alpha = new_alpha;

                mgr.redraw(self.id());
                Response::Msg(())
            }
            Event::PressStart { source, coord, .. } => {
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

#[derive(Debug, Widget)]
#[layout(grid)]
#[handler(msg = event::VoidMsg)]
struct MandlebrotWindow {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget(cspan = 2)]
    label: Label<String>,
    #[widget(row=1, halign=centre)]
    iters: ReserveP<Label<String>>,
    #[widget(row=2, handler = iter)]
    slider: Slider<i32, kas::dir::Up>,
    // extra col span allows use of Label's margin
    #[widget(col=1, cspan=2, row=1, rspan=2, handler = mbrot)]
    mbrot: Mandlebrot,
}
impl MandlebrotWindow {
    fn new_window() -> Window<MandlebrotWindow> {
        let slider = Slider::new(0, 256, 1).with_value(64);
        let mbrot = Mandlebrot::new();
        let w = MandlebrotWindow {
            core: Default::default(),
            layout_data: Default::default(),
            label: Label::new(mbrot.loc()),
            iters: ReserveP::new(Label::from("64"), |size_handle, axis| {
                Label::new("000").size_rules(size_handle, axis)
            }),
            slider,
            mbrot,
        };
        Window::new("Mandlebrot", w)
    }

    fn iter(&mut self, mgr: &mut Manager, iter: i32) -> Response<VoidMsg> {
        self.mbrot.iter = iter;
        *mgr |= self.iters.set_string(format!("{}", iter));
        Response::None
    }
    fn mbrot(&mut self, mgr: &mut Manager, _: ()) -> Response<VoidMsg> {
        *mgr |= self.label.set_string(self.mbrot.loc());
        Response::None
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let theme = kas_theme::FlatTheme::new().with_colours("dark");

    kas_wgpu::Toolkit::new_custom(PipeBuilder, theme, Options::from_env())?
        .with(MandlebrotWindow::new_window())?
        .run()
}
