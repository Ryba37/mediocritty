use winit::event_loop::EventLoop;

mod app;
mod font;
mod gpu;

fn main() {
    let event_loop = EventLoop::new().expect("failed to create loop");

    let mut app = app::App::default();
    let _ = event_loop.run_app(&mut app);
}
