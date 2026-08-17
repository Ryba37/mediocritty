use winit::event_loop::{ControlFlow, EventLoop};

use crate::term::UserEvent;

mod app;
mod color;
mod font;
mod gpu;
mod layout;
mod term;

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("failed to create loop");

    event_loop.set_control_flow(ControlFlow::Wait);

    alacritty_terminal::tty::setup_env();

    let mut app = app::App::new(event_loop.create_proxy());
    let _ = event_loop.run_app(&mut app);
}
