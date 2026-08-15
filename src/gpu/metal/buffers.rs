use std::ptr::NonNull;

use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLRenderCommandEncoder, MTLResourceOptions};

use crate::layout::{BgRect, GlyphInstance};

use super::types::{Buffer, Device};

const QUAD_POSITIONS: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

const INITIAL_CAPACITY: usize = 4096;
const BG_INITIAL_CAPACITY: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Uniforms {
    pub cell: [f32; 2],
    pub screen: [f32; 2],
    pub atlas: [f32; 2],
    pub cols: u32,
    pub pad: u32,
}

struct InstanceBuffer {
    buffer: Buffer,
    capacity: usize,
    count: usize,
    elem_size: usize,
}

pub struct Buffers {
    vertex: Buffer,
    vertex_count: usize,
    glyphs: InstanceBuffer,
    bg: InstanceBuffer,
    uniform: Buffer,
    uniforms: Uniforms,
}

impl InstanceBuffer {
    fn new<T>(device: &Device, capacity: usize) -> Result<Self, String> {
        Ok(Self {
            buffer: empty_buffer::<T>(device, capacity)?,
            capacity,
            count: 0,
            elem_size: size_of::<T>(),
        })
    }

    fn upload<T>(&mut self, device: &Device, data: &[T]) -> Result<(), String> {
        debug_assert_eq!(self.elem_size, size_of::<T>());

        self.count = data.len();

        if data.is_empty() {
            return Ok(());
        }

        if data.len() > self.capacity {
            let capacity = data.len().next_power_of_two();
            self.buffer = empty_buffer::<T>(device, capacity)?;
            self.capacity = capacity;
        }

        unsafe {
            let dst = self.buffer.contents().cast::<T>().as_ptr();
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }

        Ok(())
    }
}

impl Buffers {
    pub fn new(device: &Device, uniforms: Uniforms) -> Result<Self, String> {
        Ok(Self {
            vertex: make_buffer(device, &QUAD_POSITIONS)?,
            vertex_count: QUAD_POSITIONS.len(),
            glyphs: InstanceBuffer::new::<GlyphInstance>(device, INITIAL_CAPACITY)?,
            bg: InstanceBuffer::new::<BgRect>(device, BG_INITIAL_CAPACITY)?,
            uniform: make_buffer(device, &[uniforms])?,
            uniforms,
        })
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.count
    }

    pub fn bg_count(&self) -> usize {
        self.bg.count
    }

    pub fn set_screen(&mut self, screen: [f32; 2]) {
        self.uniforms.screen = screen;
        self.write_uniforms();
    }

    pub fn set_atlas(&mut self, atlas: [f32; 2]) {
        self.uniforms.atlas = atlas;
        self.write_uniforms();
    }

    pub fn upload(
        &mut self,
        device: &Device,
        glyphs: &[GlyphInstance],
        bg: &[BgRect],
    ) -> Result<(), String> {
        self.glyphs.upload(device, glyphs)?;
        self.bg.upload(device, bg)
    }

    pub fn bind_common(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.vertex), 0, 0);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.uniform), 0, 3);
        }
    }

    pub fn bind_glyphs(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.glyphs.buffer), 0, 1);
        }
    }

    pub fn bind_bg(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.bg.buffer), 0, 1);
        }
    }

    fn write_uniforms(&self) {
        unsafe {
            self.uniform
                .contents()
                .cast::<Uniforms>()
                .write(self.uniforms);
        }
    }
}

fn make_buffer<T>(device: &Device, data: &[T]) -> Result<Buffer, String> {
    if data.is_empty() {
        return Err("empty buffer".to_string());
    }

    unsafe {
        device.newBufferWithBytes_length_options(
            NonNull::from(data).cast(),
            size_of_val(data),
            MTLResourceOptions::StorageModeShared,
        )
    }
    .ok_or_else(|| "couldn't create buffer".to_string())
}

fn empty_buffer<T>(device: &Device, capacity: usize) -> Result<Buffer, String> {
    device
        .newBufferWithLength_options(
            size_of::<T>() * capacity,
            MTLResourceOptions::StorageModeShared,
        )
        .ok_or_else(|| "couldn't create buffer".to_string())
}
