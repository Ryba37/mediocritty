use alacritty_terminal::{
    event::{Event, EventListener},
    term::ClipboardType,
};
use std::sync::Arc;
use winit::event_loop::EventLoopProxy;

use crate::config::Config;

#[allow(
    dead_code,
    reason = "used once the linux backend distinguishes primary selection"
)]
pub enum UserEvent {
    Wakeup,
    Exit,
    Title(String),
    ClipboardStore(ClipboardType, String),
    ClipboardLoad(
        ClipboardType,
        Arc<dyn Fn(&str) -> String + Sync + Send + 'static>,
    ),
    ConfigReload(Config),
}

#[derive(Clone)]
pub struct EventProxy(EventLoopProxy<UserEvent>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let user_event = match event {
            Event::Wakeup => UserEvent::Wakeup,
            Event::Exit | Event::ChildExit(_) => UserEvent::Exit,
            Event::Title(s) => UserEvent::Title(s),
            Event::ClipboardStore(ty, text) => UserEvent::ClipboardStore(ty, text),
            Event::ClipboardLoad(ty, formatter) => UserEvent::ClipboardLoad(ty, formatter),

            _ => return,
        };

        let _ = self.0.send_event(user_event);
    }
}

impl EventProxy {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self(proxy)
    }
}
