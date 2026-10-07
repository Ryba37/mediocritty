use winit::event_loop::ActiveEventLoop;

use crate::term::UserEvent;

use super::App;

impl App {
    pub(super) fn on_user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Exit(id) => {
                if let Some(runtime) = self.runtime_mut()
                    && runtime.close_tab(id)
                {
                    event_loop.exit();
                }
            }

            UserEvent::Wakeup(id) => {
                if let Some(runtime) = self.runtime()
                    && runtime.active_id() == id
                {
                    runtime.window.request_redraw();
                }
            }

            UserEvent::Title(id, s) => {
                if let Some(runtime) = self.runtime_mut() {
                    runtime.set_tab_title(id, Some(s));
                }
            }

            UserEvent::ClipboardStore(_, text) => {
                if let Some(runtime) = self.runtime_mut() {
                    runtime.clipboard.store(text);
                }
            }

            UserEvent::ResetTitle(id) => {
                if let Some(runtime) = self.runtime_mut() {
                    runtime.set_tab_title(id, None);
                }
            }

            UserEvent::ClipboardLoad(id, _, formatter) => {
                let Some(runtime) = self.runtime_mut() else {
                    return;
                };
                let Some(index) = runtime.index_of(id) else {
                    return;
                };

                let text = runtime.clipboard.load().unwrap_or_default();
                runtime.tabs[index]
                    .terminal
                    .write(formatter(&text).into_bytes());
            }
            UserEvent::ConfigReload(config) => self.reload_config(config),
        }
    }
}
