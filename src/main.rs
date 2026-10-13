use alacritty_terminal::tty::setup_env;
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
    // sets TERM/COLORTERM with set_var, which races getenv in any other
    // thread. first thing, before winit or the config watcher spawn one
    setup_env();

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("failed to create loop");

    event_loop.set_control_flow(ControlFlow::Wait);

    let config = Config::load();
    Config::watch(event_loop.create_proxy());

    let mut app = app::App::new(event_loop.create_proxy(), config);
    let _ = event_loop.run_app(&mut app);
}
