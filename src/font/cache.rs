use crate::font::{Atlas, Font, Metrics, atlas::key, boxdraw};

const NOTDEF_CELL: u32 = 0;
const TOFU: u16 = 0;

// which atlas the cell index belongs to. the two atlases have separate slot
// numbering, so the tag has to travel with it all the way to the layout
#[derive(Clone, Copy)]
pub enum Glyph {
    Mask(u32),
    Color(u32),
}

pub struct FontCache {
    fonts: [Vec<Font>; 4],
    metrics: Metrics,
    atlas: Atlas,
    emoji: Atlas,
}

impl FontCache {
    // fonts[0] is primary and defines cell size;
    // getting bold, italic and italic bold fonts from all entries of fonts
    pub fn new(fonts: Vec<Font>) -> Result<Self, String> {
        let primary = fonts.first().ok_or("no fonts provided")?;
        let mut metrics = primary.metrics()?;

        let styled_fonts: [Vec<Font>; 4] = std::array::from_fn(|s| look_for_style(&fonts, s as u8));

        // a bold or italic face can be wider or taller than regular. size the
        // cell off regular alone and those glyphs overflow their slot, which
        // sends them through fit()'s shrink-and-center path and knocks them
        // off the baseline
        for style in &styled_fonts {
            let Some(m) = style.first().and_then(|f| f.metrics().ok()) else {
                continue;
            };

            metrics.cell_width = metrics.cell_width.max(m.cell_width);
            metrics.cell_height = metrics.cell_height.max(m.cell_height);
            metrics.ascent = metrics.ascent.max(m.ascent);
        }

        // rasterize_glyph derives the baseline as cell_height - ascent, so a
        // taller face must not leave ascent poking out the top
        metrics.ascent = metrics.ascent.min(metrics.cell_height as f32);

        let mut atlas = Atlas::new(metrics.cell_width, metrics.cell_height, 1);
        let notdef = primary.rasterize_glyph(TOFU, metrics, false)?;
        let n = atlas.insert_glyph(&notdef);
        debug_assert_eq!(n, NOTDEF_CELL);

        // one double-wide slot per color glyph. narrow ones (©, ™) waste the
        // right half, but they are rare and this keeps the slot pitch constant
        let emoji = Atlas::new(metrics.cell_width * 2, metrics.cell_height, 4);

        Ok(Self {
            fonts: styled_fonts,
            metrics,
            atlas,
            emoji,
        })
    }

    pub fn get_or_insert(&mut self, ch: char, wide: bool, style: u8) -> Glyph {
        let is_box = boxdraw::contains(ch);
        let style = if is_box { 0 } else { style };

        let key = key(ch, style);

        // mask first: it holds everything except emoji, so the color map is
        // only touched on a miss
        if let Some(n) = self.atlas.lookup(key) {
            return Glyph::Mask(n);
        }

        if let Some(n) = self.emoji.lookup(key) {
            return Glyph::Color(n);
        }

        if is_box {
            if let Some(b) = boxdraw::rasterize(ch, self.metrics) {
                return Glyph::Mask(self.atlas.insert(key, &b, false));
            }
        }

        for font in &self.fonts[style as usize] {
            let Some(glyph) = font.glyph(ch) else {
                continue;
            };

            // the face that owns the glyph decides the route, so any color
            // font works, not just apple color emoji
            if font.is_color() {
                if let Ok(b) = font.rasterize_color(glyph, self.metrics, wide) {
                    return Glyph::Color(self.emoji.insert_wide_slot(key, &b, wide));
                }
            } else if let Ok(b) = font.rasterize_glyph(glyph, self.metrics, wide) {
                return Glyph::Mask(self.atlas.insert(key, &b, wide));
            }
        }

        self.atlas.alias(key, NOTDEF_CELL);
        Glyph::Mask(NOTDEF_CELL)
    }

    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    pub fn emoji(&self) -> &Atlas {
        &self.emoji
    }

    pub fn atlases_mut(&mut self) -> (&mut Atlas, &mut Atlas) {
        (&mut self.atlas, &mut self.emoji)
    }
}

fn look_for_style(fonts: &[Font], style: u8) -> Vec<Font> {
    let debug = std::env::var_os("MEDIOCRITTY_DEBUG_FONTS").is_some();
    let mut styled_fonts = Vec::with_capacity(fonts.len());

    for f in fonts {
        let styled = f.style(style).unwrap_or_else(|| f.clone());

        // asking coretext for a trait does not guarantee it hands back the
        // face you expected, so make it easy to see what actually resolved
        if debug {
            eprintln!("style {style}: {}", styled.postscript_name());
        }

        styled_fonts.push(styled);
    }

    styled_fonts
}
