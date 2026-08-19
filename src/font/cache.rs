use crate::font::{Atlas, Font, Metrics};

const NOTDEF_CELL: u32 = 0;
const TOFU: u16 = 0;

pub struct FontCache {
    fonts: Vec<Font>,
    metrics: Metrics,
    atlas: Atlas,
}

impl FontCache {
    // fonts[0] is primary and defines cell size
    pub fn new(fonts: Vec<Font>) -> Result<Self, String> {
        let primary = fonts.first().ok_or("no fonts provided")?;
        let metrics = primary.metrics()?;

        let mut atlas = Atlas::new(metrics.cell_width, metrics.cell_height);
        let notdef = primary.rasterize_glyph(TOFU, metrics)?;
        let n = atlas.insert_glyph(&notdef);
        debug_assert_eq!(n, NOTDEF_CELL);

        Ok(Self {
            fonts,
            metrics,
            atlas,
        })
    }

    pub fn get_or_insert(&mut self, ch: char) -> u32 {
        if let Some(n) = self.atlas.lookup(ch) {
            return n;
        }

        for font in &self.fonts {
            if let Ok(bitmap) = font.rasterize(ch, self.metrics) {
                return self.atlas.insert(ch, &bitmap);
            }
        }

        self.atlas.alias(ch, NOTDEF_CELL);
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
