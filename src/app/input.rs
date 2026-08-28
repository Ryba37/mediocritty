use std::time::{Duration, Instant};

use alacritty_terminal::index::{Point, Side};
use winit::{dpi::PhysicalPosition, keyboard::ModifiersState};

use super::App;

pub(super) struct InputState {
    pub(super) modifiers: ModifiersState,
    pub(super) scroll_accum: f64,
    pub(super) mouse_pos: PhysicalPosition<f64>,
    pub(super) buttons: u8,
    pub(super) selecting: bool,
    pub(super) reported_cell: Option<(usize, usize)>,
    pub(super) autoscroll_at: Option<Instant>,
    last_click: Option<(Instant, Point)>,
    click_count: u8,
    pub(super) focused: bool,
}

impl InputState {
    pub(super) fn new() -> Self {
        Self {
            modifiers: ModifiersState::default(),
            scroll_accum: 0.0,
            mouse_pos: PhysicalPosition::new(0.0, 0.0),
            buttons: 0,
            selecting: false,
            reported_cell: None,
            autoscroll_at: None,
            last_click: None,
            click_count: 0,
            focused: true,
        }
    }

    pub(super) fn register_click(&mut self, point: Point, window_ms: u64) -> u8 {
        let now = Instant::now();

        self.click_count = match self.last_click {
            Some((at, last_point))
                if last_point == point
                    && now.duration_since(at) < Duration::from_millis(window_ms) =>
            {
                self.click_count % 3 + 1
            }
            _ => 1,
        };

        self.last_click = Some((now, point));
        self.click_count
    }

    pub(super) fn release_all(&mut self) {
        self.buttons = 0;
        self.selecting = false;
        self.autoscroll_at = None;
    }
}

impl App {
    pub(super) fn point_under_mouse(&self) -> Option<(Point, Side)> {
        let runtime = self.runtime()?;
        let (display_offset, columns, screen_lines) = runtime.terminal.viewport();

        Some(crate::input::point_from_pixels(
            self.input.mouse_pos.x,
            self.input.mouse_pos.y,
            runtime.metrics.cell_width as f64,
            runtime.metrics.cell_height as f64,
            display_offset,
            columns,
            screen_lines,
        ))
    }

    pub(super) fn cell_under_mouse(&self) -> Option<(usize, usize)> {
        let runtime = self.runtime()?;
        let (_, columns, screen_lines) = runtime.terminal.viewport();

        Some(crate::input::cell_from_pixels(
            self.input.mouse_pos.x,
            self.input.mouse_pos.y,
            runtime.metrics.cell_width as f64,
            runtime.metrics.cell_height as f64,
            columns,
            screen_lines,
        ))
    }
}
