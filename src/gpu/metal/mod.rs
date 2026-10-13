use block2::RcBlock;
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDrawable,
    MTLLoadAction, MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLStoreAction,
};
use objc2_quartz_core::{CAMetalDrawable, CATransaction};
use winit::window::Window;

use crate::{
    color::clear_color,
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
    // pipelines bake the opacity in (pixel format, GAMMA_WEIGHT), so they
    // are rebuilt only when it changes
    opacity: f32,
    buffers: Buffers,
    atlas_texture: AtlasTexture,
    emoji_texture: AtlasTexture,
    background: [f64; 4],
    frames: Frames,
}

struct Pipelines {
    glyph: Pipeline,
    emoji: Pipeline,
    bg: Pipeline,
    underline: Pipeline,
}

fn pipelines(device: &types::Device, opacity: f32) -> Result<Pipelines, String> {
    Ok(Pipelines {
        glyph: pipeline::glyph(device, opacity)?,
        emoji: pipeline::emoji(device, opacity)?,
        bg: pipeline::bg(device, opacity)?,
        underline: pipeline::underline(device, opacity)?,
    })
}

fn text_uniforms(config: &Config) -> (f32, f32) {
    (
        1.0 / config.font.gamma.max(0.01),
        1.0 + config.font.contrast.clamp(0.0, 100.0) * 0.01,
    )
}

impl MetalCtx {
    pub fn new(
        window: &Window,
        metrics: Metrics,
        atlas: &mut Atlas,
        emoji: &mut Atlas,
        config: &Config,
    ) -> Result<Self, String> {
        let opacity = config.theme.opacity;
        let context = Context::new(window, config.theme.background, opacity, config.theme.blur)?;
        let p = pipelines(context.device(), opacity)?;

        let size = window.inner_size();
        let (gamma, contrast) = text_uniforms(config);
        let uniforms = Uniforms {
            cell: [metrics.cell_width as f32, metrics.cell_height as f32],
            screen: [size.width as f32, size.height as f32],
            atlas: [atlas.width() as f32, atlas.height() as f32],
            cols: atlas.cols(),
            pad: 0,
            gamma,
            contrast,
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
            glyph_pipeline: p.glyph,
            emoji_pipeline: p.emoji,
            bg_pipeline: p.bg,
            underline_pipeline: p.underline,
            opacity,
            buffers,
            atlas_texture,
            emoji_texture,
            background: clear_color(config.theme.background.0, opacity),
            frames: Frames::new(FRAMES_IN_FLIGHT as isize),
        })
    }

    // config reload: keeps device, queue, layer and buffers, swaps only what
    // the new config touches. new atlases are picked up by the next sync
    pub fn reconfigure(
        &mut self,
        window: &Window,
        metrics: Metrics,
        config: &Config,
    ) -> Result<(), String> {
        let opacity = config.theme.opacity;

        if opacity != self.opacity {
            let p = pipelines(self.context.device(), opacity)?;
            self.glyph_pipeline = p.glyph;
            self.emoji_pipeline = p.emoji;
            self.bg_pipeline = p.bg;
            self.underline_pipeline = p.underline;
            self.opacity = opacity;
        }

        self.context
            .apply_appearance(window, config.theme.background, opacity, config.theme.blur);
        self.background = clear_color(config.theme.background.0, opacity);

        let (gamma, contrast) = text_uniforms(config);
        self.buffers.set_text(gamma, contrast);
        self.buffers.set_metrics(metrics);

        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.context.resize(width, height, scale_factor);
        self.buffers.set_screen([width as f32, height as f32]);
    }

    fn sync_atlas(&mut self, atlas: &mut Atlas, emoji: &mut Atlas) {
        if self.atlas_texture.sync(self.context.device(), atlas) {
            self.buffers
                .set_atlas([atlas.width() as f32, atlas.height() as f32], atlas.cols());
        }

        if self.emoji_texture.sync(self.context.device(), emoji) {
            self.buffers
                .set_emoji_atlas([emoji.width() as f32, emoji.height() as f32], emoji.cols());
        }
    }

    fn upload(&mut self, frame: &Frame) -> Result<usize, String> {
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

        // drawable before upload: a frame that bails out must not touch the
        // instance buffers. the slot index only advances on commit, so a
        // skipped frame never leaves a buffer the gpu still reads one step
        // closer to being overwritten
        let Some(drawable) = self.context.layer().nextDrawable() else {
            return;
        };

        let slot = match self.upload(frame) {
            Ok(slot) => slot,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

        unsafe {
            let descriptor = MTLRenderPassDescriptor::new();
            let attachment = descriptor.colorAttachments().objectAtIndexedSubscript(0);

            attachment.setTexture(Some(&drawable.texture()));
            attachment.setLoadAction(MTLLoadAction::Clear);
            attachment.setClearColor(MTLClearColor {
                red: self.background[0],
                green: self.background[1],
                blue: self.background[2],
                alpha: self.background[3],
            });
            attachment.setStoreAction(MTLStoreAction::Store);

            let Some(cmd) = self.context.queue().commandBuffer() else {
                return;
            };
            let Some(encoder) = cmd.renderCommandEncoderWithDescriptor(&descriptor) else {
                return;
            };

            self.buffers.bind_common(&encoder);
            self.buffers.set_frame(slot);

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

            CATransaction::begin();
            CATransaction::setDisableActions(true);
            drawable.present();
            CATransaction::commit();
        }
    }

    pub fn update_metrics(&mut self, metrics: Metrics) {
        self.buffers.set_metrics(metrics);
    }
}
