use crate::font::{Atlas, Font};

const NOTDEF_CELL: u32 = 0;
const TOFU: u16 = 0;

pub struct FontCache {
    font: Font,
    atlas: Atlas,
}

impl FontCache {
    pub fn new(font: Font) -> Result<Self, String> {
        let metrics = font.metrics();

        let mut atlas = Atlas::new(metrics.cell_width, metrics.cell_height);
        let notdef = font.rasterize_glyph(TOFU)?;
        let n = atlas.insert_glyph(&notdef);
        debug_assert_eq!(n, NOTDEF_CELL);

        Ok(Self { font, atlas })
    }

    pub fn get_or_insert(&mut self, ch: char) -> u32 {
        if let Some(n) = self.atlas.lookup(ch) {
            return n;
        }

        match self.font.rasterize(ch) {
            Ok(bitmap) => self.atlas.insert(ch, &bitmap),
            Err(_) => {
                self.atlas.alias(ch, NOTDEF_CELL);
                NOTDEF_CELL
            }
        }
    }

    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    pub fn atlas_mut(&mut self) -> &mut Atlas {
        &mut self.atlas
    }
}
