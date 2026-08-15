use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod color;
mod font;
mod gpu;
mod layout;

fn main() {
    let event_loop = EventLoop::new().expect("failed to create loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = app::App::default();
    let _ = event_loop.run_app(&mut app);
}
