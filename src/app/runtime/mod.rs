use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

use crate::{
    clipboard::Clipboard,
    config::Config,
    font::{FontCache, Metrics},
    gpu::Renderer,
    layout::Layout,
    term::{EventProxy, Terminal, UserEvent},
};

mod graphics;
mod interaction;

pub(crate) struct Runtime {
    pub(crate) window: Window,
    pub(crate) renderer: Renderer,
    pub(crate) cache: FontCache,
    pub(crate) layout: Layout,
    pub(crate) terminal: Terminal,
    pub(crate) clipboard: Clipboard,
    pub(crate) metrics: Metrics,
}

impl Runtime {
    pub(crate) fn new(
        window: Window,
        proxy: EventLoopProxy<UserEvent>,
        config: &Config,
    ) -> Result<Self, String> {
        let (metrics, cache, renderer) = Self::build_graphics(&window, config)?;

        let size = window.inner_size();
        let (cols, rows) = grid_size(size, metrics);

        let terminal = Terminal::new(
            EventProxy::new(proxy),
            cols,
            rows,
            metrics.cell_width as u16,
            metrics.cell_height as u16,
        )?;

        Ok(Self {
            window,
            renderer,
            cache,
            layout: Layout::new(config),
            terminal,
            clipboard: Clipboard::new(),
            metrics,
        })
    }

    pub(crate) fn redraw(&mut self, focused: bool) {
        let term = self.terminal.term().lock();

        let frame = self
            .layout
            .build(term.renderable_content(), &mut self.cache, focused);

        self.renderer.render(&frame, self.cache.atlas_mut());
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        let scale = self.window.scale_factor();

        self.renderer.resize(size.width, size.height, scale);

        let (cols, rows) = grid_size(size, self.metrics);

        self.terminal.resize(
            cols,
            rows,
            self.metrics.cell_width as u16,
            self.metrics.cell_height as u16,
        );
    }
}

pub(crate) fn grid_size(size: PhysicalSize<u32>, metrics: Metrics) -> (usize, usize) {
    let cols = (size.width / metrics.cell_width).max(1) as usize;
    let rows = (size.height / metrics.cell_height).max(1) as usize;

    (cols, rows)
}
