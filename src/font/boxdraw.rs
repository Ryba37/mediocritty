use crate::font::{Bitmap, Metrics};

const SS: usize = 4; // subpixel grid for arcs and diagonals

const NONE: u8 = 0;
const HEAVY: u8 = 2;
const DOUBLE: u8 = 3;

struct Geom {
    cell_width: usize,
    cell_height: usize,
    light: usize,
    heavy: usize,
}

impl Geom {
    fn new(metrics: Metrics) -> Self {
        let cell_width = metrics.cell_width as usize;
        let cell_height = metrics.cell_height as usize;

        let light = ((cell_height + 6) / 12).max(1);
        let heavy = (light * 2).min((cell_width.min(cell_height) / 2).max(1));

        Self {
            cell_width,
            cell_height,
            light,
            heavy,
        }
    }

    fn thick(&self, arm: u8) -> usize {
        match arm {
            NONE => 0,
            HEAVY => self.heavy,
            _ => self.light,
        }
    }

    // left edge of a vertical stroke of thickness t, centered in the cell
    fn vband(&self, t: usize) -> usize {
        self.cell_width.saturating_sub(t) / 2
    }

    fn hband(&self, t: usize) -> usize {
        self.cell_height.saturating_sub(t) / 2
    }

    fn blank(&self) -> Bitmap {
        Bitmap {
            data: vec![0; self.cell_width * self.cell_height],
            width: self.cell_width,
            height: self.cell_height,
            stride: self.cell_width,
            bpp: 1,
        }
    }
}

const fn arms(up: u8, right: u8, down: u8, left: u8) -> u8 {
    (up << 6) | (right << 4) | (down << 2) | left
}

#[rustfmt::skip]
const ARMS: [u8; 0x80] = [
    arms(0,1,0,1), // ─
    arms(0,2,0,2), // ━
    arms(1,0,1,0), // │
    arms(2,0,2,0), // ┃
    0, 0, 0, 0, 0, 0, 0, 0, // ┄┅┆┇┈┉┊┋ dashed
    arms(0,1,1,0), // ┌
    arms(0,2,1,0), // ┍
    arms(0,1,2,0), // ┎
    arms(0,2,2,0), // ┏
    arms(0,0,1,1), // ┐
    arms(0,0,1,2), // ┑
    arms(0,0,2,1), // ┒
    arms(0,0,2,2), // ┓
    arms(1,1,0,0), // └
    arms(1,2,0,0), // ┕
    arms(2,1,0,0), // ┖
    arms(2,2,0,0), // ┗
    arms(1,0,0,1), // ┘
    arms(1,0,0,2), // ┙
    arms(2,0,0,1), // ┚
    arms(2,0,0,2), // ┛
    arms(1,1,1,0), // ├
    arms(1,2,1,0), // ┝
    arms(2,1,1,0), // ┞
    arms(1,1,2,0), // ┟
    arms(2,1,2,0), // ┠
    arms(2,2,1,0), // ┡
    arms(1,2,2,0), // ┢
    arms(2,2,2,0), // ┣
    arms(1,0,1,1), // ┤
    arms(1,0,1,2), // ┥
    arms(2,0,1,1), // ┦
    arms(1,0,2,1), // ┧
    arms(2,0,2,1), // ┨
    arms(2,0,1,2), // ┩
    arms(1,0,2,2), // ┪
    arms(2,0,2,2), // ┫
    arms(0,1,1,1), // ┬
    arms(0,1,1,2), // ┭
    arms(0,2,1,1), // ┮
    arms(0,2,1,2), // ┯
    arms(0,1,2,1), // ┰
    arms(0,1,2,2), // ┱
    arms(0,2,2,1), // ┲
    arms(0,2,2,2), // ┳
    arms(1,1,0,1), // ┴
    arms(1,1,0,2), // ┵
    arms(1,2,0,1), // ┶
    arms(1,2,0,2), // ┷
    arms(2,1,0,1), // ┸
    arms(2,1,0,2), // ┹
    arms(2,2,0,1), // ┺
    arms(2,2,0,2), // ┻
    arms(1,1,1,1), // ┼
    arms(1,1,1,2), // ┽
    arms(1,2,1,1), // ┾
    arms(1,2,1,2), // ┿
    arms(2,1,1,1), // ╀
    arms(1,1,2,1), // ╁
    arms(2,1,2,1), // ╂
    arms(2,1,1,2), // ╃
    arms(2,2,1,1), // ╄
    arms(1,1,2,2), // ╅
    arms(1,2,2,1), // ╆
    arms(2,2,1,2), // ╇
    arms(1,2,2,2), // ╈
    arms(2,1,2,2), // ╉
    arms(2,2,2,1), // ╊
    arms(2,2,2,2), // ╋
    0, 0, 0, 0, // ╌╍╎╏ dashed
    arms(0,3,0,3), // ═
    arms(3,0,3,0), // ║
    arms(0,3,1,0), // ╒
    arms(0,1,3,0), // ╓
    arms(0,3,3,0), // ╔
    arms(0,0,1,3), // ╕
    arms(0,0,3,1), // ╖
    arms(0,0,3,3), // ╗
    arms(1,3,0,0), // ╘
    arms(3,1,0,0), // ╙
    arms(3,3,0,0), // ╚
    arms(1,0,0,3), // ╛
    arms(3,0,0,1), // ╜
    arms(3,0,0,3), // ╝
    arms(1,3,1,0), // ╞
    arms(3,1,3,0), // ╟
    arms(3,3,3,0), // ╠
    arms(1,0,1,3), // ╡
    arms(3,0,3,1), // ╢
    arms(3,0,3,3), // ╣
    arms(0,3,1,3), // ╤
    arms(0,1,3,1), // ╥
    arms(0,3,3,3), // ╦
    arms(1,3,0,3), // ╧
    arms(3,1,0,1), // ╨
    arms(3,3,0,3), // ╩
    arms(1,3,1,3), // ╪
    arms(3,1,3,1), // ╫
    arms(3,3,3,3), // ╬
    0, 0, 0, 0, // ╭╮╯╰ arcs
    0, 0, 0, // ╱╲╳ diagonals
    arms(0,0,0,1), // ╴
    arms(1,0,0,0), // ╵
    arms(0,1,0,0), // ╶
    arms(0,0,1,0), // ╷
    arms(0,0,0,2), // ╸
    arms(2,0,0,0), // ╹
    arms(0,2,0,0), // ╺
    arms(0,0,2,0), // ╻
    arms(0,2,0,1), // ╼
    arms(1,0,2,0), // ╽
    arms(0,1,0,2), // ╾
    arms(2,0,1,0), // ╿
];

