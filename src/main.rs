use winit::event_loop::{ControlFlow, EventLoop};

use crate::{config::Config, term::UserEvent};

mod app;
mod clipboard;
mod color;
mod config;
mod font;
mod gpu;
mod input;
mod layout;
mod mouse;
mod term;

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("failed to create loop");

    event_loop.set_control_flow(ControlFlow::Wait);

    let config = Config::load();
    Config::watch(event_loop.create_proxy());

    alacritty_terminal::tty::setup_env();

    let mut app = app::App::new(event_loop.create_proxy(), config);
    let _ = event_loop.run_app(&mut app);
}
