use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CFString, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextGetBytesPerRow, CGBitmapContextGetData, CGColorSpace,
    CGContext, CGGlyph, CGImageAlphaInfo,
};
use objc2_core_text::{CTFont, CTFontDescriptor, CTFontOrientation, CTFontUIFontType};

use crate::font::{Bitmap, GlyphId, Metrics};

pub struct Font {
    inner: CFRetained<CTFont>,
}

impl Font {
    pub fn new(name: Option<&str>, size: f64) -> Result<Self, String> {
        let inner = match name {
            Some(name) => {
                let desc = unsafe {
                    CTFontDescriptor::with_name_and_size(&CFString::from_str(name), size)
                };

                let matched = unsafe { desc.matching_font_descriptor(None) }
                    .ok_or_else(|| format!("font {name} not found"))?;

                unsafe { CTFont::with_font_descriptor(&matched, size, std::ptr::null()) }
            }
            None => unsafe {
                CTFont::new_ui_font_for_language(CTFontUIFontType::UserFixedPitch, size, None)
            }
            .ok_or_else(|| "no monospace font found".to_string())?,
        };

        Ok(Self { inner })
    }

    // only primary font needs this
    pub fn metrics(&self) -> Result<Metrics, String> {
        Self::compute_metrics(&self.inner)
    }

    pub fn rasterize_glyph(
        &self,
        glyph: GlyphId,
        target: Metrics,
        wide: bool,
    ) -> Result<Bitmap, String> {
        let width = target.cell_width as usize * if wide { 2 } else { 1 };
        let height = target.cell_height as usize;

        let space =
            CGColorSpace::new_device_gray().ok_or_else(|| "no gray color space".to_string())?;

        let ctx = unsafe {
            CGBitmapContextCreate(
                std::ptr::null_mut(),
                width,
                height,
                8,
                0,
                Some(&space),
                CGImageAlphaInfo::None.0,
            )
        }
        .ok_or_else(|| "no bitmap context".to_string())?;

        let descent = target.cell_height as f64 - target.ascent as f64;
        let glyphs = [glyph];

        CGContext::set_gray_fill_color(Some(&ctx), 1.0, 1.0);

        // advance lies for a lot of glyphs (nerd font icons with negative left
        // bearing, ambiguous width symbols), so fit by the real ink box instead
        let ink = Self::ink_bounds(&self.inner, glyph);
        let (scale, origin) = fit(ink, width as f64, height as f64, descent);

        if scale < 1.0 {
            CGContext::scale_ctm(Some(&ctx), scale, scale);
        }

        let positions = [origin];

        unsafe {
            self.inner.draw_glyphs(
                NonNull::from(&glyphs).cast(),
                NonNull::from(&positions).cast(),
                1,
                &ctx,
            );
        }

        let ptr = CGBitmapContextGetData(Some(&ctx)) as *const u8;

        if ptr.is_null() {
            return Err("no bitmap data".to_string());
        }

        let stride = CGBitmapContextGetBytesPerRow(Some(&ctx));

        if stride < width {
            return Err("bitmap stride smaller than width".to_string());
        }

        let data = unsafe { std::slice::from_raw_parts(ptr, stride * height) }.to_vec();

        Ok(Bitmap {
            data,
            width,
            height,
            stride,
        })
    }

    pub fn rasterize(&self, ch: char, target: Metrics, wide: bool) -> Result<Bitmap, String> {
        let glyph =
            Self::glyph_for(&self.inner, ch).ok_or_else(|| format!("glyph {ch} not found"))?;

        self.rasterize_glyph(glyph, target, wide)
    }

    // tight ink box of the glyph at the font's point size, relative to the
    // baseline origin, y up
    fn ink_bounds(inner: &CFRetained<CTFont>, glyph: CGGlyph) -> CGRect {
        let glyphs = [glyph];
        let mut rects = [CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0)); 1];

        unsafe {
            inner.bounding_rects_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&glyphs).cast(),
                rects.as_mut_ptr(),
                1,
            )
        };

        rects[0]
    }

    fn advance(inner: &CFRetained<CTFont>, glyph: CGGlyph) -> f64 {
        let glyphs = [glyph];
        let mut advances: [CGSize; 1] = [CGSize::new(0.0, 0.0)];

        unsafe {
            inner.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&glyphs).cast(),
                advances.as_mut_ptr(),
                1,
            )
        };

        advances[0].width
    }

    fn compute_metrics(inner: &CFRetained<CTFont>) -> Result<Metrics, String> {
        let ascent = unsafe { inner.ascent() };
        let descent = unsafe { inner.descent() };
        let leading = unsafe { inner.leading() };

        let cell_height = (ascent + descent + leading).ceil() as u32;

        let glyph = Self::glyph_for(inner, 'M').ok_or("glyph M not found")?;
        let cell_width = Self::advance(inner, glyph).ceil() as u32;

        Ok(Metrics {
            cell_width,
            cell_height,
            ascent: ascent as f32,
        })
    }

    fn glyph_for(inner: &CFRetained<CTFont>, ch: char) -> Option<CGGlyph> {
        let mut chars = [0u16; 2];
        let len = ch.encode_utf16(&mut chars).len();

        let mut glyphs: [CGGlyph; 2] = [0; 2];

        let ok = unsafe {
            inner.glyphs_for_characters(
                NonNull::from(&chars).cast(),
                NonNull::from(&mut glyphs).cast(),
                len as isize,
            )
        };

        ok.then_some(glyphs[0])
    }
}

// fits the ink box into the w*h target box (one cell, or two for a wide char)
// and returns the scale plus the draw origin in the scaled user space.
// scale < 1 only when the glyph actually sticks out; such glyphs get centered
// in the box, everything else keeps its natural bearing and baseline and is
// only clamped so nothing gets cut off by the atlas
fn fit(ink: CGRect, w: f64, h: f64, descent: f64) -> (f64, CGPoint) {
    let (iw, ih) = (ink.size.width, ink.size.height);

    // blank or degenerate glyph, nothing to fit
    if !(iw > 0.0 && ih > 0.0) {
        return (1.0, CGPoint::new(0.0, descent));
    }

    let scale = (w / iw).min(h / ih).min(1.0);
    let (sw, sh) = (iw * scale, ih * scale);

    // a glyph that had to be shrunk is an icon, not text, so center it in the
    // box on both axes. anything that fits as-is keeps its bearing and its
    // baseline, just clamped so it cannot poke out of the slot
    let (left, bottom) = if scale < 1.0 {
        ((w - sw) * 0.5, (h - sh) * 0.5)
    } else {
        (
            ink.origin.x.clamp(0.0, w - iw),
            (descent + ink.origin.y).clamp(0.0, h - ih),
        )
    };

    // draw_glyphs takes the baseline origin, so undo the ink offset and the ctm
    (
        scale,
        CGPoint::new(left / scale - ink.origin.x, bottom / scale - ink.origin.y),
    )
}
