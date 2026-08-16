use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDrawable,
    MTLLoadAction, MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLStoreAction,
};
use objc2_quartz_core::CAMetalDrawable;
use winit::window::Window;

use crate::{
    color::srgb_to_linear,
    font::{Atlas, Metrics},
    layout::Frame,
};

use atlas_texture::AtlasTexture;
use buffers::{Buffers, Uniforms};
use context::Context;
use types::Pipeline;

mod atlas_texture;
mod buffers;
mod context;
mod pipeline;
mod types;

const BG: [f32; 3] = [0.07, 0.08, 0.10];

pub struct MetalCtx {
    context: Context,
    glyph_pipeline: Pipeline,
    bg_pipeline: Pipeline,
    buffers: Buffers,
    atlas_texture: AtlasTexture,
}

impl MetalCtx {
    pub fn new(window: &Window, metrics: Metrics, atlas: &Atlas) -> Result<Self, String> {
        let context = Context::new(window)?;
        let glyph_pipeline = pipeline::glyph(context.device())?;
        let bg_pipeline = pipeline::bg(context.device())?;

        let size = window.inner_size();
        let uniforms = Uniforms {
            cell: [metrics.cell_width as f32, metrics.cell_height as f32],
            screen: [size.width as f32, size.height as f32],
            atlas: [atlas.stride() as f32, atlas.height() as f32],
            cols: atlas.cols(),
            pad: 0,
        };

        let buffers = Buffers::new(context.device(), uniforms)?;
        let atlas_texture = AtlasTexture::new(context.device(), atlas)?;

        Ok(Self {
            context,
            glyph_pipeline,
            bg_pipeline,
            buffers,
            atlas_texture,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.context.resize(width, height, scale_factor);
        self.buffers.set_screen([width as f32, height as f32]);
    }

    fn sync_atlas(&mut self, atlas: &mut Atlas) {
        if self.atlas_texture.sync(self.context.device(), atlas) {
            self.buffers
                .set_atlas([atlas.stride() as f32, atlas.height() as f32]);
        }
    }

    fn upload(&mut self, frame: &Frame) -> Result<(), String> {
        self.buffers
            .upload(self.context.device(), frame.glyphs, frame.bg)
    }

    pub fn render(&mut self, frame: &Frame, atlas: &mut Atlas) {
        self.sync_atlas(atlas);
        if let Err(e) = self.upload(frame) {
            eprintln!("{e}");
            return;
        }

        let Some(drawable) = self.context.layer().nextDrawable() else {
            return;
        };

        unsafe {
            let descriptor = MTLRenderPassDescriptor::new();
            let attachment = descriptor.colorAttachments().objectAtIndexedSubscript(0);

            attachment.setTexture(Some(&drawable.texture()));
            attachment.setLoadAction(MTLLoadAction::Clear);
            attachment.setClearColor(MTLClearColor {
                red: srgb_to_linear(BG[0]) as f64,
                green: srgb_to_linear(BG[1]) as f64,
                blue: srgb_to_linear(BG[2]) as f64,
                alpha: 1.0,
            });
            attachment.setStoreAction(MTLStoreAction::Store);

            let Some(cmd) = self.context.queue().commandBuffer() else {
                return;
            };
            let Some(encoder) = cmd.renderCommandEncoderWithDescriptor(&descriptor) else {
                return;
            };

            self.buffers.bind_common(&encoder);

            if self.buffers.bg_count() > 0 {
                encoder.setRenderPipelineState(&self.bg_pipeline);
                self.buffers.bind_bg(&encoder);
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::Triangle,
                    0,
                    self.buffers.vertex_count(),
                    self.buffers.bg_count(),
                );
            }

            if self.buffers.glyph_count() > 0 {
                encoder.setRenderPipelineState(&self.glyph_pipeline);
                self.buffers.bind_glyphs(&encoder);
                self.atlas_texture.bind(&encoder);
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::Triangle,
                    0,
                    self.buffers.vertex_count(),
                    self.buffers.glyph_count(),
                );
            }

            encoder.endEncoding();
            cmd.commit();
            cmd.waitUntilScheduled();
            drawable.present();
        }
    }
}
