use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::{
    app::DEFAULT_WINDOW_NAME,
    config::Config,
    font::Metrics,
    term::{EventProxy, TabId, Terminal, UserEvent},
};

use super::{Runtime, Tab, grid_size};

pub(super) fn spawn(
    proxy: &EventLoopProxy<UserEvent>,
    id: TabId,
    size: PhysicalSize<u32>,
    metrics: Metrics,
    bar: bool,
    config: &Config,
) -> Result<Tab, String> {
    let (cols, rows) = grid_size(size, metrics, bar);

    let terminal = Terminal::new(
        EventProxy::new(proxy.clone(), id),
        cols,
        rows,
        metrics.cell_width as u16,
        metrics.cell_height as u16,
        config.shell.locale.clone(),
    )?;

    Ok(Tab {
        id,
        terminal,
        title: None,
    })
}

impl Runtime {
    pub(crate) fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    pub(crate) fn active_id(&self) -> TabId {
        self.tabs[self.active].id
    }

    pub(crate) fn new_tab(&mut self, config: &Config) -> Result<(), String> {
        let id = TabId(self.next_id);
        let size = self.window.inner_size();
        let tab = spawn(&self.proxy, id, size, self.metrics, true, config)?;

        self.next_id += 1;
        self.tabs.push(tab);

        // the bar just appeared and took a row from the first tab
        if self.tabs.len() == 2 {
            self.resize_tabs(size, self.metrics);
        }

        self.select_tab(self.tabs.len() - 1);

        Ok(())
    }

    // true when it was the last tab and the app should quit. the last tab is
    // left in place on purpose: tabs is never empty, so tabs[active] is
    // always valid, even for events that arrive after exit was requested
    pub(crate) fn close_tab(&mut self, id: TabId) -> bool {
        let Some(index) = self.index_of(id) else {
            return false;
        };

        if self.tabs.len() == 1 {
            return true;
        }

        self.tabs.remove(index);

        if index < self.active || self.active == self.tabs.len() {
            self.active -= 1;
        }

        // back to a single tab: the bar is gone, give the row back
        if self.tabs.len() == 1 {
            self.resize_tabs(self.window.inner_size(), self.metrics);
        }

        self.sync_title();
        self.window.request_redraw();

        if self.tabs.len() > 1 {
            self.window.request_redraw();
        }

        false
    }

    // which tab is drawn under this pixel, if the bar is there at all
    pub(crate) fn tab_at(&self, x: f64, y: f64) -> Option<usize> {
        let count = self.tabs.len();

        if count < 2 || y < 0.0 {
            return None;
        }

        let (_, cols, rows) = self.terminal().viewport();
        let row = (y / self.metrics.cell_height as f64) as usize;

        // the bar is the row right under the grid. the leftover strip below it
        // counts too, there is nothing else down there to click
        if row < rows {
            return None;
        }

        let width = crate::layout::tab_width(cols, count);

        if width == 0 {
            return None;
        }

        let col = (x.max(0.0) / self.metrics.cell_width as f64) as usize;
        let index = col / width;

        (index < count).then_some(index)
    }

    pub(crate) fn select_tab(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active {
            return;
        }

        self.active = index;
        self.sync_title();
        self.window.request_redraw();
    }

    pub(crate) fn cycle_tab(&mut self, forward: bool) {
        let n = self.tabs.len();
        let step = if forward { 1 } else { n - 1 };

        self.select_tab((self.active + step) % n);
    }

    pub(crate) fn set_tab_title(&mut self, id: TabId, title: Option<String>) {
        let Some(index) = self.index_of(id) else {
            return;
        };

        self.tabs[index].title = title;

        if index == self.active {
            self.sync_title();
        }
    }

    fn sync_title(&self) {
        let title = self.tabs[self.active]
            .title
            .as_deref()
            .unwrap_or(DEFAULT_WINDOW_NAME);

        self.window.set_title(title);
    }
}
