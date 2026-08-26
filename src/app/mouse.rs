use alacritty_terminal::selection::{Selection, SelectionType};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
};

use super::App;

impl App {
    pub(super) fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let cell_height = self.runtime.as_ref().unwrap().metrics.cell_height as f64;

        let lines = crate::input::scroll_delta_to_lines(
            delta,
            cell_height,
            &mut self.input.scroll_accum,
            self.config.lines_per_notch,
        );

        if lines == 0 {
            return;
        }

        if let Some(runtime) = self.runtime_mut() {
            runtime.terminal.scroll(lines);
            runtime.window.request_redraw();
        }
    }

    pub(super) fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.input.mouse_pos = position;

        if !self.input.mouse_left_down {
            return;
        }

        let Some((point, side)) = self.point_under_mouse() else {
            return;
        };

        if let Some(runtime) = self.runtime_mut() {
            runtime.terminal.update_selection(point, side);
            runtime.window.request_redraw();
        }
    }

    pub(super) fn on_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }

        self.input.mouse_left_down = state == ElementState::Pressed;

        if state != ElementState::Pressed {
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
}
