use winit::window::Window;

use crate::{
    config::Config,
    font::{Font, FontCache, Metrics},
    gpu::Renderer,
};

use super::Runtime;

impl Runtime {
    pub(super) fn build_graphics(
        window: &Window,
        config: &Config,
    ) -> Result<(Metrics, FontCache, Renderer), String> {
        let fonts = Self::build_fonts(config, window.scale_factor())?;
        let mut cache = FontCache::new(fonts)?;
        let metrics = cache.metrics();
        let (atlas, emoji) = cache.atlases_mut();
        let renderer = Renderer::new(window, metrics, atlas, emoji, config)?;

        Ok((metrics, cache, renderer))
    }

    // config reload. the renderer is reconfigured in place: no new device,
    // layer or shader compiles unless opacity changed
    pub(crate) fn reload_graphics(&mut self, config: &Config) -> Result<(), String> {
        let fonts = Self::build_fonts(config, self.window.scale_factor())?;
        let cache = FontCache::new(fonts)?;
        let metrics = cache.metrics();

        self.renderer.reconfigure(&self.window, metrics, config)?;
        self.apply_cache(cache);
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

    pub(crate) fn rebuild_fonts(&mut self, config: &Config, scale: f64) -> Result<(), String> {
        let fonts = Self::build_fonts(config, scale)?;
        let cache = FontCache::new(fonts)?;

        self.renderer.update_metrics(cache.metrics());
        self.apply_cache(cache);

        Ok(())
    }

    // the new atlases are fresh (needs_realloc set), so the next render
    // makes their textures and pushes the sizes into the uniforms
    fn apply_cache(&mut self, cache: FontCache) {
        let metrics = cache.metrics();

        if metrics.cell_width != self.metrics.cell_width
            || metrics.cell_height != self.metrics.cell_height
        {
            self.resize_tabs(self.window.inner_size(), metrics);
        }

        self.metrics = metrics;
        self.cache = cache;
    }
}
