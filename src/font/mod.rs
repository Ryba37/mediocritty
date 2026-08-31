#[cfg(target_os = "macos")]
mod coretext;
#[cfg(target_os = "macos")]
pub use coretext::Font;

mod atlas;
mod boxdraw;
mod cache;

pub use atlas::Atlas;
pub use cache::{FontCache, Glyph};

pub type GlyphId = u16;

#[derive(Clone, Copy)]
pub struct Metrics {
    pub cell_width: u32,
    pub cell_height: u32,
    pub ascent: f32,
    // distance from the baseline down to where a plain underline sits, and
    // its thickness - both in pixels, both positive, both measured downward
    pub underline_position: f32,
    pub underline_thickness: f32,
}

pub struct Bitmap {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bpp: usize,
}
