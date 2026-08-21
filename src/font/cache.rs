use crate::font::{Atlas, Font, Metrics, atlas::key};

const NOTDEF_CELL: u32 = 0;
const TOFU: u16 = 0;

pub struct FontCache {
    fonts: [Vec<Font>; 4],
    metrics: Metrics,
    atlas: Atlas,
}

impl FontCache {
    // fonts[0] is primary ifand defines cell size;
    // getting bold, italic and italic bold fonts from all entries of fonts
    pub fn new(fonts: Vec<Font>) -> Result<Self, String> {
        let primary = fonts.first().ok_or("no fonts provided")?;
        let metrics = primary.metrics()?;

        let mut atlas = Atlas::new(metrics.cell_width, metrics.cell_height);
        let notdef = primary.rasterize_glyph(TOFU, metrics, false)?;
        let n = atlas.insert_glyph(&notdef);
        debug_assert_eq!(n, NOTDEF_CELL);

        let styled_fonts = std::array::from_fn(|s| look_for_style(&fonts, s as u8));

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
    let mut styled_fonts = Vec::new();
    for f in fonts {
        match f.style(style) {
            Some(nf) => styled_fonts.push(nf),
            None => styled_fonts.push(f.clone()),
        }
    }
    styled_fonts
}
