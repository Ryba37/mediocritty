use crate::gpu::GpuCtx;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

#[derive(Default)]
pub struct App {
    gpu: Option<GpuCtx>,
}

impl App {
    pub fn new() -> Self {
        App { gpu: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        window.request_redraw();
        match pollster::block_on(GpuCtx::new(window)) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(e) => {
                eprintln!("gpu initialization error: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(gpu) = &mut self.gpu else { return };
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => gpu.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                gpu.render();
                gpu.window.request_redraw();
            }
            _ => (),
        }
    }
}
