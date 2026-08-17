use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::font::{Font, FontCache};
use crate::gpu;
use crate::layout::Layout;
use crate::term::{EventProxy, Terminal, UserEvent};

const FONT_SIZE: f64 = 15.0;

pub struct App {
    window: Option<Window>,
    renderer: Option<gpu::Renderer>,
    cache: Option<FontCache>,
    layout: Option<Layout>,
    terminal: Option<Terminal>,
    proxy: EventLoopProxy<UserEvent>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            window: None,
            renderer: None,
            cache: None,
            layout: None,
            terminal: None,
        }
    }

    fn redraw(&mut self) {
        let (Some(renderer), Some(cache), Some(layout), Some(terminal)) = (
            &mut self.renderer,
            &mut self.cache,
            &mut self.layout,
            &self.terminal,
        ) else {
            return;
        };

        let term = terminal.term().lock();
        let frame = layout.build(term.renderable_content(), cache);

        renderer.render(&frame, cache.atlas_mut());
    }
}

impl ApplicationHandler<UserEvent> for App {
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

        let size = window.inner_size();
        let cols = (size.width / metrics.cell_width).max(1) as usize;
        let rows = (size.height / metrics.cell_height).max(1) as usize;

        let terminal = match Terminal::new(
            EventProxy::new(self.proxy.clone()),
            cols,
            rows,
            metrics.cell_width as u16,
            metrics.cell_height as u16,
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.cache = Some(cache);
        self.layout = Some(Layout::new());
        self.terminal = Some(terminal);

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

        let Some(window) = &self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            WindowEvent::Resized(size) => {
                let scale = window.scale_factor();

                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height, scale);
                }

                self.redraw();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(bytes) = crate::input::key_to_bytes(&event)
                    && let Some(terminal) = &self.terminal
                {
                    terminal.write(bytes);
                }
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Exit => event_loop.exit(),
            UserEvent::Wakeup => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UserEvent::Title(s) => {
                if let Some(window) = &self.window {
                    window.set_title(&s);
                }
            }
        }
    }
}
