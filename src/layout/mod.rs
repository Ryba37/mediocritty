use crate::color::srgb_to_linear;
use crate::font::FontCache;

const FG: [f32; 3] = [0.9, 0.9, 0.85];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GlyphInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub cell: u32,
    pub _pad: u32,
}

pub struct Layout {
    instances: Vec<GlyphInstance>,
}

pub struct Frame<'a> {
    pub instances: &'a [GlyphInstance],
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn build(&mut self, text: &str, cache: &mut FontCache) -> Frame<'_> {
        self.instances.clear();

        let fg = [
            srgb_to_linear(FG[0]),
            srgb_to_linear(FG[1]),
            srgb_to_linear(FG[2]),
            1.0,
        ];

        for (i, ch) in text.chars().enumerate() {
            self.instances.push(GlyphInstance {
                color: fg,
                offset: [i as f32, 0.0],
                cell: cache.get_or_insert(ch),
                _pad: 0,
            });
        }

        Frame {
            instances: &self.instances,
        }
    }
}
