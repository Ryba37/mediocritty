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

    pub fn insert(&mut self, ch: char, bitmap: &Bitmap) -> u32 {
        debug_assert!(bitmap.width <= self.cell_width as usize);
        debug_assert!(bitmap.height <= self.cell_height as usize);
        debug_assert!(!self.map.contains_key(&ch));

        if self.next >= self.cols * self.rows {
            self.grow();
        }

        let n = self.next;
        let x = ((n % self.cols) * self.cell_width) as usize;
        let y = ((n / self.cols) * self.cell_height) as usize;
        let stride = self.stride() as usize;

        self.data
            .chunks_exact_mut(stride)
            .skip(y)
            .zip(bitmap.data.chunks_exact(bitmap.width))
            .for_each(|(dst, src)| {
                dst[x..x + bitmap.width].copy_from_slice(src);
            });

        self.map.insert(ch, n);
        self.dirty.push(n);
        self.next += 1;

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
}
