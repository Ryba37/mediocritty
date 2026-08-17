use alacritty_terminal::grid::Dimensions;

pub struct TermSize {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermSize {
    fn columns(&self) -> usize {
        self.cols
    }

    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }
}

impl TermSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}
