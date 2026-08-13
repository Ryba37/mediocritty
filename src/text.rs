use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};

pub struct TextCtx {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffer: Buffer,
}

impl TextCtx {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        cols: usize,
        rows: usize,
    ) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        let metrics = Metrics::new(16.0, 20.0);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(Some(cols as f32 * 10.0), Some(rows as f32 * 20.0));

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            renderer,
            buffer,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.buffer.set_text(
            text,
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
            None,
        );

        self.buffer.shape_until_scroll(&mut self.font_system, false);
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        self.viewport.update(queue, Resolution { width, height });

        self.renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [TextArea {
                    buffer: &self.buffer,
                    left: 0.0,
                    top: 0.0,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: width as i32,
                        bottom: height as i32,
                    },
                    default_color: glyphon::Color::rgb(220, 220, 220),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .unwrap();
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .unwrap();
    }

    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }
}
