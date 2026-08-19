use std::collections::HashMap;

use crate::font::Bitmap;

const INITIAL_COLS: u32 = 16;
const INITIAL_ROWS: u32 = 16;

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

    pub fn insert_glyph(&mut self, bitmap: &Bitmap) -> u32 {
        debug_assert!(bitmap.stride >= bitmap.width);
        debug_assert!(bitmap.data.len() >= bitmap.stride * bitmap.height);

        if self.next >= self.cols * self.rows {
            self.grow();
        }

        let n = self.next;
        let (x, y, _, _) = self.cell_rect(n);
        let x = x as usize;
        let y = y as usize;
        let stride = self.stride() as usize;
        // fallback glyphs may overflow the cell so we clipping instead of panicing
        let w = bitmap.width.min(self.cell_width as usize);
        let h = bitmap.height.min(self.cell_height as usize);

        self.data
            .chunks_exact_mut(stride)
            .skip(y)
            .take(h)
            .zip(bitmap.data.chunks_exact(bitmap.stride))
            .for_each(|(dst, src)| {
                dst[x..x + w].copy_from_slice(&src[..w]);
            });

        self.dirty.push(n);
        self.next += 1;
        n
    }

    pub fn insert(&mut self, ch: char, bitmap: &Bitmap) -> u32 {
        debug_assert!(!self.map.contains_key(&ch));
        let n = self.insert_glyph(bitmap);
        self.map.insert(ch, n);
        n
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
