mod atlas_texture;
mod buffers;
mod context;
mod pipeline;
mod types;

use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDrawable,
    MTLLoadAction, MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLStoreAction,
};
use objc2_quartz_core::CAMetalDrawable;
use winit::window::Window;

use crate::font::{Atlas, Metrics};

use atlas_texture::AtlasTexture;
use buffers::{Buffers, Uniforms};
use context::Context;
use types::Pipeline;

pub struct MetalCtx {
    context: Context,
    pipeline: Pipeline,
    buffers: Buffers,
    atlas_texture: AtlasTexture,
}

impl MetalCtx {
    pub fn new(window: &Window, metrics: Metrics, atlas: &Atlas) -> Result<Self, String> {
        let context = Context::new(window)?;
        let pipeline = pipeline::create(context.device())?;

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
            pipeline,
            buffers,
            atlas_texture,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.context.resize(width, height, scale_factor);
        self.buffers.set_screen([width as f32, height as f32]);
    }

    pub fn sync_atlas(&mut self, atlas: &mut Atlas) {
        if self.atlas_texture.sync(self.context.device(), atlas) {
            self.buffers
                .set_atlas([atlas.stride() as f32, atlas.height() as f32]);
        }
    }

    pub fn upload_instances(
        &mut self,
        offsets: &[[f32; 2]],
        colors: &[[f32; 4]],
        cells: &[u32],
    ) -> Result<(), String> {
        self.buffers
            .upload_instances(self.context.device(), offsets, colors, cells)
    }

    pub fn render(&mut self) {
        let Some(drawable) = self.context.layer().nextDrawable() else {
            return;
        };

        unsafe {
            let descriptor = MTLRenderPassDescriptor::new();
            let attachment = descriptor.colorAttachments().objectAtIndexedSubscript(0);

            attachment.setTexture(Some(&drawable.texture()));
            attachment.setLoadAction(MTLLoadAction::Clear);
            attachment.setClearColor(MTLClearColor {
                red: 0.07,
                green: 0.08,
                blue: 0.1,
                alpha: 1.0,
            });
            attachment.setStoreAction(MTLStoreAction::Store);

            let Some(cmd) = self.context.queue().commandBuffer() else {
                return;
            };
            let Some(encoder) = cmd.renderCommandEncoderWithDescriptor(&descriptor) else {
                return;
            };

            encoder.setRenderPipelineState(&self.pipeline);
            self.buffers.bind(&encoder);
            self.atlas_texture.bind(&encoder);

            if self.buffers.instance_count() > 0 {
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::Triangle,
                    0,
                    self.buffers.vertex_count(),
                    self.buffers.instance_count(),
                );
            }

            encoder.endEncoding();
            cmd.commit();
            cmd.waitUntilScheduled();
            drawable.present();
        }
    }
}
