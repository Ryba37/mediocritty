use crate::color::srgb_to_linear;
use crate::font::FontCache;

const DARK: [f32; 3] = [0.05, 0.05, 0.06];
const LIGHT: [f32; 3] = [0.95, 0.95, 0.93];
const GAMMA_STRENGTH: f32 = 0.4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GlyphInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub cell: u32,
    pub gamma: f32,
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

        let dark = linear(DARK);
        let light = linear(LIGHT);
        let width = text.chars().count() as f32;

        self.bg.push(BgRect {
            color: dark,
            offset: [0.0, 0.0],
            width,
            _pad: 0.0,
        });

        self.bg.push(BgRect {
            color: light,
            offset: [0.0, 1.0],
            width,
            _pad: 0.0,
        });

        self.push_line(text, cache, light, dark, 0.0);
        self.push_line(text, cache, dark, light, 1.0);

        Frame {
            glyphs: &self.glyphs,
            bg: &self.bg,
        }
    }

    fn push_line(
        &mut self,
        text: &str,
        cache: &mut FontCache,
        fg: [f32; 4],
        bg: [f32; 4],
        row: f32,
    ) {
        let gamma = gamma(fg, bg);

        for (i, ch) in text.chars().enumerate() {
            self.glyphs.push(GlyphInstance {
                color: fg,
                offset: [i as f32, row],
                cell: cache.get_or_insert(ch),
                gamma,
            });
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

fn luminance(c: [f32; 4]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn gamma(fg: [f32; 4], bg: [f32; 4]) -> f32 {
    1.0 + GAMMA_STRENGTH * (luminance(fg) - luminance(bg))
}
