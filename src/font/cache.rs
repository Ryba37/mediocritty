use crate::font::{
    Atlas, Font, Metrics,
    atlas::{EXACT_BIT, key},
    boxdraw,
};

const NOTDEF_CELL: u32 = 0;

// a full screen of distinct glyphs is a few thousand, so both limits leave
// plenty of headroom before a reset. mask slots are bounded by the texture
const MASK_SLOTS: u32 = u32::MAX;
const COLOR_SLOTS: u32 = 4096;
const TOFU: u16 = 0;

// faces coretext hands out for chars the configured list lacks. kept per
// style, capped so a stream of exotic scripts can't grow the scan forever
const MAX_SYSTEM_FALLBACKS: usize = 32;

// which atlas the cell index belongs to. the two atlases have separate slot
// numbering, so the tag has to travel with it all the way to the layout
#[derive(Clone, Copy)]
pub enum Glyph {
    Mask(u32),
    Color(u32),
}

pub struct FontCache {
    fonts: [Vec<Font>; 4],
    configured: usize,
    metrics: Metrics,
    atlas: Atlas,
    emoji: Atlas,
    notdef: crate::font::Bitmap,
    // an atlas hit the gpu size limit this frame. the frame still draws
    // (missing glyphs as tofu), then the owner calls reset and redraws
    overflowed: bool,
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

        let notdef = primary.rasterize_glyph(TOFU, metrics, false)?;

        let mut atlas = Atlas::new(metrics.cell_width, metrics.cell_height, 1, MASK_SLOTS);
        let n = atlas.insert_glyph(&notdef).ok_or("atlas too small")?;
        debug_assert_eq!(n, NOTDEF_CELL);

        // one double-wide slot per color glyph. narrow ones (©, ™) waste the
        // right half, but they are rare and this keeps the slot pitch constant
        let emoji = Atlas::new(metrics.cell_width * 2, metrics.cell_height, 4, COLOR_SLOTS);

        Ok(Self {
            configured: fonts.len(),
            fonts: styled_fonts,
            metrics,
            atlas,
            emoji,
            notdef,
            overflowed: false,
        })
    }

    pub fn get_or_insert(&mut self, ch: char, wide: bool, style: u8) -> Glyph {
        let is_box = boxdraw::contains(ch);
        let style = if is_box { 0 } else { style };

        // everything boxdraw draws is exact coverage, so it opts out of the
        // shader's text gamma. the bit rides on the returned index only
        let exact = if is_box { EXACT_BIT } else { 0 };

        let key = key(ch, style, wide);

        // mask first: it holds everything except emoji, so the color map is
        // only touched on a miss
        if let Some(n) = self.atlas.lookup(key) {
            return Glyph::Mask(n | exact);
        }

        if let Some(n) = self.emoji.lookup(key) {
            return Glyph::Color(n);
        }

        if self.overflowed {
            return Glyph::Mask(NOTDEF_CELL);
        }

        let glyph = self.insert(key, ch, wide, style, is_box);

        match glyph {
            Some(Glyph::Mask(n)) => Glyph::Mask(n | exact),
            Some(g) => g,
            None => {
                self.overflowed = true;
                Glyph::Mask(NOTDEF_CELL)
            }
        }
    }

    // none only when an atlas is full
    fn insert(&mut self, key: u32, ch: char, wide: bool, style: u8, is_box: bool) -> Option<Glyph> {
        if is_box && let Some(b) = boxdraw::rasterize(ch, self.metrics) {
            return self.atlas.insert(key, &b, false).map(Glyph::Mask);
        }

        let index = match self.fonts[style as usize]
            .iter()
            .position(|f| f.glyph(ch).is_some())
        {
            Some(i) => Some(i),
            None => self.system_fallback(ch, style),
        };

        let font = index.map(|i| &self.fonts[style as usize][i]);

        if let Some(font) = font
            && let Some(glyph) = font.glyph(ch)
        {
            // the face that owns the glyph decides the route, so any color
            // font works, not just apple color emoji
            if font.is_color() {
                if let Ok(b) = font.rasterize_color(glyph, self.metrics, wide) {
                    return self.emoji.insert_wide_slot(key, &b, wide).map(Glyph::Color);
                }
            } else if let Ok(b) = font.rasterize_glyph(glyph, self.metrics, wide) {
                return self.atlas.insert(key, &b, wide).map(Glyph::Mask);
            }
        }

        self.atlas.alias(key, NOTDEF_CELL);
        Some(Glyph::Mask(NOTDEF_CELL))
    }

    // asks coretext which installed face covers ch and keeps it for next time
    fn system_fallback(&mut self, ch: char, style: u8) -> Option<usize> {
        let list = &mut self.fonts[style as usize];

        if list.len() >= self.configured + MAX_SYSTEM_FALLBACKS {
            return None;
        }

        let found = list.first()?.fallback_for(ch)?;
        let found = found
            .style(style)
            .filter(|f| f.glyph(ch).is_some())
            .unwrap_or(found);
        let name = found.postscript_name();

        if list.iter().any(|f| f.postscript_name() == name) {
            return None;
        }

        list.push(found);
        Some(list.len() - 1)
    }

    pub fn take_overflow(&mut self) -> bool {
        std::mem::take(&mut self.overflowed)
    }

    // empties both atlases. used when one of them hits the texture limit:
    // only what is on screen gets rasterized again
    pub fn reset(&mut self) {
        self.atlas.clear();
        self.emoji.clear();

        let n = self.atlas.insert_glyph(&self.notdef);
        debug_assert_eq!(n, Some(NOTDEF_CELL));
    }

    pub fn metrics(&self) -> Metrics {
        self.metrics
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
