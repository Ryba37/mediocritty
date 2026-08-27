use std::ptr::NonNull;

use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLDevice, MTLOrigin, MTLPixelFormat, MTLRegion, MTLRenderCommandEncoder, MTLSamplerDescriptor,
    MTLSamplerMinMagFilter, MTLSize, MTLTexture, MTLTextureDescriptor,
};

use crate::font::Atlas;

use super::types::{Device, Sampler, Texture};

pub struct AtlasTexture {
    texture: Texture,
    sampler: Sampler,
}

impl AtlasTexture {
    pub fn new(device: &Device, atlas: &Atlas) -> Result<Self, String> {
        Ok(Self {
            texture: create_texture(device, atlas)?,
            sampler: create_sampler(device)?,
        })
    }

    pub fn sync(&mut self, device: &Device, atlas: &mut Atlas) -> bool {
        if atlas.take_resized() {
            match create_texture(device, atlas) {
                Ok(t) => self.texture = t,
                Err(e) => {
                    eprintln!("atlas texture: {e}");
                    return false;
                }
            }

            self.upload_all(atlas);
            atlas.take_dirty();
            return true;
        }

        let dirty = atlas.take_dirty();
        if !dirty.is_empty() {
            self.upload(atlas, &dirty);
        }

        false
    }

    pub fn bind(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>) {
        unsafe {
            encoder.setFragmentTexture_atIndex(Some(&self.texture), 0);
            encoder.setFragmentSamplerState_atIndex(Some(&self.sampler), 0);
        }
    }

    fn upload(&self, atlas: &Atlas, dirty: &[u32]) {
        let row = atlas.row_bytes() as usize;
        let bpp = atlas.bpp() as usize;
        let data = atlas.data();

        debug_assert_eq!(data.len(), row * atlas.height() as usize);

        for &n in dirty {
            let (x, y, w, h) = atlas.cell_rect(n);

            let region = MTLRegion {
                origin: MTLOrigin {
                    x: x as usize,
                    y: y as usize,
                    z: 0,
                },
                size: MTLSize {
                    width: w as usize,
                    height: h as usize,
                    depth: 1,
                },
            };

            let offset = y as usize * row + x as usize * bpp;
            let ptr = NonNull::from(&data[offset]).cast();

            unsafe {
                self.texture
                    .replaceRegion_mipmapLevel_withBytes_bytesPerRow(region, 0, ptr, row);
            }
        }
    }

    fn upload_all(&self, atlas: &Atlas) {
        let row = atlas.row_bytes() as usize;
        let data = atlas.data();

        debug_assert_eq!(data.len(), row * atlas.height() as usize);

        let region = MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: atlas.width() as usize,
                height: atlas.height() as usize,
                depth: 1,
            },
        };

        unsafe {
            self.texture
                .replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                    region,
                    0,
                    NonNull::from(data).cast(),
                    row,
                );
        }
    }
}

fn create_texture(device: &Device, atlas: &Atlas) -> Result<Texture, String> {
    // the atlas knows its own format: 1 byte is a coverage mask, 4 is color.
    // sRGB so the sampler hands the shader linear rgb
    let format = if atlas.bpp() == 1 {
        MTLPixelFormat::R8Unorm
    } else {
        MTLPixelFormat::BGRA8Unorm_sRGB
    };

    let desc = unsafe {
        MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
            format,
            atlas.width() as usize,
            atlas.height() as usize,
            false,
        )
    };

    device
        .newTextureWithDescriptor(&desc)
        .ok_or_else(|| "couldn't create texture".to_string())
}

fn create_sampler(device: &Device) -> Result<Sampler, String> {
    let desc = MTLSamplerDescriptor::new();
    desc.setMinFilter(MTLSamplerMinMagFilter::Nearest);
    desc.setMagFilter(MTLSamplerMinMagFilter::Nearest);

    device
        .newSamplerStateWithDescriptor(&desc)
        .ok_or_else(|| "couldn't create sampler".to_string())
}
