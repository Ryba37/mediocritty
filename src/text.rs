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
    cell_width: f32,
    cell_height: f32,
    cols: usize,
    rows: usize,
}

impl TextCtx {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        let metrics = Metrics::new(16.0, 20.0);
        let (cell_width, cell_height) = measure_cell(&mut font_system, metrics);

        let cols = ((width as f32 / cell_width).floor() as usize).max(1);
        let rows = ((height as f32 / cell_height).floor() as usize).max(1);

        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(
            Some(cols as f32 * cell_width),
            Some(rows as f32 * cell_height),
        );

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            renderer,
            buffer,
            cell_width,
            cell_height,
            cols,
            rows,
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

    // dirty flag: перекладываем сетку только когда меняется число колонок/строк,
    // а не на каждый пиксель ресайза окна
    pub fn resize(&mut self, width: u32, height: u32) {
        let cols = ((width as f32 / self.cell_width).floor() as usize).max(1);
        let rows = ((height as f32 / self.cell_height).floor() as usize).max(1);

        if cols == self.cols && rows == self.rows {
            return;
        }

        self.cols = cols;
        self.rows = rows;
        self.buffer.set_size(
            Some(cols as f32 * self.cell_width),
            Some(rows as f32 * self.cell_height),
        );
        self.buffer.shape_until_scroll(&mut self.font_system, false);
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<(), glyphon::PrepareError> {
        self.viewport.update(queue, Resolution { width, height });

        self.renderer.prepare(
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
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
    ) -> Result<(), glyphon::RenderError> {
        self.renderer.render(&self.atlas, &self.viewport, pass)
    }

    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }
}

fn measure_cell(font_system: &mut FontSystem, metrics: Metrics) -> (f32, f32) {
    let mut probe = Buffer::new(font_system, metrics);
    probe.set_size(None, None);
    probe.set_text(
        "M",
        &Attrs::new().family(Family::Monospace),
        Shaping::Advanced,
        None,
    );
    probe.shape_until_scroll(font_system, false);

    let width = probe
        .layout_runs()
        .next()
        .map(|run| run.line_w)
        .filter(|w| *w > 0.0)
        .unwrap_or(metrics.font_size * 0.6);

    (width, metrics.line_height)
}
