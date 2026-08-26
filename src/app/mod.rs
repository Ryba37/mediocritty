use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::{app::runtime::Runtime, config::Config, term::UserEvent};

mod input;
mod keyboard;
mod mouse;
mod runtime;
mod user_event;

use input::InputState;

pub struct App {
    runtime: Option<Runtime>,
    proxy: EventLoopProxy<UserEvent>,
    input: InputState,
    config: Config,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>, config: Config) -> Self {
        Self {
            proxy,
            runtime: None,
            config,
            input: InputState::new(),
        }
    }

    fn redraw(&mut self) {
        let focused = self.input.focused;

        if let Some(runtime) = self.runtime_mut() {
            runtime.redraw(focused);
        }
    }

    fn runtime(&self) -> Option<&Runtime> {
        self.runtime.as_ref()
    }

    fn runtime_mut(&mut self) -> Option<&mut Runtime> {
        self.runtime.as_mut()
    }

    fn reload_config(&mut self, config: Config) {
        let Some(runtime) = self.runtime.as_mut() else {
            self.config = config;
            return;
        };

        match runtime.reload_graphics(&config) {
            Ok(()) => {
                self.config = config;
                runtime.window.request_redraw();
            }
            Err(e) => eprintln!("config reload: {e}, keeping previous font/renderer"),
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.is_some() {
            return;
        }

        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let runtime = match Runtime::new(window, self.proxy.clone(), &self.config) {
            Ok(runtime) => runtime,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };

        runtime.window.request_redraw();
        self.runtime = Some(runtime);
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

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };

        if runtime.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => self.redraw(),

            WindowEvent::Resized(size) => {
                if let Some(runtime) = self.runtime.as_mut() {
                    runtime.resize(size);
                }

                self.redraw();

                if let Some(runtime) = self.runtime.as_ref() {
                    runtime.window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => self.on_keyboard_input(event),

            WindowEvent::ModifiersChanged(m) => self.input.modifiers = m.state(),

            WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(delta),

            WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(position),

            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(state, button),

            WindowEvent::Focused(focused) => {
                self.input.focused = focused;

                if let Some(runtime) = self.runtime.as_mut() {
                    runtime.set_focused(focused);
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.on_user_event(event_loop, event);
    }
}
