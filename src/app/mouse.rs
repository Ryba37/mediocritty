use alacritty_terminal::{
    selection::{Selection, SelectionType},
    term::TermMode,
};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
};

use crate::mouse::{MOTION, RELEASE, WHEEL_DOWN, WHEEL_UP};

use super::App;

impl App {
    fn mouse_mode(&self) -> Option<TermMode> {
        if self.input.modifiers.shift_key() {
            return None;
        }

        let mode = self.runtime()?.terminal.mode();

        mode.intersects(TermMode::MOUSE_MODE).then_some(mode)
    }

    fn report(&mut self, button: u8, pressed: bool, mode: TermMode) {
        let Some(cell) = self.cell_under_mouse() else {
            return;
        };

        self.input.reported_cell = Some(cell);

        let Some(bytes) =
            crate::mouse::encode(button, self.input.modifiers, cell.0, cell.1, pressed, mode)
        else {
            return;
        };

        if let Some(runtime) = self.runtime() {
            runtime.terminal.write(bytes);
        }
    }

    pub(super) fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let cell_height = self.runtime().map_or(0.0, |r| r.metrics.cell_height as f64);

        let lines = crate::input::scroll_delta_to_lines(
            delta,
            cell_height,
            &mut self.input.scroll_accum,
            self.config.lines_per_notch,
        );

        if lines == 0 {
            return;
        }

        if let Some(mode) = self.mouse_mode() {
            let button = if lines > 0 { WHEEL_UP } else { WHEEL_DOWN };

            for _ in 0..lines.unsigned_abs() {
                self.report(button, true, mode);
            }

            return;
        }

        if let Some(runtime) = self.runtime_mut() {
            runtime.terminal.scroll(lines);
            runtime.window.request_redraw();
        }
    }

    pub(super) fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.input.mouse_pos = position;

        if let Some(mode) = self.mouse_mode() {
            self.report_motion(mode);
            return;
        }

        if !self.input.selecting {
            return;
        }

        self.update_autoscroll();

        let Some((point, side)) = self.point_under_mouse() else {
            return;
        };

        if let Some(runtime) = self.runtime_mut() {
            runtime.terminal.update_selection(point, side);
            runtime.window.request_redraw();
        }
    }

    fn report_motion(&mut self, mode: TermMode) {
        let held = self.input.buttons;

        let wanted = mode.contains(TermMode::MOUSE_MOTION)
            || (mode.contains(TermMode::MOUSE_DRAG) && held != 0);

        if !wanted {
            return;
        }

        let Some(cell) = self.cell_under_mouse() else {
            return;
        };

        if self.input.reported_cell == Some(cell) {
            return;
        }

        let button = if held == 0 {
            RELEASE
        } else {
            held.trailing_zeros() as u8
        };

        self.report(MOTION + button, true, mode);
    }

    pub(super) fn on_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        let Some(code) = crate::mouse::button_code(button) else {
            return;
        };

        let pressed = state == ElementState::Pressed;
        let bit = 1 << code;

        if pressed {
            self.input.buttons |= bit;
        } else {
            self.input.buttons &= !bit;
        }

        if let Some(mode) = self.mouse_mode() {
            self.input.selecting = false;
            self.input.autoscroll_at = None;

            if pressed && let Some(runtime) = self.runtime_mut() {
                runtime.terminal.clear_selection();
                runtime.window.request_redraw();
            }

            self.report(code, pressed, mode);
            return;
        }

        if button != MouseButton::Left {
            return;
        }

        self.input.selecting = pressed;

        if !pressed {
            self.input.autoscroll_at = None;
            return;
        }

        let Some((point, side)) = self.point_under_mouse() else {
            return;
        };

        let click_count = self
            .input
            .register_click(point, self.config.multi_click_window_ms);

        let selection_type = match click_count {
            1 => SelectionType::Simple,
            2 => SelectionType::Semantic,
            _ => SelectionType::Lines,
        };

        if let Some(runtime) = self.runtime_mut() {
            runtime
                .terminal
                .start_selection(Selection::new(selection_type, point, side));

            runtime.window.request_redraw();
        }
    }

    fn autoscroll_lines(&self) -> i32 {
        let Some(runtime) = self.runtime() else {
            return 0;
        };

        let height = runtime.window.inner_size().height as f64;
        let cell_height = runtime.metrics.cell_height as f64;
        let y = self.input.mouse_pos.y;

        let (distance, sign) = if y < 0.0 {
            (-y, 1)
        } else if y > height {
            (y - height, -1)
        } else {
            return 0;
        };

        let cells = (distance / cell_height.max(1.0) * self.config.autoscroll_speed).ceil();

        sign * (cells as i32).max(1)
    }

    fn update_autoscroll(&mut self) {
        self.input.autoscroll_at = (self.autoscroll_lines() != 0)
            .then(|| Instant::now() + Duration::from_millis(self.config.autoscroll_interval_ms));
    }

    pub(super) fn autoscroll_tick(&mut self) -> bool {
        let lines = self.autoscroll_lines();

        if lines == 0 || !self.input.selecting {
            return false;
        }

        if let Some(runtime) = self.runtime_mut() {
            runtime.terminal.scroll(lines);
        }

        let Some((point, side)) = self.point_under_mouse() else {
            return false;
        };

        if let Some(runtime) = self.runtime_mut() {
            runtime.terminal.update_selection(point, side);
            runtime.window.request_redraw();
        }

        true
    }
}
