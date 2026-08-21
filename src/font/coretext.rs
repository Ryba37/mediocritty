use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CFString, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextGetBytesPerRow, CGBitmapContextGetData, CGColorSpace,
    CGContext, CGGlyph, CGImageAlphaInfo,
};
use objc2_core_text::{
    CTFont, CTFontDescriptor, CTFontOrientation, CTFontSymbolicTraits, CTFontUIFontType,
};

use crate::font::{Bitmap, GlyphId, Metrics};

#[derive(Clone)]
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

        // an opaque context turns on quartz font smoothing, which dilates the
        // outlines before rasterizing. that is fake bold on top of whatever
        // weight the face already has, and it hits real bold faces hardest,
        // closing up counters. plain grayscale antialiasing only, please.
        CGContext::set_should_antialias(Some(&ctx), true);
        CGContext::set_allows_antialiasing(Some(&ctx), true);
        CGContext::set_should_smooth_fonts(Some(&ctx), false);
        CGContext::set_allows_font_smoothing(Some(&ctx), false);

        // every glyph lands at its own atlas slot at integer coordinates, so
        // there is nothing for subpixel placement to buy us - it only smears
        // the stems across two columns
        CGContext::set_allows_font_subpixel_positioning(Some(&ctx), false);
        CGContext::set_should_subpixel_position_fonts(Some(&ctx), false);
        CGContext::set_allows_font_subpixel_quantization(Some(&ctx), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&ctx), false);

        // rounded, because a fractional baseline blurs every single glyph in
        // the atlas vertically
        let descent = (target.cell_height as f64 - target.ascent as f64).round();
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

    pub fn postscript_name(&self) -> String {
        unsafe { self.inner.post_script_name() }.to_string()
    }

    pub fn style(&self, style: u8) -> Option<Self> {
        let traits = match style {
            0 => CTFontSymbolicTraits::empty(),
            1 => CTFontSymbolicTraits::BoldTrait,
            2 => CTFontSymbolicTraits::ItalicTrait,
            3 => CTFontSymbolicTraits::BoldTrait | CTFontSymbolicTraits::ItalicTrait,
            _ => return None,
        };

        let inner = unsafe {
            self.inner.copy_with_symbolic_traits(
                0.0,
                std::ptr::null(),
                traits,
                CTFontSymbolicTraits::BoldTrait | CTFontSymbolicTraits::ItalicTrait,
            )
        };

        inner.map(|inner| Self { inner })
    }
}

// fits the ink box into the w*h target box (one cell, or two for a wide char)
// and returns the scale plus the draw origin in the scaled user space.
// scale < 1 only when the glyph actually sticks out; such glyphs get centered
// in the box, everything else keeps its natural bearing and baseline and is
// only clamped so nothing gets cut off by the atlas
fn fit(ink: CGRect, w: f64, h: f64, baseline: f64) -> (f64, CGPoint) {
    let (iw, ih) = (ink.size.width, ink.size.height);

    // blank or degenerate glyph, nothing to fit
    if !(iw > 0.0 && ih > 0.0) {
        return (1.0, CGPoint::new(0.0, baseline));
    }

    let scale = (w / iw).min(h / ih).min(1.0);

    // a glyph that had to be shrunk is an icon, not text, so center it in the
    // box on both axes. draw_glyphs takes the baseline origin, so undo the ink
    // offset and the ctm
    if scale < 1.0 {
        let (sw, sh) = (iw * scale, ih * scale);

        return (
            scale,
            CGPoint::new(
                (w - sw) * 0.5 / scale - ink.origin.x,
                (h - sh) * 0.5 / scale - ink.origin.y,
            ),
        );
    }

    // text keeps its bearing and its baseline. the pen range is whatever keeps
    // the ink inside the slot; snapping it to whole pixels puts the outline on
    // the sample grid instead of halfway across it, which is the difference
    // between crisp stems and smeared ones
    let (lo_x, hi_x) = pen_range(ink.origin.x, w - iw);
    let (lo_y, hi_y) = pen_range(ink.origin.y, h - ih);

    (
        1.0,
        CGPoint::new(0.0_f64.clamp(lo_x, hi_x), baseline.clamp(lo_y, hi_y)),
    )
}

// whole-pixel bounds for the pen on one axis, given the ink's bearing and how
// much room the slot has to spare
fn pen_range(bearing: f64, slack: f64) -> (f64, f64) {
    let lo = (-bearing).ceil();
    let hi = (slack - bearing).floor();

    (lo, hi.max(lo))
}
