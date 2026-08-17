use alacritty_terminal::event::{Event, EventListener};
use winit::event_loop::EventLoopProxy;

pub enum UserEvent {
    Wakeup,
    Exit,
    Title(String),
}

#[derive(Clone)]
pub struct EventProxy(EventLoopProxy<UserEvent>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let user_event = match event {
            Event::Wakeup => UserEvent::Wakeup,
            Event::Exit | Event::ChildExit(_) => UserEvent::Exit,
            Event::Title(s) => UserEvent::Title(s),
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
