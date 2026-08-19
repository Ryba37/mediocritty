use std::collections::HashMap;

use crate::font::Bitmap;

const INITIAL_COLS: u32 = 16;
const INITIAL_ROWS: u32 = 16;

// bit 31 of a returned/stored cell index marks a wide (2-slot) glyph;
// GlyphInstance::cell in layout.rs carries this bit straight through to
// the shader, which decodes it to size the quad and UV rect.
pub const WIDE_BIT: u32 = 1 << 31;

pub struct Atlas {
    data: Vec<u8>,
    cell_width: u32,
    cell_height: u32,
    cols: u32,
    rows: u32,
    map: HashMap<char, u32>,
    next: u32,
    dirty: Vec<u32>,
    resized: bool,
}

impl Atlas {
    pub fn new(cell_width: u32, cell_height: u32) -> Self {
        let size = (INITIAL_COLS * cell_width * INITIAL_ROWS * cell_height) as usize;

        Self {
            data: vec![0; size],
            cell_width,
            cell_height,
            cols: INITIAL_COLS,
            rows: INITIAL_ROWS,
            map: HashMap::new(),
            next: 0,
            dirty: Vec::new(),
            resized: true,
        }
    }

    pub fn stride(&self) -> u32 {
        self.cols * self.cell_width
    }

    pub fn height(&self) -> u32 {
        self.rows * self.cell_height
    }

    fn byte_size(&self) -> usize {
        (self.stride() * self.height()) as usize
    }

    fn grow(&mut self) {
        self.rows *= 2;
        self.data.resize(self.byte_size(), 0);
        self.resized = true;
        self.dirty.clear();
    }

    fn alloc(&mut self) -> u32 {
        if self.next >= self.cols * self.rows {
            self.grow();
        }

        let n = self.next;
        self.next += 1;
        n
    }

    // reserves two horizontally adjacent slots so their pixels are
    // contiguous in memory, giving a wide glyph one 2*cell_width canvas.
    fn alloc_wide(&mut self) -> u32 {
        if self.next % self.cols == self.cols - 1 {
            self.alloc(); // last column: waste it, pair can't fit
        }

        let n = self.alloc();
        self.alloc();
        n
    }

    fn write(&mut self, n: u32, cell_cols: u32, bitmap: &Bitmap) {
        debug_assert!(bitmap.stride >= bitmap.width);
        debug_assert!(bitmap.data.len() >= bitmap.stride * bitmap.height);

        let (x, y, _, _) = self.cell_rect(n);
        let x = x as usize;
        let y = y as usize;
        let stride = self.stride() as usize;
        // coretext::fit already keeps every glyph inside its slot(s), this is
        // just a safety net so a bad bitmap clips instead of panicking
        let w = bitmap.width.min((self.cell_width * cell_cols) as usize);
        let h = bitmap.height.min(self.cell_height as usize);

        self.data
            .chunks_exact_mut(stride)
            .skip(y)
            .take(h)
            .zip(bitmap.data.chunks_exact(bitmap.stride))
            .for_each(|(dst, src)| {
                dst[x..x + w].copy_from_slice(&src[..w]);
            });

        for slot in n..n + cell_cols {
            self.dirty.push(slot);
        }
    }

    pub fn insert_glyph(&mut self, bitmap: &Bitmap) -> u32 {
        let n = self.alloc();
        self.write(n, 1, bitmap);
        n
    }

    pub fn insert(&mut self, ch: char, bitmap: &Bitmap, wide: bool) -> u32 {
        debug_assert!(!self.map.contains_key(&ch));

        let tagged = if wide {
            let slot = self.alloc_wide();
            self.write(slot, 2, bitmap);
            slot | WIDE_BIT
        } else {
            self.insert_glyph(bitmap)
        };

        self.map.insert(ch, tagged);
        tagged
    }

    pub fn take_dirty(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.dirty)
    }

    pub fn lookup(&self, ch: char) -> Option<u32> {
        self.map.get(&ch).copied()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn cols(&self) -> u32 {
        self.cols
    }

    pub fn take_resized(&mut self) -> bool {
        std::mem::take(&mut self.resized)
    }

    pub fn alias(&mut self, ch: char, cell: u32) {
        debug_assert!(cell < self.next);
        self.map.insert(ch, cell);
    }

    pub fn cell_rect(&self, n: u32) -> (u32, u32, u32, u32) {
        let x = (n % self.cols) * self.cell_width;
        let y = (n / self.cols) * self.cell_height;

        (x, y, self.cell_width, self.cell_height)
    }
}