#[inline]
pub fn contains(ch: char) -> bool {
    matches!(ch, '\u{2500}'..='\u{259F}')
}

pub fn rasterize(ch: char, metrics: Metrics) -> Option<Bitmap> {
    if !contains(ch) {
        return None;
    }

    let i = ch as usize - 0x2500;
    let g = Geom::new(metrics);
    if g.cell_width == 0 || g.cell_height == 0 {
        return None;
    }

    let mut bm = g.blank();

    match i {
        0x04..=0x0B | 0x4C..=0x4F => dashed(&mut bm, &g, i),
        0x6D..=0x70 => arc(&mut bm, &g, i - 0x6D),
        0x71..=0x73 => diagonal(&mut bm, &g, i - 0x71),
        0x80..=0x9F => blocks(&mut bm, &g, i - 0x80),
        _ => {
            let a = [
                (ARMS[i] >> 6) & 3,
                (ARMS[i] >> 4) & 3,
                (ARMS[i] >> 2) & 3,
                ARMS[i] & 3,
            ];

            if a.contains(&DOUBLE) {
                double(&mut bm, &g, a);
            } else {
                plain(&mut bm, &g, a);
            }
        }
    }

    Some(bm)
}

// arms overshoot the centre into the far edge of the perpendicular band, so a
// junction stays solid whatever the two weights are
fn plain(bm: &mut Bitmap, g: &Geom, a: [u8; 4]) {
    let vt = g.thick(a[0].max(a[2]));
    let ht = g.thick(a[1].max(a[3]));
    let (bx, by) = (g.vband(vt), g.hband(ht));

    if a[0] != NONE {
        let t = g.thick(a[0]);
        vline(bm, 0, by + ht, g.vband(t), t);
    }

    if a[2] != NONE {
        let t = g.thick(a[2]);
        vline(bm, by, g.cell_height, g.vband(t), t);
    }

    if a[3] != NONE {
        let t = g.thick(a[3]);
        hline(bm, 0, bx + vt, g.hband(t), t);
    }

    if a[1] != NONE {
        let t = g.thick(a[1]);
        hline(bm, bx, g.cell_width, g.hband(t), t);
    }
}

