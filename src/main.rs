use winit::event_loop::{ControlFlow, EventLoop};

use crate::term::UserEvent;

mod app;
mod clipboard;
mod color;
mod font;
mod gpu;
mod input;
mod layout;
mod term;
mod theme;

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("failed to create loop");

    event_loop.set_control_flow(ControlFlow::Wait);

    alacritty_terminal::tty::setup_env();

    let mut app = app::App::new(event_loop.create_proxy());
    let _ = event_loop.run_app(&mut app);
}
