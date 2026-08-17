use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::vte::ansi::CursorShape;

use crate::color::srgb_to_linear;
use crate::font::FontCache;

const FG: [f32; 3] = [0.95, 0.95, 0.93];
const BG: [f32; 3] = [0.07, 0.08, 0.10];
const CURSOR: [f32; 3] = [0.8, 0.8, 0.8];
const GAMMA_STRENGTH: f32 = 0.2;

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
    pub size: [f32; 2],
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

    pub fn build(&mut self, content: RenderableContent, cache: &mut FontCache) -> Frame<'_> {
        self.glyphs.clear();
        self.bg.clear();

        let fg = linear(FG);
        let gamma = gamma(fg, linear(BG));

        for item in content.display_iter {
            let ch = item.cell.c;

            if ch == ' ' {
                continue;
            }

            self.glyphs.push(GlyphInstance {
                color: fg,
                offset: [item.point.column.0 as f32, item.point.line.0 as f32],
                cell: cache.get_or_insert(ch),
                gamma,
            });
        }

        let cursor = content.cursor;

        if let Some(size) = cursor_size(cursor.shape) {
            self.bg.push(BgRect {
                color: linear(CURSOR),
                offset: [
                    cursor.point.column.0 as f32,
                    cursor.point.line.0 as f32 + cursor_y_offset(cursor.shape),
                ],
                size,
            });
        }

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

fn luminance(c: [f32; 4]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn gamma(fg: [f32; 4], bg: [f32; 4]) -> f32 {
    1.0 + GAMMA_STRENGTH * (luminance(fg) - luminance(bg))
}

fn cursor_size(shape: CursorShape) -> Option<[f32; 2]> {
    match shape {
        CursorShape::Block | CursorShape::HollowBlock => Some([1.0, 1.0]),
        CursorShape::Underline => Some([1.0, 0.15]),
        CursorShape::Beam => Some([0.15, 1.0]),
        CursorShape::Hidden => None,
    }
}

fn cursor_y_offset(shape: CursorShape) -> f32 {
    match shape {
        CursorShape::Underline => 0.85,
        _ => 0.0,
    }
}
