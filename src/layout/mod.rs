use alacritty_terminal::index::Point;
use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};

use crate::color::{indexed, linear};
use crate::config::Config;
use crate::font::{FontCache, Glyph};

struct Theme {
    background: [u8; 3],
    foreground: [u8; 3],
    cursor: [u8; 3],
    palette: [[u8; 3]; 16],
    dim_strength: f32,
    hollow_cursor_thickness: f32,
    bold_is_bright: bool,
}

impl Theme {
    fn from_config(config: &Config) -> Self {
        let palette = config.theme.palette.to_array();

        Self {
            background: config.theme.background.0,
            foreground: config.theme.foreground.0,
            cursor: config.theme.cursor.0,
            palette,
            dim_strength: config.theme.dim_strength,
            hollow_cursor_thickness: config.cursor.hollow_cursor_thickness,
            bold_is_bright: config.font.bold_is_bright,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GlyphInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    // may carry font::WIDE_BIT and font::EXACT_BIT in the top bits - see
    // atlas.rs
    pub cell: u32,
    // how much of the gamma curve the fragment shader should apply to this
    // glyph's coverage, 0 for white on black, 1 for black on white
    pub gamma_mix: f32,
}

// metal rounds a struct up to its alignment, so float2 + uint is 16 bytes
// there. pad here or the shader indexes into garbage
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EmojiInstance {
    pub offset: [f32; 2],
    pub cell: u32,
    pub pad: u32,
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
    emoji: Vec<EmojiInstance>,
    bg: Vec<BgRect>,
    theme: Theme,
}

pub struct Frame<'a> {
    pub glyphs: &'a [GlyphInstance],
    pub emoji: &'a [EmojiInstance],
    pub bg: &'a [BgRect],
}

impl Layout {
    pub fn new(config: &Config) -> Self {
        Self {
            glyphs: Vec::new(),
            emoji: Vec::new(),
            bg: Vec::new(),
            theme: Theme::from_config(config),
        }
    }

    pub fn set_theme(&mut self, config: &Config) {
        self.theme = Theme::from_config(config);
    }

    pub fn build(
        &mut self,
        content: RenderableContent,
        cache: &mut FontCache,
        focused: bool,
    ) -> Frame<'_> {
        self.glyphs.clear();
        self.emoji.clear();
        self.bg.clear();

        let colors = content.colors;
        let window_bg = to_linear(self.theme.background);
        let cursor_point = content.cursor.point;
        let display_offset = content.display_offset as i32;

        let block_cursor = matches!(
            content.cursor.shape,
            CursorShape::Block | CursorShape::HollowBlock
        );
        let hollow_cursor = block_cursor && !focused;

        for item in content.display_iter {
            let cell = item.cell;

            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }

            let wide = cell.flags.contains(Flags::WIDE_CHAR);
            let col = item.point.column.0 as f32;
            let row = (item.point.line.0 + display_offset) as f32;

            let inverse = cell.flags.contains(Flags::INVERSE);
            let at_cursor = block_cursor && !hollow_cursor && item.point == cursor_point;
            let selected = content
                .selection
                .as_ref()
                .is_some_and(|s| s.contains_cell(&item, cursor_point, content.cursor.shape));

            let bold = cell.flags.contains(Flags::BOLD);
            let dim = cell.flags.contains(Flags::DIM);
            let italic = cell.flags.contains(Flags::ITALIC);

            let bright = bold && self.theme.bold_is_bright;

            let (mut fg, mut bg) = (
                resolve(cell.fg, colors, &self.theme, bright, dim),
                resolve(cell.bg, colors, &self.theme, false, false),
            );

            if inverse ^ at_cursor ^ selected {
                std::mem::swap(&mut fg, &mut bg);
            }

            let width = if wide { 2.0 } else { 1.0 };

            if bg != window_bg {
                self.bg.push(BgRect {
                    color: bg,
                    offset: [col, row],
                    size: [width, 1.0],
                });
            }

            if cell.c == ' ' {
                continue;
            }

            let style = match (bold, italic) {
                (false, false) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (true, true) => 3,
            };

            // emoji carry their own color, so fg and the gamma curve are
            // dropped and the instance is a third of the size
            match cache.get_or_insert(cell.c, wide, style) {
                Glyph::Mask(n) => self.glyphs.push(GlyphInstance {
                    color: fg,
                    offset: [col, row],
                    cell: n,
                    gamma_mix: gamma_mix(fg, bg),
                }),
                Glyph::Color(n) => self.emoji.push(EmojiInstance {
                    offset: [col, row],
                    cell: n,
                    pad: 0,
                }),
            }
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
            emoji: &self.emoji,
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
            color: to_linear(self.theme.cursor),
            offset: [point.column.0 as f32, point.line.0 as f32 + y_offset],
            size,
        });
    }

    fn push_hollow_cursor(&mut self, point: Point) {
        let col = point.column.0 as f32;
        let row = point.line.0 as f32;
        let color = to_linear(self.theme.cursor);
        let t = self.theme.hollow_cursor_thickness;

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

fn resolve(
    color: Color,
    colors: &Colors,
    theme: &Theme,
    is_bright: bool,
    is_dim: bool,
) -> [f32; 4] {
    let (rgb, is_dimmed) = match color {
        Color::Spec(rgb) => ([rgb.r, rgb.g, rgb.b], false),
        Color::Indexed(i) => {
            let (idx, use_dim) = resolve_index(i, is_bright, is_dim);

            match colors[idx as usize] {
                Some(rgb) => ([rgb.r, rgb.g, rgb.b], use_dim),
                None => (indexed(idx, &theme.palette), use_dim),
            }
        }
        Color::Named(named) => match named {
            NamedColor::Foreground => (theme.foreground, false),
            NamedColor::Background => (theme.background, false),
            NamedColor::Cursor => (theme.cursor, false),
            other => {
                let (idx, use_dim) = resolve_index(other as u8, is_bright, is_dim);

                match colors[idx as usize] {
                    Some(rgb) => ([rgb.r, rgb.g, rgb.b], use_dim),
                    None => (indexed(idx, &theme.palette), use_dim),
                }
            }
        },
    };

    let rgb = if is_dim && !is_dimmed {
        dim(rgb, theme.dim_strength)
    } else {
        rgb
    };

    to_linear(rgb)
}

fn to_linear(rgb: [u8; 3]) -> [f32; 4] {
    [linear(rgb[0]), linear(rgb[1]), linear(rgb[2]), 1.0]
}

fn luminance(c: [f32; 4]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

// weight for the shader's gamma curve, following kitty's foreground_contrast:
// the darker the text is relative to its background, the more curve it gets.
// 0 is white on black, 1 is black on white. both colors are linear here
fn gamma_mix(fg: [f32; 4], bg: [f32; 4]) -> f32 {
    ((1.0 - luminance(fg) + luminance(bg)) * 0.5).clamp(0.0, 1.0)
}

fn dim(color: [u8; 3], dim_strength: f32) -> [u8; 3] {
    color.map(|c| (c as f32 * dim_strength) as u8)
}

fn resolve_index(color: u8, is_bright: bool, is_dim: bool) -> (u8, bool) {
    if is_dim {
        if color >= 8 && color <= 15 {
            return (color - 8, true);
        }

        return (color, false);
    }

    if is_bright && color < 8 {
        return (color + 8, false);
    }

    (color, false)
}
