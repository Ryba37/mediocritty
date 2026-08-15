use crate::color::srgb_to_linear;
use crate::font::FontCache;

const FG: [f32; 3] = [0.9, 0.9, 0.85];
const BG_HIGHLIGHT: [f32; 3] = [0.16, 0.18, 0.24];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GlyphInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub cell: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BgRect {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub width: f32,
    pub _pad: f32,
}

pub struct Layout {
    glyphs: Vec<GlyphInstance>,
    bg: Vec<BgRect>,
}

pub struct Frame<'a> {
    pub glyphs: &'a [GlyphInstance],
    pub bg: &'a [BgRect],
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    pub fn new() -> Self {
        Self {
            glyphs: Vec::new(),
            bg: Vec::new(),
        }
    }

    pub fn build(&mut self, text: &str, cache: &mut FontCache) -> Frame<'_> {
        self.glyphs.clear();
        self.bg.clear();

        let fg = linear(FG);
        let highlight = linear(BG_HIGHLIGHT);

        for (i, ch) in text.chars().enumerate() {
            self.glyphs.push(GlyphInstance {
                color: fg,
                offset: [i as f32, 0.0],
                cell: cache.get_or_insert(ch),
                _pad: 0,
            });
        }

        self.bg.push(BgRect {
            color: highlight,
            offset: [0.0, 0.0],
            width: self.glyphs.len() as f32,
            _pad: 0.0,
        });

        Frame {
            glyphs: &self.glyphs,
            bg: &self.bg,
        }
    }
}

fn linear(c: [f32; 3]) -> [f32; 4] {
    [
        srgb_to_linear(c[0]),
        srgb_to_linear(c[1]),
        srgb_to_linear(c[2]),
        1.0,
    ]
}
