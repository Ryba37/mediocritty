use winit::event_loop::ActiveEventLoop;

use crate::term::UserEvent;

use super::App;

impl App {
    pub(super) fn on_user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Exit => event_loop.exit(),

            UserEvent::Wakeup => {
                if let Some(runtime) = self.runtime() {
                    runtime.window.request_redraw();
                }
            }

            UserEvent::Title(s) => {
                if let Some(runtime) = self.runtime() {
                    runtime.window.set_title(&s);
                }
            }

            UserEvent::ClipboardStore(_, text) => {
                if let Some(runtime) = self.runtime_mut() {
                    runtime.clipboard.store(text);
                }
            }

            UserEvent::ClipboardLoad(_, formatter) => {
                let Some(runtime) = self.runtime_mut() else {
                    return;
                };
                let text = runtime.clipboard.load().unwrap_or_default();
                runtime.terminal.write(formatter(&text).into_bytes());
            }

            UserEvent::ConfigReload(config) => self.reload_config(config),
        }
    }
}