// a run caps the perpendicular rails that die on it and butts against the ones
// passing through - that is the whole difference between ╔ and ╠
fn double(bm: &mut Bitmap, g: &Geom, a: [u8; 4]) {
    let (up, right, down, left) = (a[0], a[1], a[2], a[3]);
    let (w, h) = (g.cell_width, g.cell_height);
    let t = g.light;

    let xl = g.vband(3 * t);
    let xr = xl + 2 * t;
    let yt = g.hband(3 * t);
    let yb = yt + 2 * t;
    let (xc, yc) = (g.vband(t), g.hband(t));

    let vd = up == DOUBLE || down == DOUBLE;
    let hd = left == DOUBLE || right == DOUBLE;

    if hd {
        let (rt, rb, lt, lb) = if vd {
            (
                if up == DOUBLE { xr } else { xl },
                if down == DOUBLE { xr } else { xl },
                if up == DOUBLE { xl + t } else { xr + t },
                if down == DOUBLE { xl + t } else { xr + t },
            )
        } else if up != NONE || down != NONE {
            (xc, xc, xc + t, xc + t)
        } else {
            (0, 0, w, w)
        };

        if right == DOUBLE {
            hline(bm, rt, w, yt, t);
            hline(bm, rb, w, yb, t);
        }

        if left == DOUBLE {
            hline(bm, 0, lt, yt, t);
            hline(bm, 0, lb, yb, t);
        }
    } else if left != NONE || right != NONE {
        let (s, e) = if up != NONE && down != NONE {
            (xr, xl + t)
        } else {
            (xl, xr + t)
        };

        if right != NONE {
            hline(bm, s, w, yc, t);
        }

        if left != NONE {
            hline(bm, 0, e, yc, t);
        }
    }

    if vd {
        let (dl, dr, ul, ur) = if hd {
            (
                if left == DOUBLE { yb } else { yt },
                if right == DOUBLE { yb } else { yt },
                if left == DOUBLE { yt + t } else { yb + t },
                if right == DOUBLE { yt + t } else { yb + t },
            )
        } else if left != NONE || right != NONE {
            (yc, yc, yc + t, yc + t)
        } else {
            (0, 0, h, h)
        };

        if down == DOUBLE {
            vline(bm, dl, h, xl, t);
            vline(bm, dr, h, xr, t);
        }

        if up == DOUBLE {
            vline(bm, 0, ul, xl, t);
            vline(bm, 0, ur, xr, t);
        }
    } else if up != NONE || down != NONE {
        let (s, e) = if left != NONE && right != NONE {
            (yb, yt + t)
        } else {
            (yt, yb + t)
        };

        if down != NONE {
            vline(bm, s, h, xc, t);
        }

        if up != NONE {
            vline(bm, 0, e, xc, t);
        }
    }
}

fn dashed(bm: &mut Bitmap, g: &Geom, i: usize) {
    let n = match i {
        0x04..=0x07 => 3,
        0x08..=0x0B => 4,
        _ => 2,
    };

    let t = if i & 1 != 0 { g.heavy } else { g.light };
    let vertical = i & 2 != 0;
    let len = if vertical {
        g.cell_height
    } else {
        g.cell_width
    };

    let gap = (len / (n * 3)).max(1);
    // dash k spans [cut(k), cut(k + 1) - gap), so the rounding slack is spread
    // over the run instead of piling up on the last dash
    let cut = |k: usize| k * (len + gap) / n;

    for k in 0..n {
        let (a, b) = (cut(k), cut(k + 1).saturating_sub(gap));

        if vertical {
            vline(bm, a, b, g.vband(t), t);
        } else {
            hline(bm, a, b, g.hband(t), t);
        }
    }
}

// quarter circle tangent to both stroke centrelines, plus the straight tails
fn arc(bm: &mut Bitmap, g: &Geom, k: usize) {
    let (w, h) = (g.cell_width, g.cell_height);
    let t = g.light;
    let (right, down) = (k == 0 || k == 3, k < 2);
    let (bx, by) = (g.vband(t), g.hband(t));

    let half = t as f32 / 2.0;
    let r = w.min(h) as f32 / 2.0;
    let cx = (bx as f32 + half + if right { r } else { -r }).max(0.0);
    let cy = (by as f32 + half + if down { r } else { -r }).max(0.0);
    let (ix, iy) = (cx as usize, cy as usize);

    if right {
        hline(bm, ix, w, by, t);
    } else {
        hline(bm, 0, ix + t, by, t);
    }

    if down {
        vline(bm, iy, h, bx, t);
    } else {
        vline(bm, 0, iy + t, bx, t);
    }

    let (r0, r1) = ((r - half).max(0.0).powi(2), (r + half).powi(2));
    let step = 1.0 / SS as f32;

    for y in 0..h {
        for x in 0..w {
            let mut hits = 0;

            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) * step;
                    let py = y as f32 + (sy as f32 + 0.5) * step;

                    if (px > cx) == right || (py > cy) == down {
                        continue;
                    }

                    let d = (px - cx).powi(2) + (py - cy).powi(2);
                    hits += (d >= r0 && d <= r1) as usize;
                }
            }

            blend(bm, x, y, hits);
        }
    }
}

