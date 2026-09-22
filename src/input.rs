use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::term::TermMode;
use winit::event::{ElementState, KeyEvent, Modifiers, MouseScrollDelta};
use winit::keyboard::{Key, KeyCode, ModifiersKeyState, NamedKey, PhysicalKey};
use winit::platform::macos::OptionAsAlt;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

pub fn key_to_bytes(
    event: &KeyEvent,
    mods: &Modifiers,
    mode: TermMode,
    option_as_alt: OptionAsAlt,
) -> Option<Vec<u8>> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let state = mods.state();

    if state.super_key() {
        return None;
    }

    let shift = state.shift_key();
    let ctrl = state.control_key();
    let alt = alt_sends_esc(mods, option_as_alt);

    if let Key::Named(named) = &event.logical_key
        && let Some(bytes) = named_key(*named, shift, ctrl, alt, mode)
    {
        return Some(bytes);
    }

    if ctrl && let Some(b) = ctrl_byte(event) {
        return Some(if alt { vec![0x1b, b] } else { vec![b] });
    }

    let text = event.text.as_deref().filter(|t| !t.is_empty())?;

    let mut out = Vec::with_capacity(text.len() + alt as usize);
    if alt {
        out.push(0x1b);
    }
    out.extend_from_slice(text.as_bytes());

    Some(out)
}

pub fn scroll_delta_to_lines(
    delta: MouseScrollDelta,
    cell_height: f64,
    accum: &mut f64,
    lines_per_notch: f64,
) -> i32 {
    *accum += match delta {
        MouseScrollDelta::LineDelta(_, y) => y as f64 * lines_per_notch,
        MouseScrollDelta::PixelDelta(pos) if cell_height > 0.0 => pos.y / cell_height,
        MouseScrollDelta::PixelDelta(_) => 0.0,
    };

    let lines = accum.trunc();
    *accum -= lines;

    lines as i32
}

pub fn cell_from_pixels(
    x: f64,
    y: f64,
    cell_width: f64,
    cell_height: f64,
    columns: usize,
    screen_lines: usize,
) -> (usize, usize) {
    let column = ((x.max(0.0) / cell_width) as usize).min(columns.saturating_sub(1));
    let line = ((y.max(0.0) / cell_height) as usize).min(screen_lines.saturating_sub(1));

    (column, line)
}

pub fn point_from_pixels(
    x: f64,
    y: f64,
    cell_width: f64,
    cell_height: f64,
    display_offset: usize,
    columns: usize,
    screen_lines: usize,
) -> (Point, Side) {
    let (column, line) = cell_from_pixels(x, y, cell_width, cell_height, columns, screen_lines);
    let line = line as i32 - display_offset as i32;

    let cell_x = x.max(0.0) - column as f64 * cell_width;
    let side = if cell_x < cell_width / 2.0 {
        Side::Left
    } else {
        Side::Right
    };

    (Point::new(Line(line), Column(column)), side)
}

fn alt_sends_esc(mods: &Modifiers, opt: OptionAsAlt) -> bool {
    let l = mods.lalt_state() == ModifiersKeyState::Pressed;
    let r = mods.ralt_state() == ModifiersKeyState::Pressed;

    match opt {
        OptionAsAlt::OnlyLeft => l,
        OptionAsAlt::OnlyRight => r,
        OptionAsAlt::Both => l || r,
        OptionAsAlt::None => false,
    }
}

fn named_key(key: NamedKey, shift: bool, ctrl: bool, alt: bool, mode: TermMode) -> Option<Vec<u8>> {
    // xterm: 1 + shift + alt*2 + ctrl*4
    let m = 1 + shift as u8 + alt as u8 * 2 + ctrl as u8 * 4;

    let letter = match key {
        NamedKey::ArrowUp => Some(b'A'),
        NamedKey::ArrowDown => Some(b'B'),
        NamedKey::ArrowRight => Some(b'C'),
        NamedKey::ArrowLeft => Some(b'D'),
        NamedKey::Home => Some(b'H'),
        NamedKey::End => Some(b'F'),
        _ => None,
    };

    if let Some(c) = letter {
        return Some(if m > 1 {
            vec![0x1b, b'[', b'1', b';', b'0' + m, c]
        } else if mode.contains(TermMode::APP_CURSOR) {
            vec![0x1b, b'O', c]
        } else {
            vec![0x1b, b'[', c]
        });
    }

    let num = match key {
        NamedKey::Insert => Some(b'2'),
        NamedKey::Delete => Some(b'3'),
        NamedKey::PageUp => Some(b'5'),
        NamedKey::PageDown => Some(b'6'),
        _ => None,
    };

    if let Some(n) = num {
        return Some(if m > 1 {
            vec![0x1b, b'[', n, b';', b'0' + m, b'~']
        } else {
            vec![0x1b, b'[', n, b'~']
        });
    }

    let base: &[u8] = match key {
        NamedKey::Enter if mode.contains(TermMode::LINE_FEED_NEW_LINE) => b"\r\n",
        NamedKey::Enter => b"\r",
        NamedKey::Backspace if ctrl => b"\x08",
        NamedKey::Backspace => b"\x7f",
        NamedKey::Tab if shift => return Some(b"\x1b[Z".to_vec()),
        NamedKey::Tab => b"\t",
        NamedKey::Escape => b"\x1b",
        NamedKey::Space if ctrl => b"\0",
        NamedKey::Space if alt => b" ",
        _ => return None,
    };

    let mut out = Vec::with_capacity(base.len() + alt as usize);
    if alt {
        out.push(0x1b);
    }
    out.extend_from_slice(base);

    Some(out)
}

fn ctrl_byte(event: &KeyEvent) -> Option<u8> {
    let ch = match event.key_without_modifiers() {
        Key::Character(s) => single_ascii(&s),
        _ => None,
    }
    // non latin layout, using physical key
    .or_else(|| physical_char(event.physical_key))?;

    match ch {
        b'a'..=b'z' | b'A'..=b'Z' | b'[' | b'\\' | b']' | b'^' | b'_' => Some(ch & 0x1f),
        b'?' => Some(0x7f),
        b'@' => Some(0),
        _ => None,
    }
}

fn single_ascii(s: &str) -> Option<u8> {
    match s.as_bytes() {
        [b] => Some(*b),
        _ => None,
    }
}

fn physical_char(key: PhysicalKey) -> Option<u8> {
    use KeyCode::*;

    let PhysicalKey::Code(code) = key else {
        return None;
    };

    Some(match code {
        KeyA => b'a',
        KeyB => b'b',
        KeyC => b'c',
        KeyD => b'd',
        KeyE => b'e',
        KeyF => b'f',
        KeyG => b'g',
        KeyH => b'h',
        KeyI => b'i',
        KeyJ => b'j',
        KeyK => b'k',
        KeyL => b'l',
        KeyM => b'm',
        KeyN => b'n',
        KeyO => b'o',
        KeyP => b'p',
        KeyQ => b'q',
        KeyR => b'r',
        KeyS => b's',
        KeyT => b't',
        KeyU => b'u',
        KeyV => b'v',
        KeyW => b'w',
        KeyX => b'x',
        KeyY => b'y',
        KeyZ => b'z',
        BracketLeft => b'[',
        Backslash => b'\\',
        BracketRight => b']',
        _ => return None,
    })
}
