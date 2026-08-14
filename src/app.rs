use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::gpu;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    renderer: Option<gpu::Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }

        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let renderer = match gpu::Renderer::new(&window) {
            Ok(r) => r,
            Err(e) => {
                eprint!("{e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
            return;
        }

        let (Some(window), Some(renderer)) = (&self.window, &self.renderer) else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                renderer.render();
            }
            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height, window.scale_factor());
                renderer.render();
            }
            _ => (),
        }
    }
}
