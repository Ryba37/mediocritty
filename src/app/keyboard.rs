use winit::{event::ElementState, keyboard::Key};

use crate::term::UserEvent;

use super::{App, OPTION_AS_ALT};

impl App {
    pub(super) fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        let modifiers = self.input.modifiers;

        if event.state == ElementState::Pressed
            && modifiers.state().super_key()
            && let Key::Character(s) = &event.logical_key
        {
            if let Some(d) = s.chars().next().and_then(|c| c.to_digit(10))
                && d >= 1
                && let Some(runtime) = self.runtime_mut()
            {
                // cmd+9 is "last tab" everywhere on macos
                let index = if d == 9 {
                    runtime.tabs.len() - 1
                } else {
                    d as usize - 1
                };

                runtime.select_tab(index);
                return;
            }

            match s.as_str() {
                "c" | "C" => {
                    if let Some(runtime) = self.runtime_mut()
                        && let Some(text) = runtime.terminal().selection_text()
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

                "t" | "T" => {
                    if let Some(runtime) = self.runtime.as_mut()
                        && let Err(e) = runtime.new_tab(&self.config)
                    {
                        eprintln!("new tab: {e}");
                    }
                    return;
                }

                "w" | "W" => {
                    if let Some(runtime) = self.runtime() {
                        let _ = self.proxy.send_event(UserEvent::Exit(runtime.active_id()));
                    }
                    return;
                }

                "{" => {
                    if let Some(runtime) = self.runtime_mut() {
                        runtime.cycle_tab(false);
                    }
                    return;
                }

                "}" => {
                    if let Some(runtime) = self.runtime_mut() {
                        runtime.cycle_tab(true);
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
                runtime.terminal().mode(),
                OPTION_AS_ALT,
            )
        {
            runtime.terminal().write(bytes);
        }
    }
}
