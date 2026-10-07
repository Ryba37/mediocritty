use alacritty_terminal::term::TermMode;

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
}
