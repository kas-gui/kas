// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use std::f32;
use std::mem::size_of;

use wgpu_glyph::{
    GlyphBrush, GlyphCruncher, HorizontalAlign, Layout, Scale, Section, VerticalAlign,
};

use kas::class::{Align, Class};
use kas::geom::{AxisInfo, Coord, Margins, Size, SizeRules};
use kas::{event, TkAction, TkWindow, Widget};

/// Font size (units are half-point sizes?)
const FONT_SIZE: f32 = 20.0;
/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;
/// Frame size (adjusted as above)
const FRAME_SIZE: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
struct VertPos(f32, f32);

#[derive(Clone, Copy, Debug)]
struct VertCol(f32, f32, f32);

#[derive(Clone, Copy, Debug)]
struct Vertex(VertPos, VertCol);

struct TriBuffer {
    v: Vec<Vertex>,
}

impl TriBuffer {
    fn new() -> Self {
        TriBuffer { v: vec![] }
    }

    fn clear(&mut self) {
        self.v.clear();
    }

    fn create_buffer(&self, device: &wgpu::Device) -> (wgpu::Buffer, u32) {
        let buffer = device
            .create_buffer_mapped(self.v.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&self.v);
        let count = self.v.len() as u32;
        (buffer, count)
    }

    fn add_quad(&mut self, aa: VertPos, bb: VertPos, col: VertCol) {
        let ab = VertPos(aa.0, bb.1);
        let ba = VertPos(bb.0, aa.1);

        #[rustfmt::skip]
        self.v.extend_from_slice(&[
            Vertex(aa, col), Vertex(ba, col), Vertex(ab, col),
            Vertex(ab, col), Vertex(ba, col), Vertex(bb, col),
        ]);
    }
}

/// Widget renderer
pub(crate) struct Widgets {
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    tri_buffer: TriBuffer,
    pub(crate) glyph_brush: GlyphBrush<'static, ()>,
    font_scale: f32,
    margin: f32,
    frame_size: f32,
    action: TkAction,
    pub(crate) ev_mgr: event::ManagerData,
}

pub fn read_glsl(code: &str, stage: glsl_to_spirv::ShaderType) -> Vec<u32> {
    wgpu::read_spirv(glsl_to_spirv::compile(&code, stage).unwrap()).unwrap()
}

