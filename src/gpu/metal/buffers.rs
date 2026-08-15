use std::ptr::NonNull;

use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLRenderCommandEncoder, MTLResourceOptions};

use super::types::{Buffer, Device};

const QUAD_POSITIONS: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

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
    offsets: Buffer,
    colors: Buffer,
    cells: Buffer,
    instance_count: usize,
    uniform: Buffer,
    uniforms: Uniforms,
}

impl Buffers {
    pub fn new(device: &Device, uniforms: Uniforms) -> Result<Self, String> {
        Ok(Self {
            vertex: make_buffer(device, &QUAD_POSITIONS)?,
            vertex_count: QUAD_POSITIONS.len(),
            offsets: make_buffer(device, &[[0.0f32, 0.0]])?,
            colors: make_buffer(device, &[[0.0f32; 4]])?,
            cells: make_buffer(device, &[0u32])?,
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
        offsets: &[[f32; 2]],
        colors: &[[f32; 4]],
        cells: &[u32],
    ) -> Result<(), String> {
        debug_assert_eq!(offsets.len(), colors.len());
        debug_assert_eq!(offsets.len(), cells.len());

        if offsets.is_empty() {
            self.instance_count = 0;
            return Ok(());
        }

        self.offsets = make_buffer(device, offsets)?;
        self.colors = make_buffer(device, colors)?;
        self.cells = make_buffer(device, cells)?;
        self.instance_count = offsets.len();

        Ok(())
    }

    pub fn bind(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.vertex), 0, 0);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.offsets), 0, 1);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.colors), 0, 2);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.uniform), 0, 3);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.cells), 0, 4);
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

pub fn make_buffer<T>(device: &Device, data: &[T]) -> Result<Buffer, String> {
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
