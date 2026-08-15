#[cfg(target_os = "macos")]
mod coretext;
#[cfg(target_os = "macos")]
pub use coretext::Font;

#[derive(Clone, Copy)]
pub struct Metrics {
    pub cell_width: f32,
    pub cell_height: f32,
    pub ascent: f32,
}

pub struct Bitmap {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}
