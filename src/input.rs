use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::term::TermMode;
use winit::event::{ElementState, KeyEvent, MouseScrollDelta};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

const LINES_PER_NOTCH: f64 = 3.0;

pub fn key_to_bytes(
    event: &KeyEvent,
    modifiers: ModifiersState,
    mode: TermMode,
) -> Option<Vec<u8>> {
    if event.state != ElementState::Pressed {
        return None;
    }

    if let Key::Named(named) = event.logical_key
        && let Some(bytes) = app_cursor_key(&named, mode)
    {
        return Some(bytes);
    }

    if let Some(bytes) = named_key(&event.logical_key) {
        return Some(bytes);
    }

    let bare = event.key_without_modifiers();

    if modifiers.control_key()
        && let Key::Character(s) = &bare
        && let Some(byte) = control_byte(s)
    {
        return Some(vec![byte]);
    }

    if modifiers.alt_key()
        && let Key::Character(s) = &bare
    {
        let mut bytes = vec![0x1b];
        bytes.extend_from_slice(s.as_bytes());
        return Some(bytes);
    }

    let text = event.text.as_ref()?;

    if text.is_empty() {
        return None;
    }

    Some(text.as_bytes().to_vec())
}

pub fn scroll_delta_to_lines(delta: MouseScrollDelta, cell_height: f64, accum: &mut f64) -> i32 {
    *accum += match delta {
        MouseScrollDelta::LineDelta(_, y) => y as f64 * LINES_PER_NOTCH,
        MouseScrollDelta::PixelDelta(pos) if cell_height > 0.0 => pos.y / cell_height,
        MouseScrollDelta::PixelDelta(_) => 0.0,
    };

    let lines = accum.trunc();
    *accum -= lines;

    lines as i32
}

pub fn point_from_pixels(
    x: f64,
    y: f64,
    cell_width: f64,
    cell_height: f64,
    display_offset: usize,
    columns: usize,
) -> (Point, Side) {
    let column = ((x / cell_width) as usize).min(columns.saturating_sub(1));
    let line = (y / cell_height) as i32 - display_offset as i32;

    let cell_x = x - column as f64 * cell_width;
    let side = if cell_x < cell_width / 2.0 {
        Side::Left
    } else {
        Side::Right
    };

    (Point::new(Line(line), Column(column)), side)
}

fn named_key(key: &Key) -> Option<Vec<u8>> {
    let Key::Named(named) = key else {
        return None;
    };

    let bytes = match named {
        NamedKey::Enter => vec![b'\r'],
        NamedKey::Backspace => vec![0x7f],
        NamedKey::Tab => vec![b'\t'],
        NamedKey::Escape => vec![0x1b],
        NamedKey::ArrowUp => b"\x1b[A".to_vec(),
        NamedKey::ArrowDown => b"\x1b[B".to_vec(),
        NamedKey::ArrowRight => b"\x1b[C".to_vec(),
        NamedKey::ArrowLeft => b"\x1b[D".to_vec(),
        NamedKey::Home => b"\x1b[H".to_vec(),
        NamedKey::End => b"\x1b[F".to_vec(),
        NamedKey::PageUp => b"\x1b[5~".to_vec(),
        NamedKey::PageDown => b"\x1b[6~".to_vec(),
        NamedKey::Delete => b"\x1b[3~".to_vec(),
        NamedKey::Insert => b"\x1b[2~".to_vec(),
        _ => return None,
    };

    Some(bytes)
}

fn control_byte(s: &str) -> Option<u8> {
    let mut chars = s.chars();
    let ch = chars.next()?;

    if chars.next().is_some() {
        return None;
    }

    if !ch.is_ascii() {
        return None;
    }

    let byte = ch as u8;

    match byte {
        b'a'..=b'z' | b'A'..=b'Z' | b'[' | b'\\' | b']' | b'^' | b'_' => Some(byte & 0x1f),
        b'?' => Some(0x7f),
        b' ' | b'@' => Some(0),
        _ => None,
    }
}

fn app_cursor_key(named: &NamedKey, mode: TermMode) -> Option<Vec<u8>> {
    if !mode.contains(TermMode::APP_CURSOR) {
        return None;
    }
    let bytes = match named {
        NamedKey::ArrowUp => b"\x1bOA".to_vec(),
        NamedKey::ArrowDown => b"\x1bOB".to_vec(),
        NamedKey::ArrowRight => b"\x1bOC".to_vec(),
        NamedKey::ArrowLeft => b"\x1bOD".to_vec(),
        _ => return None,
    };
    Some(bytes)
}
