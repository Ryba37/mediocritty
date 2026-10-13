use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};

use crate::color::{indexed, linear};
use crate::config::Config;
use crate::font::{FontCache, Glyph, Metrics};
use std::fmt::Write;

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

// style codes must match the fragment-shader defines in underline.metal
pub const UNDERLINE_STYLE_STRAIGHT: u32 = 0;
pub const UNDERLINE_STYLE_UNDERCURL: u32 = 1;
pub const UNDERLINE_STYLE_DOTTED: u32 = 2;
pub const UNDERLINE_STYLE_DASHED: u32 = 3;

// metal rounds this up to 48 bytes (float4 alignment), so pad explicitly -
// see the comment on EmojiInstance above
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnderlineInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub style: u32,
    pub pad: [u32; 3],
}

// what build needs from one grid cell, copied out while the term lock is
// held so glyph rasterization (slow, coretext) runs without it
struct CellSnap {
    c: char,
    fg: Color,
    bg: Color,
    underline: Option<Color>,
    flags: Flags,
    col: usize,
    // screen row, display offset already applied
    row: i32,
    selected: bool,
    at_cursor: bool,
    // range into Layout::zerowidth
    zw_start: u32,
    zw_len: u32,
}

pub struct Layout {
    cells: Vec<CellSnap>,
    zerowidth: Vec<char>,
    colors: Colors,
    cursor_shape: CursorShape,
    cursor_point: Point,
    display_offset: i32,
    glyphs: Vec<GlyphInstance>,
    emoji: Vec<EmojiInstance>,
    bg: Vec<BgRect>,
    underlines: Vec<UnderlineInstance>,
    theme: Theme,
    label: String,
}

pub struct Frame<'a> {
    pub glyphs: &'a [GlyphInstance],
    pub emoji: &'a [EmojiInstance],
    pub bg: &'a [BgRect],
    pub underlines: &'a [UnderlineInstance],
}

impl Layout {
    pub fn new(config: &Config) -> Self {
        Self {
            cells: Vec::new(),
            zerowidth: Vec::new(),
            colors: Colors::default(),
            cursor_shape: CursorShape::Hidden,
            cursor_point: Point::new(Line(0), Column(0)),
            display_offset: 0,
            glyphs: Vec::new(),
            emoji: Vec::new(),
            bg: Vec::new(),
            underlines: Vec::new(),
            theme: Theme::from_config(config),
            label: String::from(""),
        }
    }

    pub fn set_theme(&mut self, config: &Config) {
        self.theme = Theme::from_config(config);
    }

