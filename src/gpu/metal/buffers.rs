use std::ptr::NonNull;

use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLRenderCommandEncoder, MTLResourceOptions};

use crate::layout::GlyphInstance;

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Uniforms {
    pub cell: [f32; 2],
    pub screen: [f32; 2],
    pub atlas: [f32; 2],
    pub cols: u32,
    pub pad: u32,
}

pub struct Buffers {
    vertex: Buffer,
    vertex_count: usize,
    instances: Buffer,
    capacity: usize,
    instance_count: usize,
    uniform: Buffer,
    uniforms: Uniforms,
}

impl Buffers {
    pub fn new(device: &Device, uniforms: Uniforms) -> Result<Self, String> {
        Ok(Self {
            vertex: make_buffer(device, &QUAD_POSITIONS)?,
            vertex_count: QUAD_POSITIONS.len(),
            instances: empty_buffer::<GlyphInstance>(device, INITIAL_CAPACITY)?,
            capacity: INITIAL_CAPACITY,
            instance_count: 0,
            uniform: make_buffer(device, &[uniforms])?,
            uniforms,
        })
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn instance_count(&self) -> usize {
        self.instance_count
    }

    pub fn set_screen(&mut self, screen: [f32; 2]) {
        self.uniforms.screen = screen;
        self.write_uniforms();
    }

    pub fn set_atlas(&mut self, atlas: [f32; 2]) {
        self.uniforms.atlas = atlas;
        self.write_uniforms();
    }

    pub fn upload_instances(
        &mut self,
        device: &Device,
        instances: &[GlyphInstance],
    ) -> Result<(), String> {
        self.instance_count = instances.len();

        if instances.is_empty() {
            return Ok(());
        }

        if instances.len() > self.capacity {
            let capacity = instances.len().next_power_of_two();
            self.instances = empty_buffer::<GlyphInstance>(device, capacity)?;
            self.capacity = capacity;
        }

        unsafe {
            let dst = self.instances.contents().cast::<GlyphInstance>().as_ptr();
            std::ptr::copy_nonoverlapping(instances.as_ptr(), dst, instances.len());
        }

        Ok(())
    }

    pub fn bind(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.vertex), 0, 0);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.instances), 0, 1);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.uniform), 0, 3);
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
