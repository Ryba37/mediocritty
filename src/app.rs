use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::font::Font;
use crate::gpu;

const FONT_SIZE: f64 = 15.0;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    renderer: Option<gpu::Renderer>,
    font: Option<Font>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }

        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let font = match Font::new(None, FONT_SIZE * window.scale_factor()) {
            Ok(font) => font,
            Err(e) => {
                eprint!("{e}");
                event_loop.exit();
                return;
            }
        };

        let bm = font.rasterize('g').unwrap();

        for y in 0..bm.height {
            let row: String = (0..bm.width)
                .map(|x| match bm.data[y * bm.width + x] {
                    0..=63 => '*',
                    64..=127 => '.',
                    128..=191 => '+',
                    _ => '#',
                })
                .collect();
            println!("{row}");
        }

        let renderer = match gpu::Renderer::new(&window, font.metrics()) {
            Ok(r) => r,
            Err(e) => {
                eprint!("{e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.font = Some(font);

        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
            return;
        }

        let (Some(window), Some(renderer)) = (&self.window, &self.renderer) else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                renderer.render();
            }
            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height, window.scale_factor());
                renderer.render();
            }
            // todo: пересоздавать шрифт при ScaleFactorChanged
            _ => (),
        }
    }
}