    // the only part that runs under the term lock: a flat copy, no glyph work
    pub fn snapshot(&mut self, content: RenderableContent) {
        self.cells.clear();
        self.zerowidth.clear();

        let cursor_point = content.cursor.point;
        let shape = content.cursor.shape;
        let display_offset = content.display_offset as i32;

        self.colors = *content.colors;
        self.cursor_shape = shape;
        self.cursor_point = cursor_point;
        self.display_offset = display_offset;

        for item in content.display_iter {
            let cell = item.cell;

            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }

            let zw_start = self.zerowidth.len() as u32;
            if let Some(zw) = cell.zerowidth() {
                self.zerowidth
                    .extend(zw.iter().copied().filter(|&c| visible_mark(c)));
            }

            self.cells.push(CellSnap {
                c: cell.c,
                fg: cell.fg,
                bg: cell.bg,
                underline: cell.underline_color(),
                flags: cell.flags,
                col: item.point.column.0,
                row: item.point.line.0 + display_offset,
                selected: content
                    .selection
                    .as_ref()
                    .is_some_and(|s| s.contains_cell(&item, cursor_point, shape)),
                at_cursor: item.point == cursor_point,
                zw_start,
                zw_len: self.zerowidth.len() as u32 - zw_start,
            });
        }
    }

    // turns the last snapshot into instances; no lock needed
    pub fn build(&mut self, cache: &mut FontCache, focused: bool) -> Frame<'_> {
        self.glyphs.clear();
        self.emoji.clear();
        self.bg.clear();
        self.underlines.clear();

        let metrics = cache.metrics();
        let colors = &self.colors;
        let theme = &self.theme;
        let window_bg = to_linear(theme.background);
        let cursor_color = to_linear(dynamic(colors, NamedColor::Cursor).unwrap_or(theme.cursor));

        let block_cursor = matches!(
            self.cursor_shape,
            CursorShape::Block | CursorShape::HollowBlock
        );
        let hollow_cursor = block_cursor && !focused;
        let solid_cursor = block_cursor && !hollow_cursor;

        for cell in &self.cells {
            let wide = cell.flags.contains(Flags::WIDE_CHAR);
            let col = cell.col as f32;
            let row = cell.row as f32;

            let bold = cell.flags.contains(Flags::BOLD);
            let dim = cell.flags.contains(Flags::DIM);
            let italic = cell.flags.contains(Flags::ITALIC);

            let bright = bold && theme.bold_is_bright;

            let (mut fg, mut bg) = (
                resolve(cell.fg, colors, theme, bright, dim),
                resolve(cell.bg, colors, theme, false, false),
            );

            if cell.flags.contains(Flags::INVERSE) ^ cell.selected {
                std::mem::swap(&mut fg, &mut bg);
            }

            // the block cursor is drawn in the cursor color, the char under it
            // takes whatever the cell's background was
            if solid_cursor && cell.at_cursor {
                fg = bg;
                bg = cursor_color;
            }

            let width = if wide { 2.0 } else { 1.0 };

            let underline_flags = cell.flags & Flags::ALL_UNDERLINES;
            if !underline_flags.is_empty() {
                let underline_color = cell
                    .underline
                    .map(|c| resolve(c, colors, theme, false, false))
                    .unwrap_or(fg);

                push_underline(
                    &mut self.underlines,
                    underline_flags,
                    underline_color,
                    col,
                    row,
                    width,
                    &metrics,
                );
            }

            if bg != window_bg {
                self.bg.push(BgRect {
                    color: bg,
                    offset: [col, row],
                    size: [width, 1.0],
                });
            }

            let style = match (bold, italic) {
                (false, false) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (true, true) => 3,
            };

            let marks =
                &self.zerowidth[cell.zw_start as usize..(cell.zw_start + cell.zw_len) as usize];

            if cell.c != ' ' {
                push_glyph(
                    &mut self.glyphs,
                    &mut self.emoji,
                    cache,
                    cell.c,
                    wide,
                    style,
                    [col, row],
                    fg,
                    bg,
                );
            }

            // combining marks: no shaping, each one is drawn over the base
            // cell. good enough for accents, which is what shows up in practice
            for &mark in marks {
                if let Glyph::Mask(n) = cache.get_or_insert(mark, wide, style) {
                    self.glyphs.push(GlyphInstance {
                        color: fg,
                        offset: [col, row],
                        cell: n,
                        gamma_mix: gamma_mix(fg, bg),
                    });
                }
            }
        }

        if self.display_offset == 0 {
            let shape = if hollow_cursor {
                CursorShape::HollowBlock
            } else {
                self.cursor_shape
            };
            self.push_cursor(shape, self.cursor_point, cursor_color);
        }

        self.frame()
    }

    pub fn frame(&self) -> Frame<'_> {
        Frame {
            glyphs: &self.glyphs,
            emoji: &self.emoji,
            bg: &self.bg,
            underlines: &self.underlines,
        }
    }

    // one row of equal-width tabs right under the grid. call after build, it
    // appends to the same frame
    pub fn push_tab_bar<'a>(
        &mut self,
        cache: &mut FontCache,
        tabs: impl ExactSizeIterator<Item = (&'a str, bool)>,
        row: usize,
        cols: usize,
    ) {
        let width = tab_width(cols, tabs.len());

        if width == 0 {
            return;
        }

        let window_bg = to_linear(self.theme.background);
        let active_bg = to_linear(self.theme.foreground);
        let inactive_fg = to_linear(dim(self.theme.foreground, self.theme.dim_strength));
        let row = row as f32;

        // the last cell of a tab stays empty so neighbours don't touch
        let room = width - 1;

        for (i, (title, active)) in tabs.enumerate() {
            let start = (i * width) as f32;

            let (fg, bg, style) = if active {
                (window_bg, active_bg, 1)
            } else {
                (inactive_fg, window_bg, 0)
            };

            if active {
                self.bg.push(BgRect {
                    color: bg,
                    offset: [start, row],
                    size: [width as f32, 1.0],
                });
            }

            self.label.clear();
            let _ = write!(self.label, " {}: {title}", i + 1);

            let len = self.label.chars().count();

            for (k, ch) in self.label.chars().take(room).enumerate() {
                let ch = if len > room && k == room - 1 {
                    '…'
                } else {
                    ch
                };

                if ch == ' ' {
                    continue;
                }

                let offset = [start + k as f32, row];

                push_glyph(
                    &mut self.glyphs,
                    &mut self.emoji,
                    cache,
                    ch,
                    false,
                    style,
                    offset,
                    fg,
                    bg,
                );
            }
        }
    }

    fn push_cursor(&mut self, shape: CursorShape, point: Point, color: [f32; 4]) {
        if shape == CursorShape::HollowBlock {
            self.push_hollow_cursor(point, color);
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
            color,
            offset: [point.column.0 as f32, point.line.0 as f32 + y_offset],
            size,
        });
    }

    fn push_hollow_cursor(&mut self, point: Point, color: [f32; 4]) {
        let col = point.column.0 as f32;
        let row = point.line.0 as f32;
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
    let indexed_rgb = |i: u8, bright: bool, dim: bool| {
        let (idx, use_dim) = resolve_index(i, bright, dim);

        match colors[idx as usize] {
            Some(rgb) => ([rgb.r, rgb.g, rgb.b], use_dim),
            None => (indexed(idx, &theme.palette), use_dim),
        }
    };

    let (rgb, is_dimmed) = match color {
        Color::Spec(rgb) => ([rgb.r, rgb.g, rgb.b], false),
        Color::Indexed(i) => indexed_rgb(i, is_bright, is_dim),
        Color::Named(named) => match named {
            // OSC 10/11/12 land in colors[256..259], the theme is the fallback
            NamedColor::Foreground | NamedColor::BrightForeground => (
                dynamic(colors, NamedColor::Foreground).unwrap_or(theme.foreground),
                false,
            ),
            NamedColor::DimForeground => (
                dim(
                    dynamic(colors, NamedColor::Foreground).unwrap_or(theme.foreground),
                    theme.dim_strength,
                ),
                true,
            ),
            NamedColor::Background => (dynamic(colors, named).unwrap_or(theme.background), false),
            NamedColor::Cursor => (dynamic(colors, named).unwrap_or(theme.cursor), false),
            other => {
                // 0..16 are the palette, the Dim* block mirrors 0..8 dimmed.
                // a plain `as u8` would wrap 259+ onto random palette slots
                let n = other as usize;
                let dim_black = NamedColor::DimBlack as usize;

                if n < 16 {
                    indexed_rgb(n as u8, is_bright, is_dim)
                } else if (dim_black..dim_black + 8).contains(&n) {
                    let (rgb, _) = indexed_rgb((n - dim_black) as u8, false, false);
                    (dim(rgb, theme.dim_strength), true)
                } else {
                    (theme.foreground, false)
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

fn dynamic(colors: &Colors, named: NamedColor) -> Option<[u8; 3]> {
    colors[named].map(|rgb| [rgb.r, rgb.g, rgb.b])
}

#[allow(clippy::too_many_arguments)]
fn push_glyph(
    glyphs: &mut Vec<GlyphInstance>,
    emoji: &mut Vec<EmojiInstance>,
    cache: &mut FontCache,
    ch: char,
    wide: bool,
    style: u8,
    offset: [f32; 2],
    fg: [f32; 4],
    bg: [f32; 4],
) {
    // emoji carry their own color, so fg and the gamma curve are dropped and
    // the instance is a third of the size
    match cache.get_or_insert(ch, wide, style) {
        Glyph::Mask(n) => glyphs.push(GlyphInstance {
            color: fg,
            offset,
            cell: n,
            gamma_mix: gamma_mix(fg, bg),
        }),
        Glyph::Color(n) => emoji.push(EmojiInstance {
            offset,
            cell: n,
            pad: 0,
        }),
    }
}

// zero-width chars that only steer other glyphs (joiners, variation
// selectors) and have nothing to draw on their own
fn visible_mark(c: char) -> bool {
    !matches!(c,
        '\u{200b}'..='\u{200f}'
        | '\u{2060}'..='\u{2064}'
        | '\u{fe00}'..='\u{fe0f}'
        | '\u{e0100}'..='\u{e01ef}'
    )
}

fn push_underline(
    underlines: &mut Vec<UnderlineInstance>,
    flags: Flags,
    color: [f32; 4],
    col: f32,
    row: f32,
    width: f32,
    metrics: &Metrics,
) {
    let cell_h = metrics.cell_height as f32;
    let descent = cell_h - metrics.ascent;
    let ut = metrics.underline_thickness;
    let straight_y = metrics.ascent + metrics.underline_position - ut / 2.0;

    let mut push = |y_top: f32, height: f32, style: u32| {
        let y_top = y_top.min(cell_h - height).max(0.0);

        underlines.push(UnderlineInstance {
            color,
            offset: [col, row + y_top / cell_h],
            size: [width, height / cell_h],
            style,
            pad: [0; 3],
        });
    };

    if flags.contains(Flags::DOUBLE_UNDERLINE) {
        push(
            metrics.ascent + 0.25 * descent - ut / 2.0,
            ut,
            UNDERLINE_STYLE_STRAIGHT,
        );
        push(
            metrics.ascent + 0.75 * descent - ut / 2.0,
            ut,
            UNDERLINE_STYLE_STRAIGHT,
        );
    } else if flags.contains(Flags::UNDERCURL) {
        push(metrics.ascent, descent, UNDERLINE_STYLE_UNDERCURL);
    } else if flags.contains(Flags::DOTTED_UNDERLINE) {
        push(metrics.ascent, descent, UNDERLINE_STYLE_DOTTED);
    } else if flags.contains(Flags::DASHED_UNDERLINE) {
        push(straight_y, ut, UNDERLINE_STYLE_DASHED);
    } else if flags.contains(Flags::UNDERLINE) {
        push(straight_y, ut, UNDERLINE_STYLE_STRAIGHT);
    }
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
        if (8..=15).contains(&color) {
            return (color - 8, true);
        }

        return (color, false);
    }

    if is_bright && color < 8 {
        return (color + 8, false);
    }

    (color, false)
}

// tabs split the bar evenly. never 0: with more tabs than columns each one
// still gets a cell and the ones past the edge are reachable by keyboard
pub fn tab_width(cols: usize, count: usize) -> usize {
    (cols / count.max(1)).max(1)
}
