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

const DEFAULT_LOCALE: &str = "en_US.UTF-8";

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
        locale_cfg: Option<String>,
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

        let mut options = tty::Options::default();

        let candidate = locale_cfg.unwrap_or_else(system_locale);
        let locale = if locale_available(&candidate) {
            candidate
        } else {
            String::from(DEFAULT_LOCALE)
        };

        for key in [
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "LC_COLLATE",
            "LC_MESSAGES",
            "LC_MONETARY",
            "LC_NUMERIC",
            "LC_TIME",
        ] {
            options.env.insert(key.into(), locale.clone());
        }

        let pty = tty::new(&options, window_size, 0).map_err(|e| format!("pty: {e}"))?;

        let event_loop = EventLoop::new(term.clone(), proxy.clone(), pty, false, false)
            .map_err(|e| format!("event loop: {e}"))?;

        let notifier = Notifier(event_loop.channel());
        proxy.set_writer(Notifier(event_loop.channel()));
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

    pub fn viewport(&self) -> (usize, usize, usize) {
        let term = self.term.lock();
        let grid = term.grid();

        (grid.display_offset(), grid.columns(), grid.screen_lines())
    }

    pub fn start_selection(&mut self, selection: Selection) {
        self.term.lock().selection = Some(selection);
    }

    pub fn clear_selection(&mut self) {
        self.term.lock().selection = None;
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

#[cfg(target_os = "macos")]
fn system_locale() -> String {
    std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| format!("{s}.UTF-8"))
        .unwrap_or_else(|| "en_US.UTF-8".into())
}

#[cfg(target_os = "macos")]
fn locale_available(loc: &str) -> bool {
    std::process::Command::new("locale")
        .arg("-a")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l == loc))
        .unwrap_or(false)
}
