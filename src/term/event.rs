use alacritty_terminal::{
    event::{Event, EventListener, Notify},
    event_loop::Notifier,
    term::ClipboardType,
};
use std::sync::{Arc, OnceLock};
use winit::event_loop::EventLoopProxy;

use crate::config::Config;

#[allow(
    dead_code,
    reason = "used once the linux backend distinguishes primary selection"
)]
pub enum UserEvent {
    Wakeup(TabId),
    Exit(TabId),
    Title(TabId, String),
    ResetTitle(TabId),
    ClipboardStore(ClipboardType, String),
    ClipboardLoad(
        TabId,
        ClipboardType,
        Arc<dyn Fn(&str) -> String + Sync + Send + 'static>,
    ),
    ConfigReload(Config),
}

#[derive(Clone)]
pub struct EventProxy {
    id: TabId,
    proxy: EventLoopProxy<UserEvent>,
    writer: Arc<OnceLock<Notifier>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TabId(pub u32);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let id = self.id;

        let user_event = match event {
            Event::PtyWrite(text) => {
                if let Some(notifier) = self.writer.get() {
                    notifier.notify(text.into_bytes());
                }
                return;
            }

            Event::Wakeup => UserEvent::Wakeup(id),
            Event::Exit | Event::ChildExit(_) => UserEvent::Exit(id),
            Event::Title(s) => UserEvent::Title(id, s),
            Event::ClipboardStore(ty, text) => UserEvent::ClipboardStore(ty, text),
            Event::ClipboardLoad(ty, formatter) => UserEvent::ClipboardLoad(id, ty, formatter),
            Event::ResetTitle => UserEvent::ResetTitle(id),

            _ => return,
        };

        let _ = self.proxy.send_event(user_event);
    }
}

impl EventProxy {
    pub fn new(proxy: EventLoopProxy<UserEvent>, id: TabId) -> Self {
        Self {
            id,
            proxy,
            writer: Arc::new(OnceLock::new()),
        }
    }

    // must be called before the io thread starts, else an early query
    // from the child can be dropped with no writer set yet
    pub fn set_writer(&self, notifier: Notifier) {
        let _ = self.writer.set(notifier);
    }
}
