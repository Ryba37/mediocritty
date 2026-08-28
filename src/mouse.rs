use alacritty_terminal::term::TermMode;
use winit::{event::MouseButton, keyboard::ModifiersState};

pub const MOTION: u8 = 32;
pub const RELEASE: u8 = 3;
pub const WHEEL_UP: u8 = 64;
pub const WHEEL_DOWN: u8 = 65;

const X10_LIMIT: usize = 223;

pub fn button_code(button: MouseButton) -> Option<u8> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

fn modifier_bits(modifiers: ModifiersState) -> u8 {
    u8::from(modifiers.shift_key()) * 4
        + u8::from(modifiers.alt_key()) * 8
        + u8::from(modifiers.control_key()) * 16
}

pub fn encode(
    button: u8,
    modifiers: ModifiersState,
    column: usize,
    line: usize,
    pressed: bool,
    mode: TermMode,
) -> Option<Vec<u8>> {
    let modifiers = modifier_bits(modifiers);
    let (column, line) = (column + 1, line + 1);

    if mode.contains(TermMode::SGR_MOUSE) {
        let code = button + modifiers;
        let kind = if pressed { 'M' } else { 'm' };

        return Some(format!("\x1b[<{code};{column};{line}{kind}").into_bytes());
    }

    if column > X10_LIMIT || line > X10_LIMIT {
        return None;
    }

    let code = if pressed { button } else { RELEASE } + modifiers;

    Some(vec![
        0x1b,
        b'[',
        b'M',
        MOTION + code,
        MOTION + column as u8,
        MOTION + line as u8,
    ])
}
