use alacritty_terminal::index::Point;
use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};

use crate::color::srgb_to_linear;
use crate::font::FontCache;
use crate::theme;

const GAMMA_STRENGTH: f32 = 0.2;
const HOLLOW_CURSOR_THICKNESS: f32 = 0.08;

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

    pub fn build(
        &mut self,
        content: RenderableContent,
        cache: &mut FontCache,
        focused: bool,
    ) -> Frame<'_> {
        self.glyphs.clear();
        self.bg.clear();

        let colors = content.colors;
        let window_bg = to_linear(theme::BACKGROUND);
        let cursor_point = content.cursor.point;
        let display_offset = content.display_offset as i32;

        let block_cursor = matches!(
            content.cursor.shape,
            CursorShape::Block | CursorShape::HollowBlock
        );
        let hollow_cursor = block_cursor && !focused;

        for item in content.display_iter {
            let cell = item.cell;
            let col = item.point.column.0 as f32;
            let row = (item.point.line.0 + display_offset) as f32;

            let inverse = cell.flags.contains(Flags::INVERSE);
            let at_cursor = block_cursor && !hollow_cursor && item.point == cursor_point;
            let selected = content
                .selection
                .as_ref()
                .is_some_and(|s| s.contains_cell(&item, cursor_point, content.cursor.shape));

            let (mut fg, mut bg) = (
                resolve(cell.fg, colors, true),
                resolve(cell.bg, colors, false),
            );

            if inverse ^ at_cursor ^ selected {
                std::mem::swap(&mut fg, &mut bg);
            }

            if bg != window_bg {
                self.bg.push(BgRect {
                    color: bg,
                    offset: [col, row],
                    size: [1.0, 1.0],
                });
            }

            if cell.c == ' ' {
                continue;
            }

            self.glyphs.push(GlyphInstance {
                color: fg,
                offset: [col, row],
                cell: cache.get_or_insert(cell.c),
                gamma: gamma(fg, bg),
            });
        }

        if display_offset == 0 {
            let shape = if hollow_cursor {
                CursorShape::HollowBlock
            } else {
                content.cursor.shape
            };
            self.push_cursor(shape, cursor_point);
        }

        Frame {
            glyphs: &self.glyphs,
            bg: &self.bg,
        }
    }

    fn push_cursor(&mut self, shape: CursorShape, point: Point) {
        if shape == CursorShape::HollowBlock {
            self.push_hollow_cursor(point);
            return;
        }

        let size = match shape {
            CursorShape::Block => return,
            CursorShape::Underline => [1.0, 0.15],
            CursorShape::Beam => [0.15, 1.0],
            CursorShape::Hidden => return,
            CursorShape::HollowBlock => unreachable!(),
        };

        let y_offset = if shape == CursorShape::Underline {
            0.85
        } else {
            0.0
        };

        self.bg.push(BgRect {
            color: to_linear(theme::CURSOR),
            offset: [point.column.0 as f32, point.line.0 as f32 + y_offset],
            size,
        });
    }

    fn push_hollow_cursor(&mut self, point: Point) {
        let col = point.column.0 as f32;
        let row = point.line.0 as f32;
        let color = to_linear(theme::CURSOR);
        let t = HOLLOW_CURSOR_THICKNESS;

        let edges = [
            ([col, row], [1.0, t]),
            ([col, row + 1.0 - t], [1.0, t]),
            ([col, row], [t, 1.0]),
            ([col + 1.0 - t, row], [t, 1.0]),
        ];

        for (offset, size) in edges {
            self.bg.push(BgRect {
                color,
                offset,
                size,
            });
        }
    }
}

fn resolve(color: Color, colors: &Colors, is_fg: bool) -> [f32; 4] {
    let rgb = match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Indexed(i) => match colors[i as usize] {
            Some(rgb) => [rgb.r, rgb.g, rgb.b],
            None => theme::indexed(i),
        },
        Color::Named(named) => match named {
            NamedColor::Foreground => theme::FOREGROUND,
            NamedColor::Background => theme::BACKGROUND,
            NamedColor::Cursor => theme::CURSOR,
            other => match colors[other as usize] {
                Some(rgb) => [rgb.r, rgb.g, rgb.b],
                None => theme::indexed(other as u8),
            },
        },
    };

    let _ = is_fg;

    to_linear(rgb)
}

fn to_linear(rgb: [u8; 3]) -> [f32; 4] {
    [
        srgb_to_linear(rgb[0] as f32 / 255.0),
        srgb_to_linear(rgb[1] as f32 / 255.0),
        srgb_to_linear(rgb[2] as f32 / 255.0),
        1.0,
    ]
}

fn luminance(c: [f32; 4]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn gamma(fg: [f32; 4], bg: [f32; 4]) -> f32 {
    1.0 + GAMMA_STRENGTH * (luminance(fg) - luminance(bg))
}
