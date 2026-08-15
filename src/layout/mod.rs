use crate::color::srgb_to_linear;
use crate::font::FontCache;

const FG: [f32; 3] = [0.9, 0.9, 0.85];

pub struct Layout {
    offsets: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    cells: Vec<u32>,
}

pub struct Frame<'a> {
    pub offsets: &'a [[f32; 2]],
    pub colors: &'a [[f32; 4]],
    pub cells: &'a [u32],
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    pub fn new() -> Self {
        Self {
            offsets: Vec::new(),
            colors: Vec::new(),
            cells: Vec::new(),
        }
    }

    pub fn build(&mut self, text: &str, cache: &mut FontCache) -> Frame<'_> {
        self.offsets.clear();
        self.colors.clear();
        self.cells.clear();

        let fg = [
            srgb_to_linear(FG[0]),
            srgb_to_linear(FG[1]),
            srgb_to_linear(FG[2]),
            1.0,
        ];

        for (i, ch) in text.chars().enumerate() {
            self.offsets.push([i as f32, 0.0]);
            self.colors.push(fg);
            self.cells.push(cache.get_or_insert(ch));
        }

        Frame {
            offsets: &self.offsets,
            colors: &self.colors,
            cells: &self.cells,
        }
    }
}
