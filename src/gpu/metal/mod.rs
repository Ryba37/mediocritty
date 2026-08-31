use block2::RcBlock;
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDrawable,
    MTLLoadAction, MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLStoreAction,
};
use objc2_quartz_core::CAMetalDrawable;
use winit::window::Window;

use crate::{
    color::linear,
    config::Config,
    font::{Atlas, Metrics},
    gpu::metal::{
        buffers::FRAMES_IN_FLIGHT,
        sync::{Frames, Guards},
    },
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
mod sync;
mod types;

pub struct MetalCtx {
    context: Context,
    glyph_pipeline: Pipeline,
    emoji_pipeline: Pipeline,
    bg_pipeline: Pipeline,
    underline_pipeline: Pipeline,
    buffers: Buffers,
    atlas_texture: AtlasTexture,
    emoji_texture: AtlasTexture,
    background: [f32; 3],
    frames: Frames,
}

impl MetalCtx {
    pub fn new(
        window: &Window,
        metrics: Metrics,
        atlas: &Atlas,
        emoji: &Atlas,
        config: &Config,
    ) -> Result<Self, String> {
        let context = Context::new(window)?;
        let glyph_pipeline = pipeline::glyph(context.device())?;
        let emoji_pipeline = pipeline::emoji(context.device())?;
        let bg_pipeline = pipeline::bg(context.device())?;
        let underline_pipeline = pipeline::underline(context.device())?;

        let size = window.inner_size();
        let uniforms = Uniforms {
            cell: [metrics.cell_width as f32, metrics.cell_height as f32],
            screen: [size.width as f32, size.height as f32],
            atlas: [atlas.width() as f32, atlas.height() as f32],
            cols: atlas.cols(),
            pad: 0,
            gamma: config.font.gamma.max(0.01),
            contrast: 1.0 + config.font.contrast.clamp(0.0, 100.0) * 0.01,
            emoji_atlas: [emoji.width() as f32, emoji.height() as f32],
            emoji_cols: emoji.cols(),
            emoji_pad: 0,
            underline_thickness: metrics.underline_thickness,
            // half the descent, same as alacritty: the curl swings through
            // the whole descent band without spilling past it
            undercurl_amplitude: 0.5 * (metrics.cell_height as f32 - metrics.ascent),
        };

        let buffers = Buffers::new(context.device(), uniforms)?;
        let atlas_texture = AtlasTexture::new(context.device(), atlas)?;
        let emoji_texture = AtlasTexture::new(context.device(), emoji)?;

        Ok(Self {
            context,
            glyph_pipeline,
            emoji_pipeline,
            bg_pipeline,
            underline_pipeline,
            buffers,
            atlas_texture,
            emoji_texture,
            background: linear_background(config.theme.background.0),
            frames: Frames::new(FRAMES_IN_FLIGHT as isize),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.context.resize(width, height, scale_factor);
        self.buffers.set_screen([width as f32, height as f32]);
    }

    fn sync_atlas(&mut self, atlas: &mut Atlas, emoji: &mut Atlas) {
        if self.atlas_texture.sync(self.context.device(), atlas) {
            self.buffers
                .set_atlas([atlas.width() as f32, atlas.height() as f32]);
        }

        if self.emoji_texture.sync(self.context.device(), emoji) {
            self.buffers
                .set_emoji_atlas([emoji.width() as f32, emoji.height() as f32], emoji.cols());
        }
    }

    fn upload(&mut self, frame: &Frame) -> Result<(), String> {
        self.buffers.upload(
            self.context.device(),
            frame.glyphs,
            frame.emoji,
            frame.bg,
            frame.underlines,
        )
    }

    pub fn render(&mut self, frame: &Frame, atlas: &mut Atlas, emoji: &mut Atlas) {
        let mut guard = Guards::new(&self.frames);

        self.sync_atlas(atlas, emoji);
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
                red: self.background[0] as f64,
                green: self.background[1] as f64,
                blue: self.background[2] as f64,
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

            // last: emoji blend over whatever the mask pass left, and the pass
            // costs nothing on a screen without any
            if self.buffers.emoji_count() > 0 {
                encoder.setRenderPipelineState(&self.emoji_pipeline);
                self.buffers.bind_emoji(&encoder);
                self.emoji_texture.bind(&encoder);
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::Triangle,
                    0,
                    self.buffers.vertex_count(),
                    self.buffers.emoji_count(),
                );
            }

            // last of all: underlines paint over glyph descenders same as a
            // real terminal, and undercurl/dotted need to blend their
            // fragment-computed pattern over whatever's already there
            if self.buffers.underline_count() > 0 {
                encoder.setRenderPipelineState(&self.underline_pipeline);
                self.buffers.bind_underlines(&encoder);
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::Triangle,
                    0,
                    self.buffers.vertex_count(),
                    self.buffers.underline_count(),
                );
            }

            encoder.endEncoding();
            let semaphore = self.frames.handle();
            let block = RcBlock::new(move |_| {
                semaphore.signal();
            });
            cmd.addCompletedHandler(RcBlock::as_ptr(&block));
            guard.release();
            cmd.commit();
            cmd.waitUntilScheduled();
            drawable.present();
        }
    }
}

fn linear_background(rgb: [u8; 3]) -> [f32; 3] {
    [linear(rgb[0]), linear(rgb[1]), linear(rgb[2])]
}
