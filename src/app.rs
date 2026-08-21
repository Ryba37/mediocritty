use std::time::{Duration, Instant};

use alacritty_terminal::index::Point;
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::TermMode;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState};
use winit::window::{Window, WindowId};

use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::font::{Font, FontCache, Metrics};
use crate::gpu;
use crate::layout::Layout;
use crate::term::{EventProxy, Terminal, UserEvent};

pub struct App {
    window: Option<Window>,
    renderer: Option<gpu::Renderer>,
    cache: Option<FontCache>,
    layout: Option<Layout>,
    terminal: Option<Terminal>,
    clipboard: Option<Clipboard>,
    proxy: EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    metrics: Option<Metrics>,
    scroll_accum: f64,
    mouse_pos: PhysicalPosition<f64>,
    mouse_left_down: bool,
    last_click: Option<(Instant, Point)>,
    click_count: u8,
    focused: bool,
    config: Config,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>, config: Config) -> Self {
        Self {
            proxy,
            window: None,
            renderer: None,
            cache: None,
            layout: None,
            terminal: None,
            clipboard: None,
            modifiers: ModifiersState::default(),
            metrics: None,
            scroll_accum: 0.0,
            mouse_pos: PhysicalPosition::new(0.0, 0.0),
            mouse_left_down: false,
            last_click: None,
            click_count: 0,
            focused: true,
            config,
        }
    }

    fn point_under_mouse(&self) -> Option<(Point, alacritty_terminal::index::Side)> {
        let metrics = self.metrics?;
        let terminal = self.terminal.as_ref()?;

        Some(crate::input::point_from_pixels(
            self.mouse_pos.x,
            self.mouse_pos.y,
            metrics.cell_width as f64,
            metrics.cell_height as f64,
            terminal.display_offset(),
            terminal.columns(),
        ))
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
        let frame = layout.build(term.renderable_content(), cache, self.focused);

        renderer.render(&frame, cache.atlas_mut());
    }

    fn reload_config(&mut self, config: Config) {
        let Some(window) = &self.window else {
            self.config = config;
            return;
        };

        match Self::build_graphics(window, &config) {
            Ok((metrics, cache, renderer)) => {
                if let (Some(terminal), Some(old_metrics)) = (&mut self.terminal, self.metrics)
                    && (metrics.cell_width != old_metrics.cell_width
                        || metrics.cell_height != old_metrics.cell_height)
                {
                    let (cols, rows) = grid_size(window.inner_size(), metrics);
                    terminal.resize(
                        cols,
                        rows,
                        metrics.cell_width as u16,
                        metrics.cell_height as u16,
                    );
                }

                self.metrics = Some(metrics);
                self.cache = Some(cache);
                self.renderer = Some(renderer);
            }
            Err(e) => eprintln!("config reload: {e}, keeping previous font/renderer"),
        }

        if let Some(layout) = &mut self.layout {
            layout.set_theme(&config);
        }

        self.config = config;
        window.request_redraw();
    }

    fn build_fonts(config: &Config, scale: f64) -> Result<Vec<Font>, String> {
        let size = config.font.size * scale;
        let mut fonts = vec![Font::new(Some(&config.font.family), size)?];

        for name in &config.font.fallback {
            match Font::new(Some(name), size) {
                Ok(font) => fonts.push(font),
                Err(e) => eprintln!("fallback font {name}: {e}, skipping"),
            }
        }

        Ok(fonts)
    }

    fn build_graphics(
        window: &Window,
        config: &Config,
    ) -> Result<(Metrics, FontCache, gpu::Renderer), String> {
        let fonts = Self::build_fonts(config, window.scale_factor())?;
        let cache = FontCache::new(fonts)?;
        let metrics = cache.metrics();
        let renderer = gpu::Renderer::new(window, metrics, cache.atlas(), config)?;

        Ok((metrics, cache, renderer))
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

        let (metrics, cache, renderer) = match Self::build_graphics(&window, &self.config) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let (cols, rows) = grid_size(size, metrics);

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
        self.layout = Some(Layout::new(&self.config));
        self.terminal = Some(terminal);
        self.clipboard = Some(Clipboard::new());
        self.metrics = Some(metrics);

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

                if let (Some(metrics), Some(terminal)) = (self.metrics, &mut self.terminal) {
                    let (cols, rows) = grid_size(size, metrics);

                    terminal.resize(
                        cols,
                        rows,
                        metrics.cell_width as u16,
                        metrics.cell_height as u16,
                    );
                }

                self.redraw();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && self.modifiers.super_key()
                    && let Key::Character(s) = &event.logical_key
                {
                    match s.as_str() {
                        "c" | "C" => {
                            if let (Some(terminal), Some(clipboard)) =
                                (&self.terminal, &mut self.clipboard)
                                && let Some(text) = terminal.selection_text()
                            {
                                clipboard.store(text);
                            }
                            return;
                        }
                        "v" | "V" => {
                            if let (Some(terminal), Some(clipboard)) =
                                (&self.terminal, &mut self.clipboard)
                                && let Some(text) = clipboard.load()
                            {
                                terminal.write(text.into_bytes());
                            }
                            return;
                        }
                        _ => {}
                    }
                }

                if let Some(terminal) = &self.terminal
                    && let Some(bytes) =
                        crate::input::key_to_bytes(&event, self.modifiers, terminal.mode())
                {
                    terminal.write(bytes);
                }
            }
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),

            WindowEvent::MouseWheel { delta, .. } => {
                let cell_height = self.metrics.map(|m| m.cell_height as f64).unwrap_or(0.0);
                let lines = crate::input::scroll_delta_to_lines(
                    delta,
                    cell_height,
                    &mut self.scroll_accum,
                    self.config.lines_per_notch,
                );

                if lines != 0
                    && let Some(terminal) = &mut self.terminal
                {
                    terminal.scroll(lines);
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = position;

                if self.mouse_left_down
                    && let Some((point, side)) = self.point_under_mouse()
                    && let Some(terminal) = &mut self.terminal
                {
                    terminal.update_selection(point, side);
                    window.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.mouse_left_down = state == ElementState::Pressed;

                if state != ElementState::Pressed {
                    return;
                }

                let Some((point, side)) = self.point_under_mouse() else {
                    return;
                };

                let now = Instant::now();
                self.click_count = match self.last_click {
                    Some((at, last_point))
                        if last_point == point
                            && now.duration_since(at)
                                < Duration::from_millis(self.config.multi_click_window_ms) =>
                    {
                        self.click_count % 3 + 1
                    }
                    _ => 1,
                };
                self.last_click = Some((now, point));

                let selection_type = match self.click_count {
                    1 => SelectionType::Simple,
                    2 => SelectionType::Semantic,
                    _ => SelectionType::Lines,
                };

                if let Some(terminal) = &mut self.terminal {
                    terminal.start_selection(Selection::new(selection_type, point, side));
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;

                if let Some(terminal) = &self.terminal {
                    let mut term = terminal.term().lock();
                    term.is_focused = focused;
                    let report = term.mode().contains(TermMode::FOCUS_IN_OUT);
                    drop(term);

                    if report {
                        terminal.write(if focused {
                            b"\x1b[I".to_vec()
                        } else {
                            b"\x1b[O".to_vec()
                        });
                    }
                }

                window.request_redraw();
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
            UserEvent::ClipboardStore(_, text) => {
                if let Some(clipboard) = &mut self.clipboard {
                    clipboard.store(text);
                }
            }
            UserEvent::ClipboardLoad(_, formatter) => {
                let text = self
                    .clipboard
                    .as_mut()
                    .and_then(Clipboard::load)
                    .unwrap_or_default();

                if let Some(terminal) = &self.terminal {
                    terminal.write(formatter(&text).into_bytes());
                }
            }
            UserEvent::ConfigReload(config) => self.reload_config(config),
        }
    }
}

fn grid_size(size: PhysicalSize<u32>, metrics: Metrics) -> (usize, usize) {
    let cols = (size.width / metrics.cell_width).max(1) as usize;
    let rows = (size.height / metrics.cell_height).max(1) as usize;

    (cols, rows)
}
