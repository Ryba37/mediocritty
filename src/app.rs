use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::font::{Font, FontCache};
use crate::gpu;
use crate::layout::Layout;

const FONT_SIZE: f64 = 15.0;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    renderer: Option<gpu::Renderer>,
    cache: Option<FontCache>,
    layout: Option<Layout>,
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
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        let metrics = font.metrics();

        let cache = match FontCache::new(font) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        let renderer = match gpu::Renderer::new(&window, metrics, cache.atlas()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        let layout = Layout::new();

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.cache = Some(cache);
        self.layout = Some(layout);

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

        let (Some(window), Some(renderer), Some(cache), Some(layout)) = (
            &self.window,
            &mut self.renderer,
            &mut self.cache,
            &mut self.layout,
        ) else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                let frame = layout.build("mediocritty lol", cache);
                renderer.render(&frame, cache.atlas_mut());
            }
            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height, window.scale_factor());
                let frame = layout.build("mediocritty lol", cache);
                renderer.render(&frame, cache.atlas_mut());
            }
            // todo: пересоздавать шрифт при ScaleFactorChanged
            _ => (),
        }
    }
}
