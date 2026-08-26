use winit::window::Window;

use crate::{
    config::Config,
    font::{Font, FontCache, Metrics},
    gpu::Renderer,
};

use super::{grid_size, Runtime};

impl Runtime {
    pub(super) fn build_graphics(
        window: &Window,
        config: &Config,
    ) -> Result<(Metrics, FontCache, Renderer), String> {
        let fonts = Self::build_fonts(config, window.scale_factor())?;
        let cache = FontCache::new(fonts)?;
        let metrics = cache.metrics();
        let renderer = Renderer::new(window, metrics, cache.atlas(), config)?;

        Ok((metrics, cache, renderer))
    }

    pub(crate) fn reload_graphics(&mut self, config: &Config) -> Result<(), String> {
        let (metrics, cache, renderer) = Self::build_graphics(&self.window, config)?;

        if metrics.cell_width != self.metrics.cell_width
            || metrics.cell_height != self.metrics.cell_height
        {
            let (cols, rows) = grid_size(self.window.inner_size(), metrics);

            self.terminal.resize(
                cols,
                rows,
                metrics.cell_width as u16,
                metrics.cell_height as u16,
            );
        }

        self.metrics = metrics;
        self.cache = cache;
        self.renderer = renderer;
        self.layout.set_theme(config);

        Ok(())
    }

    fn build_fonts(config: &Config, scale: f64) -> Result<Vec<Font>, String> {
        let size = config.font.size * scale;
        let mut fonts = vec![Font::new(Some(&config.font.family), size)?];

        for name in &config.font.fallback {
            match Font::new(Some(name), size) {
                Ok(font) => fonts.push(font),
                Err(e) => eprintln!("fallback font {name}: {e}, skipping"),
            }
        }

        Ok(fonts)
    }
}