fn diagonal(bm: &mut Bitmap, g: &Geom, k: usize) {
    let (w, h) = (g.cell_width as f32, g.cell_height as f32);
    // distances stay unnormalised, so the limit carries the segment length
    let lim = g.light as f32 / 2.0 * (w * w + h * h).sqrt();
    let step = 1.0 / SS as f32;

    for y in 0..g.cell_height {
        for x in 0..g.cell_width {
            let mut hits = 0;

            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) * step;
                    let py = y as f32 + (sy as f32 + 0.5) * step;

                    let down = (px * h - py * w).abs() <= lim;
                    let up = (px * h + py * w - w * h).abs() <= lim;

                    hits += match k {
                        0 => up,
                        1 => down,
                        _ => up || down,
                    } as usize;
                }
            }

            blend(bm, x, y, hits);
        }
    }
}

// upper left, upper right, lower left, lower right
#[rustfmt::skip]
const QUADS: [u8; 10] = [
    0b0100, // ▖
    0b1000, // ▗
    0b0001, // ▘
    0b1101, // ▙
    0b1001, // ▚
    0b0111, // ▛
    0b1011, // ▜
    0b0010, // ▝
    0b0110, // ▞
    0b1110, // ▟
];

fn blocks(bm: &mut Bitmap, g: &Geom, i: usize) {
    let (w, h) = (g.cell_width, g.cell_height);

    match i {
        0x00 => fill(bm, 0, 0, w, eighths(h, 4)),               // ▀
        0x01..=0x07 => fill(bm, 0, h - eighths(h, i), w, h),    // ▁▂▃▄▅▆▇
        0x08 => fill(bm, 0, 0, w, h),                           // █
        0x09..=0x0F => fill(bm, 0, 0, eighths(w, 0x10 - i), h), // ▉▊▋▌▍▎▏
        0x10 => fill(bm, eighths(w, 4), 0, w, h),               // ▐
        0x11..=0x13 => bm.data.fill(((i - 0x10) * 255 / 4) as u8), // ░▒▓
        0x14 => fill(bm, 0, 0, w, eighths(h, 1)),               // ▔
        0x15 => fill(bm, w - eighths(w, 1), 0, w, h),           // ▕
        _ => {
            let q = QUADS[i - 0x16];
            let (mx, my) = (eighths(w, 4), eighths(h, 4));

            if q & 0b0001 != 0 {
                fill(bm, 0, 0, mx, my);
            }
            if q & 0b0010 != 0 {
                fill(bm, mx, 0, w, my);
            }
            if q & 0b0100 != 0 {
                fill(bm, 0, my, mx, h);
            }
            if q & 0b1000 != 0 {
                fill(bm, mx, my, w, h);
            }
        }
    }
}

// n eighths of a span, rounded the same way everywhere so a stack of partial
// blocks in neighbouring cells lines up
fn eighths(span: usize, n: usize) -> usize {
    (span * n + 4) / 8
}

fn blend(bm: &mut Bitmap, x: usize, y: usize, hits: usize) {
    if hits == 0 {
        return;
    }

    let v = (hits * 255 / (SS * SS)) as u8;
    let p = &mut bm.data[y * bm.stride + x];
    *p = (*p).max(v);
}

fn hline(bm: &mut Bitmap, x0: usize, x1: usize, y: usize, t: usize) {
    fill(bm, x0, y, x1, y + t);
}

fn vline(bm: &mut Bitmap, y0: usize, y1: usize, x: usize, t: usize) {
    fill(bm, x, y0, x + t, y1);
}

fn fill(bm: &mut Bitmap, x0: usize, y0: usize, x1: usize, y1: usize) {
    let x1 = x1.min(bm.width);
    let y1 = y1.min(bm.height);

    if x0 >= x1 {
        return;
    }

    for y in y0..y1 {
        let row = y * bm.stride;
        bm.data[row + x0..row + x1].fill(255);
    }
}
