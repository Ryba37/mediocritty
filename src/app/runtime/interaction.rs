use alacritty_terminal::{
    event::WindowSize,
    grid::Dimensions,
    term::TermMode,
    vte::ansi::{NamedColor, Rgb},
};

use crate::{config::Config, term::TabId};

use super::Runtime;

impl Runtime {
    pub(crate) fn paste(&mut self) {
        let Some(text) = self.clipboard.load() else {
            return;
        };

        let text = text.replace("\r\n", "\r").replace('\n', "\r");

        if self.terminal().mode().contains(TermMode::BRACKETED_PASTE) {
            let mut out = Vec::with_capacity(text.len() + 12);

            out.extend_from_slice(b"\x1b[200~");
            out.extend(text.bytes().filter(|b| *b != 0x1b));
            out.extend_from_slice(b"\x1b[201~");

            self.terminal().write(out);
        } else {
            self.terminal().write(text.into_bytes());
        }
    }

    // the window gained or lost focus
    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.focus_tab(self.active, focused);
        self.window.request_redraw();
    }

    // tells one tab whether it has the keyboard, and reports it to the
    // program if it asked for focus events
    pub(super) fn focus_tab(&self, index: usize, focused: bool) {
        let terminal = &self.tabs[index].terminal;
        let mut term = terminal.term().lock();

        if term.is_focused == focused {
            return;
        }

        term.is_focused = focused;

        let report = term.mode().contains(TermMode::FOCUS_IN_OUT);

        drop(term);

        if report {
            terminal.write(if focused {
                b"\x1b[I".to_vec()
            } else {
                b"\x1b[O".to_vec()
            });
        }
    }

    // OSC 4/10/11/12 query. whatever the program set itself wins, the theme
    // fills in the rest
    pub(crate) fn answer_color(
        &self,
        id: TabId,
        index: usize,
        formatter: &(dyn Fn(Rgb) -> String + Sync + Send),
        config: &Config,
    ) {
        let Some(tab) = self.index_of(id).map(|i| &self.tabs[i]) else {
            return;
        };

        let set = tab.terminal.term().lock().colors()[index];
        let theme = &config.theme;
        let palette = theme.palette.to_array();

        let [r, g, b] = set.map(|c| [c.r, c.g, c.b]).unwrap_or_else(|| match index {
            0..=255 => crate::color::indexed(index as u8, &palette),
            i if i == NamedColor::Background as usize => theme.background.0,
            i if i == NamedColor::Cursor as usize => theme.cursor.0,
            _ => theme.foreground.0,
        });

        tab.terminal.write(formatter(Rgb { r, g, b }).into_bytes());
    }

    // CSI 14 t
    pub(crate) fn answer_size(
        &self,
        id: TabId,
        formatter: &(dyn Fn(WindowSize) -> String + Sync + Send),
    ) {
        let Some(tab) = self.index_of(id).map(|i| &self.tabs[i]) else {
            return;
        };

        let (cols, rows) = {
            let term = tab.terminal.term().lock();
            (term.columns(), term.screen_lines())
        };

        let size = WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width: self.metrics.cell_width as u16,
            cell_height: self.metrics.cell_height as u16,
        };

        tab.terminal.write(formatter(size).into_bytes());
    }
}
