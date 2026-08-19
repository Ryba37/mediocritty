use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CFString, CGPoint, CGSize};
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

    pub fn rasterize_glyph(&self, glyph: GlyphId, target: Metrics) -> Result<Bitmap, String> {
        let width = target.cell_width as usize;
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
        let positions = [CGPoint::new(0.0, descent)];

        CGContext::set_gray_fill_color(Some(&ctx), 1.0, 1.0);

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

    pub fn rasterize(&self, ch: char, target: Metrics) -> Result<Bitmap, String> {
        let glyph =
            Self::glyph_for(&self.inner, ch).ok_or_else(|| format!("glyph {ch} not found"))?;

        self.rasterize_glyph(glyph, target)
    }

    fn compute_metrics(inner: &CFRetained<CTFont>) -> Result<Metrics, String> {
        let ascent = unsafe { inner.ascent() };
        let descent = unsafe { inner.descent() };
        let leading = unsafe { inner.leading() };

        let cell_height = (ascent + descent + leading).ceil() as u32;

        let glyph = Self::glyph_for(inner, 'M').ok_or("glyph M not found")?;
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

        let cell_width = advances[0].width.ceil() as u32;

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
