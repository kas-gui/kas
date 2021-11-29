// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Common pipeline parts

use kas::cast::Conv;
use kas::macros::autoimpl;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::ops::Range;

#[autoimpl(Default)]
#[derive(Clone, Debug)]
struct PassData<V: bytemuck::Pod> {
    vertices: Vec<V>,
    count: u32,
    data_range: Range<u64>,
}

/// Per-window state
#[autoimpl(Default)]
pub struct Window<V: bytemuck::Pod> {
    passes: Vec<PassData<V>>,
    buffer: Option<wgpu::Buffer>,
    buffer_size: u64,
}

impl<V: bytemuck::Pod> Window<V> {
    /// Prepare vertex buffers
    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let req_len = self
            .passes
            .iter()
            .map(|pd| u64::conv(pd.vertices.len() * size_of::<V>()))
            .sum();
        let byte_len = match NonZeroU64::new(req_len) {
            Some(nz) => nz,
            None => {
                for pass in self.passes.iter_mut() {
                    pass.count = 0;
                }
                return;
            }
        };

        if req_len <= self.buffer_size {
            let buffer = self.buffer.as_ref().unwrap();
            let mut slice = staging_belt.write_buffer(encoder, buffer, 0, byte_len, device);
            copy_to_slice(&mut self.passes, &mut slice);
        } else {
            // Size must be a multiple of alignment
            let mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
            let buffer_size = (byte_len.get() + mask) & !mask;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vertex buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: true,
            });

            let mut slice = buffer.slice(..byte_len.get()).get_mapped_range_mut();
            copy_to_slice(&mut self.passes, &mut slice);
            drop(slice);

            buffer.unmap();
            self.buffer = Some(buffer);
            self.buffer_size = buffer_size;
        }

        fn copy_to_slice<V: bytemuck::Pod>(passes: &mut [PassData<V>], slice: &mut [u8]) {
            let mut byte_offset = 0;
            for pass in passes.iter_mut() {
                let len = u32::conv(pass.vertices.len());
                let byte_len = u64::from(len) * u64::conv(size_of::<V>());
                let byte_end = byte_offset + byte_len;

                slice[usize::conv(byte_offset)..usize::conv(byte_end)]
                    .copy_from_slice(bytemuck::cast_slice(&pass.vertices));

                pass.vertices.clear();
                pass.count = len;
                pass.data_range = byte_offset..byte_end;
                byte_offset = byte_end;
            }
        }
    }

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline,
        bg_common: &'a wgpu::BindGroup,
    ) {
        if let Some(buffer) = self.buffer.as_ref() {
            if let Some(pass) = self.passes.get(pass) {
                if pass.data_range.is_empty() {
                    return;
                }
                rpass.set_pipeline(pipeline);
                rpass.set_bind_group(0, bg_common, &[]);
                rpass.set_vertex_buffer(0, buffer.slice(pass.data_range.clone()));
                rpass.draw(0..pass.count, 0..1);
            }
        }
    }

    pub fn add_vertices(&mut self, pass: usize, slice: &[V]) {
        debug_assert_eq!(slice.len() % 3, 0);

        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, Default::default());
        }

        self.passes[pass].vertices.extend_from_slice(slice);
    }
}
