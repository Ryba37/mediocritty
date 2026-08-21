use crate::font::{Atlas, Font, Metrics, atlas::key};

const NOTDEF_CELL: u32 = 0;
const TOFU: u16 = 0;

pub struct FontCache {
    fonts: [Vec<Font>; 4],
    metrics: Metrics,
    atlas: Atlas,
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

        let mut atlas = Atlas::new(metrics.cell_width, metrics.cell_height);
        let notdef = primary.rasterize_glyph(TOFU, metrics, false)?;
        let n = atlas.insert_glyph(&notdef);
        debug_assert_eq!(n, NOTDEF_CELL);

        Ok(Self {
            fonts: styled_fonts,
            metrics,
            atlas,
        })
    }

    pub fn get_or_insert(&mut self, ch: char, wide: bool, style: u8) -> u32 {
        let key = key(ch, style);
        if let Some(n) = self.atlas.lookup(key) {
            return n;
        }

        for font in &self.fonts[style as usize] {
            if let Ok(bitmap) = font.rasterize(ch, self.metrics, wide) {
                return self.atlas.insert(key, &bitmap, wide);
            }
        }

        self.atlas.alias(key, NOTDEF_CELL);
        NOTDEF_CELL
    }

    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    pub fn atlas_mut(&mut self) -> &mut Atlas {
        &mut self.atlas
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
