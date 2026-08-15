use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CGPoint, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextGetData, CGColorSpace, CGContext,
    CGContextSetGrayFillColor, CGGlyph, CGImageAlphaInfo,
};
use objc2_core_text::{CTFont, CTFontOrientation, CTFontUIFontType};

use crate::font::{Bitmap, Metrics};

pub struct Font {
    inner: CFRetained<CTFont>,
    metrics: Metrics,
}

impl Font {
    pub fn new(name: Option<&str>, size: f64) -> Result<Self, String> {
        match name {
            Some(_name) => Err("not implemented yet".to_string()),
            None => {
                let inner = unsafe {
                    CTFont::new_ui_font_for_language(CTFontUIFontType::UserFixedPitch, size, None)
                }
                .ok_or_else(|| "no monospace font found".to_string())?;

                let metrics = Self::compute_metrics(&inner)?;

                Ok(Self { inner, metrics })
            }
        }
    }

    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    pub fn rasterize(&self, ch: char) -> Result<Bitmap, String> {
        let glyph =
            Self::glyph_for(&self.inner, ch).ok_or_else(|| format!("glyph {ch} not found"))?;

        let width = self.metrics.cell_width as usize;
        let height = self.metrics.cell_height as usize;

        let space =
            CGColorSpace::new_device_gray().ok_or_else(|| "no gray color space".to_string())?;

        let ctx = unsafe {
            CGBitmapContextCreate(
                std::ptr::null_mut(),
                width,
                height,
                8,
                width,
                Some(&space),
                CGImageAlphaInfo::None.0,
            )
        }
        .ok_or_else(|| "no bitmap context".to_string())?;

        let descent = self.metrics.cell_height as f64 - self.metrics.ascent as f64;
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

        let data = unsafe { std::slice::from_raw_parts(ptr, width * height) }.to_vec();

        Ok(Bitmap {
            data,
            width,
            height,
        })
    }

    fn compute_metrics(inner: &CFRetained<CTFont>) -> Result<Metrics, String> {
        let ascent = unsafe { inner.ascent() };
        let descent = unsafe { inner.descent() };
        let leading = unsafe { inner.leading() };

        let cell_height = (ascent + descent + leading).ceil() as f32;

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

        let cell_width = advances[0].width.ceil() as f32;

        Ok(Metrics {
            cell_width,
            cell_height,
            ascent: ascent as f32,
        })
    }

    fn glyph_for(inner: &CFRetained<CTFont>, ch: char) -> Option<CGGlyph> {
        let chars: [u16; 1] = [ch as u16];
        let mut glyphs: [CGGlyph; 1] = [0];

        let ok = unsafe {
            inner.glyphs_for_characters(
                NonNull::from(&chars).cast(),
                NonNull::from(&mut glyphs).cast(),
                1,
            )
        };

        ok.then_some(glyphs[0])
    }
}
