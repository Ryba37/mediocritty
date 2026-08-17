use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{Key, NamedKey},
};

pub fn key_to_bytes(event: &KeyEvent) -> Option<Vec<u8>> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let bytes = match &event.logical_key {
        Key::Named(NamedKey::Enter) => vec![b'\r'],
        Key::Named(NamedKey::Backspace) => vec![0x7f],
        Key::Named(NamedKey::Tab) => vec![b'\t'],
        Key::Named(NamedKey::Escape) => vec![0x1b],
        Key::Named(NamedKey::ArrowUp) => b"\x1b[A".to_vec(),
        Key::Named(NamedKey::ArrowDown) => b"\x1b[B".to_vec(),
        Key::Named(NamedKey::ArrowRight) => b"\x1b[C".to_vec(),
        Key::Named(NamedKey::ArrowLeft) => b"\x1b[D".to_vec(),
        _ => event.text.as_ref()?.as_bytes().to_vec(),
    };

    Some(bytes)
}