impl Widgets {
    pub fn new(
        device: &wgpu::Device,
        size: Size,
        dpi_factor: f64,
        glyph_brush: GlyphBrush<'static, ()>,
    ) -> Self {
        let vs_bytes = read_glsl(
            include_str!("shaders/tri_buffer.vert"),
            glsl_to_spirv::ShaderType::Vertex,
        );
        let fs_bytes = read_glsl(
            include_str!("shaders/tri_buffer.frag"),
            glsl_to_spirv::ShaderType::Fragment,
        );

        let vs_module = device.create_shader_module(&vs_bytes);
        let fs_module = device.create_shader_module(&fs_bytes);

        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let uniform_buf = device
            .create_buffer_mapped(
                scale_factor.len(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&scale_factor);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniform_buf,
                    range: 0..(size_of::<Scale>() as u64),
                },
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
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
                        format: wgpu::VertexFormat::Float3,
                        offset: size_of::<VertPos>() as u64,
                        shader_location: 1,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let dpi = dpi_factor as f32;
        Widgets {
            bind_group,
            uniform_buf,
            render_pipeline,
            tri_buffer: TriBuffer::new(),
            glyph_brush,
            font_scale: (FONT_SIZE * dpi).round(),
            margin: (MARGIN * dpi).round(),
            frame_size: (FRAME_SIZE * dpi).round(),
            action: TkAction::None,
            ev_mgr: event::ManagerData::new(dpi_factor),
        }
    }

    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        let dpi = dpi_factor as f32;
        self.font_scale = (FONT_SIZE * dpi).round();
        self.margin = (MARGIN * dpi).round();
        self.frame_size = (FRAME_SIZE * dpi).round();
        self.ev_mgr.set_dpi_factor(dpi_factor);
        // Note: we rely on caller to resize widget
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let uniform_buf = device
            .create_buffer_mapped(scale_factor.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&scale_factor);
        let byte_len = size_of::<Scale>() as u64;

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        encoder.copy_buffer_to_buffer(&uniform_buf, 0, &self.uniform_buf, 0, byte_len);
        encoder.finish()
    }

    #[inline]
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        rpass: &mut wgpu::RenderPass,
        win: &dyn kas::Window,
    ) {
        self.tri_buffer.clear();

        self.draw_iter(win.as_widget());

        let (buffer, count) = self.tri_buffer.create_buffer(device);

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&buffer, 0)]);
        rpass.draw(0..count, 0..1);
    }

    fn draw_iter(&mut self, widget: &dyn kas::Widget) {
        // draw widget; recurse over children
        self.draw_widget(widget);

        for n in 0..widget.len() {
            self.draw_iter(widget.get(n).unwrap());
        }
    }

    fn draw_widget(&mut self, widget: &dyn kas::Widget) {
        // This is a hacky draw routine just to show where widgets are.
        let w_id = widget.id();

        // Note: coordinates place the origin at the top-left.
        let rect = widget.rect();
        let (mut x0, mut y0) = rect.pos_f32();
        let (w, h) = rect.size_f32();
        let (mut x1, mut y1) = (x0 + w, y0 + h);

        let mut background = VertCol(0.1, 0.1, 0.1);

        let margin = self.margin;
        let scale = Scale::uniform(self.font_scale);
        let mut bounds = (x1 - x0 - margin - margin, y1 - y0 - margin - margin);

        let alignments = widget.class().alignments();
        // TODO: support justified alignment
        let (h_align, h_offset) = match alignments.1 {
            Align::Begin | Align::Justify => (HorizontalAlign::Left, 0.0),
            Align::Center => (HorizontalAlign::Center, 0.5 * bounds.0),
            Align::End => (HorizontalAlign::Right, bounds.0),
        };
        let (v_align, v_offset) = match alignments.1 {
            Align::Begin | Align::Justify => (VerticalAlign::Top, 0.0),
            Align::Center => (VerticalAlign::Center, 0.5 * bounds.1),
            Align::End => (VerticalAlign::Bottom, bounds.1),
        };
        let layout = Layout::default().h_align(h_align).v_align(v_align);
        let mut text_pos = (x0 + margin + h_offset, y0 + margin + v_offset);

        match widget.class() {
            Class::Container | Class::Window => {
                // do not draw containers
                return;
            }
            Class::Label(cls) => {
                let color = [1.0, 1.0, 1.0, 1.0];
                let section = Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                };
                self.glyph_brush.queue(section);
            }
            Class::Entry(cls) => {
                let f = self.frame_size;
                /*
                batch.add(Shape::Rectangle(
                    Rect::new(x0, y0, x1, y1),
                    Stroke::new(f, VertCol(0.6, 0.6, 0.6)),
                    Fill::Empty(),
                ));
                */
                x0 += f;
                x1 -= f;
                y0 += f;
                y1 -= f;
                bounds.0 -= 2.0 * f;
                bounds.1 -= 2.0 * f;
                text_pos.0 += f;
                text_pos.1 += f;

                let mut text = cls.get_text().to_string();
                if self.ev_mgr.key_grab(w_id) {
                    // TODO: proper edit character and positioning
                    text.push('|');
                }
                background = VertCol(1.0, 1.0, 1.0);
                let color = [0.0, 0.0, 0.0, 1.0];
                self.glyph_brush.queue(Section {
                    text: &text,
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                });
            }
            Class::Button(cls) => {
                background = VertCol(0.2, 0.7, 1.0);
                if self.ev_mgr.is_depressed(w_id) {
                    background = VertCol(0.2, 0.6, 0.8);
                } else if self.ev_mgr.is_hovered(w_id) {
                    background = VertCol(0.25, 0.8, 1.0);
                }
                let color = [1.0, 1.0, 1.0, 1.0];
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                });
            }
            Class::CheckBox(cls) => {
                let f = self.frame_size;
                /*
                batch.add(Shape::Rectangle(
                    Rect::new(x0, y0, x1, y1),
                    Stroke::new(f, VertCol(0.6, 0.6, 0.6)),
                    Fill::Empty(),
                ));
                */
                x0 += f;
                x1 -= f;
                y0 += f;
                y1 -= f;
                bounds.0 -= 2.0 * f;
                bounds.1 -= 2.0 * f;
                text_pos.0 += f;
                text_pos.1 += f;

                background = VertCol(1.0, 1.0, 1.0);
                let color = [0.0, 0.0, 0.0, 1.0];
                // TODO: draw check mark *and* optional text
                // let text = if cls.get_bool() { "✓" } else { "" };
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                });
            }
            Class::Frame => {
                /*
                batch.add(Shape::Rectangle(
                    Rect::new(x0, y0, x1, y1),
                    Stroke::new(self.frame_size, VertCol(0.6, 0.6, 0.6)),
                    Fill::Empty(),
                ));
                */
            }
        }

        // draw any highlights within the margin area
        // note: may be disabled via 'false &&' prefix
        let hover = false && self.ev_mgr.is_hovered(w_id);
        let key_focus = self.ev_mgr.key_focus(w_id);
        let key_grab = self.ev_mgr.key_grab(w_id);
        /*
        let mut stroke = Stroke::NONE;
        if hover || key_focus || key_grab {
            let mut frame = VertCol(0.7, 0.7, 0.7);
            if hover {
                frame.g = 1.0;
            }
            if key_focus {
                frame.r = 1.0;
            }
            if key_grab {
                frame.b = 1.0;
            }
            stroke = Stroke::new(margin, frame);
        }
        */
        self.tri_buffer
            .add_quad(VertPos(x0, y0), VertPos(x1, y1), background);
    }
}

