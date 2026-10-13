use std::collections::HashMap;

use crate::font::Bitmap;

// every mac that runs metal 2 supports 16k textures, so this is the ceiling
// for both axes. the atlas never asks the gpu for more than this
pub const MAX_TEXTURE_DIM: u32 = 16384;

// columns are fixed for the atlas' life (a slot index maps to x/y through
// them), so pick enough that growing rows alone reaches a big capacity
const MAX_COLS: u32 = 64;
const INITIAL_ROWS: u32 = 16;

const CHAR_BITS: u32 = 21;
const STYLE_BITS: u32 = 2;

// bit 31 of a returned/stored cell index marks a wide (2-slot) glyph;
// GlyphInstance::cell in layout.rs carries this bit straight through to
// the shader, which decodes it to size the quad and UV rect.
pub const WIDE_BIT: u32 = 1 << 31;

// bit 30 marks coverage we rasterized ourselves and know to be exact, so the
// shader leaves the gamma curve and the contrast boost off it. never stored in
// the map, get_or_insert puts it on the way out
pub const EXACT_BIT: u32 = 1 << 30;

pub struct Atlas {
    data: Vec<u8>,
    cell_width: u32,
    cell_height: u32,
    bpp: u32,
    cols: u32,
    rows: u32,
    max_rows: u32,
    map: HashMap<u32, u32>,
    next: u32,
    dirty: Vec<u32>,
    resized: bool,
}

impl Atlas {
    // max_slots caps memory below the texture limit: a full 16k color atlas
    // is ~140MB on each side of the bus, mask glyphs are a quarter the size
    pub fn new(cell_width: u32, cell_height: u32, bpp: u32, max_slots: u32) -> Self {
        let cell_width = cell_width.max(1);
        let cell_height = cell_height.max(1);
        let cols = (MAX_TEXTURE_DIM / cell_width).clamp(2, MAX_COLS);
        let max_rows = (MAX_TEXTURE_DIM / cell_height)
            .min(max_slots.div_ceil(cols))
            .max(1);
        let rows = INITIAL_ROWS.min(max_rows);

        let mut atlas = Self {
            data: Vec::new(),
            cell_width,
            cell_height,
            bpp,
            cols,
            rows,
            max_rows,
            map: HashMap::new(),
            next: 0,
            dirty: Vec::new(),
            resized: true,
        };
        atlas.data = vec![0; atlas.byte_size()];
        atlas
    }

    // texture width in pixels
    pub fn width(&self) -> u32 {
        self.cols * self.cell_width
    }

    pub fn height(&self) -> u32 {
        self.rows * self.cell_height
    }

    pub fn bpp(&self) -> u32 {
        self.bpp
    }

    // bytesPerRow for the upload
    pub fn row_bytes(&self) -> u32 {
        self.width() * self.bpp
    }

    fn byte_size(&self) -> usize {
        self.row_bytes() as usize * self.height() as usize
    }

    fn grow(&mut self) -> bool {
        if self.rows >= self.max_rows {
            return false;
        }

        self.rows = (self.rows * 2).min(self.max_rows);
        self.data.resize(self.byte_size(), 0);
        self.resized = true;
        self.dirty.clear();
        true
    }

    // none once the texture is at the gpu limit; the cache then resets
    fn alloc(&mut self, slots: u32) -> Option<u32> {
        // a pair can't straddle a row end, its pixels must be contiguous
        if slots == 2 && self.next % self.cols == self.cols - 1 {
            self.next += 1;
        }

        while self.next + slots > self.cols * self.rows {
            if !self.grow() {
                return None;
            }
        }

        let n = self.next;
        self.next += slots;
        Some(n)
    }

    fn write(&mut self, n: u32, cell_cols: u32, bitmap: &Bitmap) {
        debug_assert_eq!(bitmap.bpp, self.bpp as usize);
        debug_assert!(bitmap.stride >= bitmap.width * bitmap.bpp);
        debug_assert!(bitmap.data.len() >= bitmap.stride * bitmap.height);

        let (x, y, _, _) = self.cell_rect(n);
        let bpp = self.bpp as usize;
        let x = x as usize * bpp;
        let y = y as usize;
        let row = self.row_bytes() as usize;
        // coretext::fit already keeps every glyph inside its slot(s), this is
        // just a safety net so a bad bitmap clips instead of panicking
        let w = bitmap.width.min((self.cell_width * cell_cols) as usize) * bpp;
        let h = bitmap.height.min(self.cell_height as usize);

        self.data
            .chunks_exact_mut(row)
            .skip(y)
            .take(h)
            .zip(bitmap.data.chunks_exact(bitmap.stride))
            .for_each(|(dst, src)| {
                dst[x..x + w].copy_from_slice(&src[..w]);
            });

        // only the row matters to the uploader
        self.dirty.push(n);
    }

    pub fn insert_glyph(&mut self, bitmap: &Bitmap) -> Option<u32> {
        let n = self.alloc(1)?;
        self.write(n, 1, bitmap);
        Some(n)
    }

    pub fn insert(&mut self, key: u32, bitmap: &Bitmap, wide: bool) -> Option<u32> {
        debug_assert!(!self.map.contains_key(&key));

        let tagged = if wide {
            let slot = self.alloc(2)?;
            self.write(slot, 2, bitmap);
            slot | WIDE_BIT
        } else {
            self.insert_glyph(bitmap)?
        };

        self.map.insert(key, tagged);
        Some(tagged)
    }

    // for an atlas whose slot is already two cells wide: one slot per glyph,
    // WIDE_BIT only says how much of it to sample
    pub fn insert_wide_slot(&mut self, key: u32, bitmap: &Bitmap, wide: bool) -> Option<u32> {
        debug_assert!(!self.map.contains_key(&key));

        let n = self.insert_glyph(bitmap)?;
        let tagged = if wide { n | WIDE_BIT } else { n };

        self.map.insert(key, tagged);
        Some(tagged)
    }

    // drops every glyph and shrinks back. a fresh texture is made on the next
    // sync, frames still in flight keep sampling the old one
    pub fn clear(&mut self) {
        self.map.clear();
        self.next = 0;
        self.rows = INITIAL_ROWS.min(self.max_rows);
        self.data.clear();
        self.data.resize(self.byte_size(), 0);
        self.data.shrink_to_fit();
        self.dirty.clear();
        self.resized = true;
    }

    pub fn take_dirty(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.dirty)
    }

    pub fn lookup(&self, key: u32) -> Option<u32> {
        self.map.get(&key).copied()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn cols(&self) -> u32 {
        self.cols
    }

    pub fn needs_realloc(&self) -> bool {
        self.resized
    }

    // called once the gpu side really has the new texture, so a failed
    // allocation is retried instead of uploading into a texture too small
    pub fn realloc_done(&mut self) {
        self.resized = false;
        self.dirty.clear();
    }

    pub fn alias(&mut self, key: u32, cell: u32) {
        debug_assert!(cell & !(WIDE_BIT | EXACT_BIT) < self.next);
        self.map.insert(key, cell);
    }

    pub fn cell_rect(&self, n: u32) -> (u32, u32, u32, u32) {
        let x = (n % self.cols) * self.cell_width;
        let y = (n / self.cols) * self.cell_height;

        (x, y, self.cell_width, self.cell_height)
    }
}

// the same char drawn one and two cells wide are different bitmaps
pub fn key(ch: char, style: u8, wide: bool) -> u32 {
    ch as u32 | ((style as u32) << CHAR_BITS) | ((wide as u32) << (CHAR_BITS + STYLE_BITS))
}
