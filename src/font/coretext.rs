use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CGSize};
use objc2_core_graphics::CGGlyph;
use objc2_core_text::{CTFont, CTFontOrientation, CTFontUIFontType};

use crate::font::Metrics;

pub struct Font {
    inner: CFRetained<CTFont>,
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

                Ok(Self { inner })
            }
        }
    }

    pub fn family_name(&self) -> String {
        unsafe {
            self.inner
                .family_name()
                .as_str_unchecked()
                .unwrap()
                .to_string()
        }
    }

    pub fn metrics(&self) -> Result<Metrics, String> {
        let ascent = unsafe { self.inner.ascent() };
        let descent = unsafe { self.inner.descent() };
        let leading = unsafe { self.inner.leading() };

        let cell_height = (ascent + descent + leading).ceil() as f32;

        let chars: [u16; 1] = ['M' as u16];
        let mut glyphs: [CGGlyph; 1] = [0];

        let ok = unsafe {
            self.inner.glyphs_for_characters(
                NonNull::from(&chars).cast(),
                NonNull::from(&mut glyphs).cast(),
                1,
            )
        };

        if !ok {
            return Err("glyph M does not exist".to_string());
        }

        let mut advances: [CGSize; 1] = [CGSize::new(0.0, 0.0)];

        unsafe {
            self.inner.advances_for_glyphs(
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
}
