use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::font::{Font, FontCache};
use crate::gpu;

const FONT_SIZE: f64 = 15.0;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    renderer: Option<gpu::Renderer>,
    cache: Option<FontCache>,
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

        let mut cache = match FontCache::new(font) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        let mut renderer = match gpu::Renderer::new(&window, metrics, cache.atlas()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        let text = "mediocritty lol";
        let mut offsets = Vec::new();
        let mut colors = Vec::new();
        let mut cells = Vec::new();

        for (i, ch) in text.chars().enumerate() {
            offsets.push([i as f32, 0.0]);
            colors.push([0.9, 0.9, 0.85, 1.0]);
            cells.push(cache.get_or_insert(ch));
        }

        if let Err(e) = renderer.upload_instances(&offsets, &colors, &cells) {
            eprintln!("{e}");
            event_loop.exit();
            return;
        }

        renderer.sync_atlas(cache.atlas_mut());

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.cache = Some(cache);

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

        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
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
