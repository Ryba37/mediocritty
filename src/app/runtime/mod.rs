use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

use crate::{
    app::DEFAULT_WINDOW_NAME,
    clipboard::Clipboard,
    config::Config,
    font::{FontCache, Metrics},
    gpu::Renderer,
    layout::Layout,
    term::{TabId, Terminal, UserEvent},
};

use alacritty_terminal::grid::Dimensions;

mod graphics;
mod interaction;
mod tabs;

pub(crate) struct Tab {
    pub(crate) id: TabId,
    pub(crate) terminal: Terminal,
    // none until the shell sets one
    pub(crate) title: Option<String>,
}

pub(crate) struct Runtime {
    pub(crate) window: Window,
    pub(crate) renderer: Renderer,
    pub(crate) cache: FontCache,
    pub(crate) layout: Layout,
    pub(crate) clipboard: Clipboard,
    pub(crate) metrics: Metrics,
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active: usize,
    proxy: EventLoopProxy<UserEvent>,
    next_id: u32,
    focused: bool,
}

impl Runtime {
    pub(crate) fn new(
        window: Window,
        proxy: EventLoopProxy<UserEvent>,
        config: &Config,
    ) -> Result<Self, String> {
        let (metrics, cache, renderer) = Self::build_graphics(&window, config)?;
        let tab = tabs::spawn(
            &proxy,
            TabId(0),
            window.inner_size(),
            metrics,
            false,
            config,
        )?;

        Ok(Self {
            window,
            renderer,
            cache,
            layout: Layout::new(config),
            clipboard: Clipboard::new(),
            metrics,
            tabs: vec![tab],
            active: 0,
            proxy,
            next_id: 1,
            focused: false,
        })
    }

    pub(crate) fn terminal(&self) -> &Terminal {
        &self.tabs[self.active].terminal
    }

    pub(crate) fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.tabs[self.active].terminal
    }

    pub(crate) fn redraw(&mut self, focused: bool) {
        let term = self.tabs[self.active].terminal.term().lock();

        self.layout
            .build(term.renderable_content(), &mut self.cache, focused);

        let (rows, cols) = (term.screen_lines(), term.columns());

        // the io thread needs this lock to parse pty output, so give it back
        // before the slow part
        drop(term);

        if self.tabs.len() > 1 {
            let active = self.active;
            let tabs = self.tabs.iter().enumerate().map(|(i, t)| {
                (
                    t.title.as_deref().unwrap_or(DEFAULT_WINDOW_NAME),
                    i == active,
                )
            });

            self.layout.push_tab_bar(&mut self.cache, tabs, rows, cols);
        }

        let frame = self.layout.frame();
        let (atlas, emoji) = self.cache.atlases_mut();
        self.renderer.render(&frame, atlas, emoji);
    }

    // every tab keeps its own grid and pty, so all of them have to learn the
    // new size, not just the visible one
    pub(crate) fn resize_tabs(&mut self, size: PhysicalSize<u32>, metrics: Metrics) {
        let (cols, rows) = grid_size(size, metrics, self.tabs.len() > 1);

        for tab in &mut self.tabs {
            tab.terminal.resize(
                cols,
                rows,
                metrics.cell_width as u16,
                metrics.cell_height as u16,
            );
        }
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let scale = self.window.scale_factor();

        self.renderer.resize(size.width, size.height, scale);
        self.resize_tabs(size, self.metrics);
    }
}

pub(crate) fn grid_size(size: PhysicalSize<u32>, metrics: Metrics, bar: bool) -> (usize, usize) {
    let cols = (size.width / metrics.cell_width).max(1) as usize;
    let rows = (size.height / metrics.cell_height) as usize;

    (cols, rows.saturating_sub(bar as usize).max(1))
}