impl TkWindow for Widgets {
    fn data(&self) -> &event::ManagerData {
        &self.ev_mgr
    }

    fn update_data(&mut self, f: &mut dyn FnMut(&mut event::ManagerData) -> bool) {
        if f(&mut self.ev_mgr) {
            self.send_action(TkAction::Redraw);
        }
    }

    fn size_rules(&mut self, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        let line_height = self.font_scale as u32;
        let bound = |s: &mut Widgets, vert: bool| -> u32 {
            let bounds = widget.class().text().and_then(|text| {
                let mut bounds = (f32::INFINITY, f32::INFINITY);
                if let Some(size) = axis.fixed(false) {
                    bounds.1 = size as f32;
                } else if let Some(size) = axis.fixed(true) {
                    bounds.0 = size as f32;
                }
                s.glyph_brush.glyph_bounds(Section {
                    text,
                    screen_position: (0.0, 0.0),
                    scale: Scale::uniform(s.font_scale),
                    bounds,
                    ..Section::default()
                })
            });

            bounds
                .map(|rect| match vert {
                    false => rect.max.x - rect.min.x,
                    true => rect.max.y - rect.min.y,
                } as u32)
                .unwrap_or(0)
        };

        let size = match widget.class() {
            Class::Container | Class::Frame | Class::Window => SizeRules::EMPTY, // not important
            Class::Label(_) => {
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(self, false).max(min))
                } else {
                    SizeRules::variable(line_height, bound(self, true).max(line_height))
                }
            }
            Class::Entry(_) => {
                let frame = 2 * self.frame_size as u32;
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(self, false).max(min)) + frame
                } else {
                    SizeRules::fixed(line_height + frame)
                }
            }
            Class::Button(_) => {
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(self, false).max(min))
                } else {
                    SizeRules::fixed(line_height)
                }
            }
            Class::CheckBox(_) => {
                let frame = 2 * self.frame_size as u32;
                SizeRules::fixed(line_height + frame)
            }
        };

        size + SizeRules::fixed(self.margin as u32 * 2)
    }

    fn margins(&self, widget: &dyn Widget) -> Margins {
        match widget.class() {
            Class::Frame => {
                let off = self.frame_size as i32;
                let size = 2 * off as u32;
                Margins {
                    outer: Size(size, size),
                    offset: Coord(off, off),
                    inner: Coord::ZERO,
                }
            }
            _ => Margins {
                outer: Size::ZERO,
                offset: Coord::ZERO,
                inner: Coord::ZERO,
            },
        }
    }

    #[inline]
    fn redraw(&mut self, _: &dyn Widget) {
        self.send_action(TkAction::Redraw);
    }

    #[inline]
    fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }
}
