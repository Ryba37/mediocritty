use std::sync::Arc;
use std::thread::JoinHandle;

use alacritty_terminal::{
    Term,
    event::{Notify, OnResize, WindowSize},
    event_loop::{EventLoop, Msg, Notifier, State},
    grid::{Dimensions, Scroll},
    index::{Point, Side},
    selection::Selection,
    sync::FairMutex,
    term::{Config, TermMode},
    tty::{self, Pty},
};

mod event;
mod size;

pub use event::{EventProxy, UserEvent};
pub use size::TermSize;

pub struct Terminal {
    term: Arc<FairMutex<Term<EventProxy>>>,
    notifier: Notifier,
    io_thread: Option<JoinHandle<(EventLoop<Pty, EventProxy>, State)>>,
}

impl Terminal {
    pub fn new(
        proxy: EventProxy,
        cols: usize,
        rows: usize,
        cell_width: u16,
        cell_height: u16,
    ) -> Result<Self, String> {
        let size = TermSize::new(cols, rows);
        let term = Term::new(Config::default(), &size, proxy.clone());
        let term = Arc::new(FairMutex::new(term));

        let window_size = WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width,
            cell_height,
        };

        let pty =
            tty::new(&tty::Options::default(), window_size, 0).map_err(|e| format!("pty: {e}"))?;

        let event_loop = EventLoop::new(term.clone(), proxy, pty, false, false)
            .map_err(|e| format!("event loop: {e}"))?;

        let notifier = Notifier(event_loop.channel());
        let io_thread = event_loop.spawn();

        Ok(Self {
            term,
            notifier,
            io_thread: Some(io_thread),
        })
    }

    pub fn term(&self) -> &Arc<FairMutex<Term<EventProxy>>> {
        &self.term
    }

    pub fn write(&self, data: Vec<u8>) {
        self.notifier.notify(data);
    }

    pub fn mode(&self) -> TermMode {
        *self.term().lock().mode()
    }

    pub fn scroll(&mut self, delta: i32) {
        if delta == 0 {
            return;
        }

        self.term.lock().scroll_display(Scroll::Delta(delta));
    }

    pub fn display_offset(&self) -> usize {
        self.term.lock().grid().display_offset()
    }

    pub fn columns(&self) -> usize {
        self.term.lock().columns()
    }

    pub fn start_selection(&mut self, selection: Selection) {
        self.term.lock().selection = Some(selection);
    }

    pub fn update_selection(&mut self, point: Point, side: Side) {
        if let Some(selection) = self.term.lock().selection.as_mut() {
            selection.update(point, side);
        }
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term.lock().selection_to_string()
    }

    pub fn shutdown(&mut self) {
        let _ = self.notifier.0.send(Msg::Shutdown);

        if let Some(thread) = self.io_thread.take() {
            let _ = thread.join();
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize, cell_width: u16, cell_height: u16) {
        let term_size = TermSize::new(cols, rows);
        let window_size = WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width,
            cell_height,
        };

        self.term.lock().resize(term_size);
        self.notifier.on_resize(window_size);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.shutdown();
    }
}
