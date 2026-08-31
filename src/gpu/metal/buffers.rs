use std::ptr::NonNull;

use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLRenderCommandEncoder, MTLResourceOptions};

use crate::layout::{BgRect, EmojiInstance, GlyphInstance, UnderlineInstance};

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
// most screens have none, so start small and let it grow
const EMOJI_INITIAL_CAPACITY: usize = 64;
const UNDERLINE_INITIAL_CAPACITY: usize = 256;

pub const FRAMES_IN_FLIGHT: usize = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Uniforms {
    pub cell: [f32; 2],
    pub screen: [f32; 2],
    pub atlas: [f32; 2],
    pub cols: u32,
    pub pad: u32,
    pub gamma: f32,
    pub contrast: f32,
    pub emoji_atlas: [f32; 2],
    pub emoji_cols: u32,
    pub emoji_pad: u32,
    // font-wide constants the underline fragment shader needs for its
    // dotted/undercurl patterns - everything else about an underline's
    // placement is baked into its instance by the layout stage
    pub underline_thickness: f32,
    pub undercurl_amplitude: f32,
}

struct InstanceBuffer {
    buffers: [Buffer; FRAMES_IN_FLIGHT],
    capacity: usize,
    count: usize,
    elem_size: usize,
}

pub struct Buffers {
    vertex: Buffer,
    vertex_count: usize,
    glyphs: InstanceBuffer,
    emoji: InstanceBuffer,
    bg: InstanceBuffer,
    underlines: InstanceBuffer,
    uniform: Buffer,
    uniforms: Uniforms,
    frame: usize,
}

impl InstanceBuffer {
    fn new<T>(device: &Device, capacity: usize) -> Result<Self, String> {
        let buffers: [Buffer; FRAMES_IN_FLIGHT] = [
            empty_buffer::<T>(device, capacity)?,
            empty_buffer::<T>(device, capacity)?,
            empty_buffer::<T>(device, capacity)?,
        ];

        Ok(Self {
            buffers,
            capacity,
            count: 0,
            elem_size: size_of::<T>(),
        })
    }

    fn upload<T>(&mut self, device: &Device, data: &[T], frame: usize) -> Result<(), String> {
        debug_assert_eq!(self.elem_size, size_of::<T>());

        self.count = data.len();

        if data.is_empty() {
            return Ok(());
        }

        if data.len() > self.capacity {
            let capacity = data.len().next_power_of_two();
            for buffer in self.buffers.iter_mut() {
                *buffer = empty_buffer::<T>(device, capacity)?;
            }

            self.capacity = capacity;
        }

        unsafe {
            let dst = self.buffers[frame].contents().cast::<T>().as_ptr();
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
            emoji: InstanceBuffer::new::<EmojiInstance>(device, EMOJI_INITIAL_CAPACITY)?,
            bg: InstanceBuffer::new::<BgRect>(device, BG_INITIAL_CAPACITY)?,
            underlines: InstanceBuffer::new::<UnderlineInstance>(
                device,
                UNDERLINE_INITIAL_CAPACITY,
            )?,
            uniform: make_buffer(device, &[uniforms])?,
            uniforms,
            frame: 0,
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

    pub fn emoji_count(&self) -> usize {
        self.emoji.count
    }

    pub fn underline_count(&self) -> usize {
        self.underlines.count
    }

    pub fn set_screen(&mut self, screen: [f32; 2]) {
        self.uniforms.screen = screen;
        self.write_uniforms();
    }

    pub fn set_atlas(&mut self, atlas: [f32; 2]) {
        self.uniforms.atlas = atlas;
        self.write_uniforms();
    }

    pub fn set_emoji_atlas(&mut self, atlas: [f32; 2], cols: u32) {
        self.uniforms.emoji_atlas = atlas;
        self.uniforms.emoji_cols = cols;
        self.write_uniforms();
    }

    pub fn upload(
        &mut self,
        device: &Device,
        glyphs: &[GlyphInstance],
        emoji: &[EmojiInstance],
        bg: &[BgRect],
        underlines: &[UnderlineInstance],
    ) -> Result<(), String> {
        self.frame = (self.frame + 1) % FRAMES_IN_FLIGHT;

        self.glyphs.upload(device, glyphs, self.frame)?;
        self.emoji.upload(device, emoji, self.frame)?;
        self.bg.upload(device, bg, self.frame)?;
        self.underlines.upload(device, underlines, self.frame)
    }

    pub fn bind_common(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.vertex), 0, 0);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.uniform), 0, 3);
            // the glyph shader reads gamma/contrast in the fragment stage
            encoder.setFragmentBuffer_offset_atIndex(Some(&self.uniform), 0, 3);
        }
    }

    pub fn bind_glyphs(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.glyphs.buffers[self.frame]), 0, 1);
        }
    }

    pub fn bind_emoji(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.emoji.buffers[self.frame]), 0, 1);
        }
    }

    pub fn bind_bg(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.bg.buffers[self.frame]), 0, 1);
        }
    }

    pub fn bind_underlines(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(
                Some(&self.underlines.buffers[self.frame]),
                0,
                1,
            );
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
