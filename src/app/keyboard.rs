use winit::{event::ElementState, keyboard::Key};

use super::{App, OPTION_AS_ALT};

impl App {
    pub(super) fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        let modifiers = self.input.modifiers;

        if event.state == ElementState::Pressed
            && modifiers.state().super_key()
            && let Key::Character(s) = &event.logical_key
        {
            match s.as_str() {
                "c" | "C" => {
                    if let Some(runtime) = self.runtime_mut()
                        && let Some(text) = runtime.terminal.selection_text()
                    {
                        runtime.clipboard.store(text);
                    }
                    return;
                }

                "v" | "V" => {
                    if let Some(runtime) = self.runtime_mut() {
                        runtime.paste();
                    }
                    return;
                }

                _ => {}
            }
        }

        if let Some(runtime) = self.runtime_mut()
            && let Some(bytes) = crate::input::key_to_bytes(
                &event,
                &modifiers,
                runtime.terminal.mode(),
                OPTION_AS_ALT,
            )
        {
            runtime.terminal.write(bytes);
        }
    }
}
