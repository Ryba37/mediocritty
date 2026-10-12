use objc2_foundation::NSString;
use objc2_metal::{MTLBlendFactor, MTLDevice, MTLLibrary, MTLRenderPipelineDescriptor};

use super::types::{Device, Pipeline, pixel_format};

pub fn glyph(device: &Device, opacity: f32) -> Result<Pipeline, String> {
    build(
        device,
        opacity,
        include_str!("glyph.metal"),
        "vs_main",
        "fs_main",
        true,
    )
}

pub fn emoji(device: &Device, opacity: f32) -> Result<Pipeline, String> {
    build(
        device,
        opacity,
        include_str!("emoji.metal"),
        "vs_emoji",
        "fs_emoji",
        true,
    )
}

pub fn bg(device: &Device, opacity: f32) -> Result<Pipeline, String> {
    build(
        device,
        opacity,
        include_str!("bg.metal"),
        "vs_bg",
        "fs_bg",
        false,
    )
}

pub fn underline(device: &Device, opacity: f32) -> Result<Pipeline, String> {
    build(
        device,
        opacity,
        include_str!("underline.metal"),
        "vs_underline",
        "fs_underline",
        true,
    )
}

// GAMMA_WEIGHT scales the glyph gamma curve: it's tuned against the theme bg,
// but at low opacity most of what's behind the text is the desktop
fn prelude(opacity: f32) -> String {
    if opacity < 1.0 {
        format!("#define GAMMA_BLEND\n#define GAMMA_WEIGHT {opacity:.4}\n")
    } else {
        "#define GAMMA_WEIGHT 1.0\n".to_string()
    }
}

fn build(
    device: &Device,
    opacity: f32,
    source: &str,
    vs_name: &str,
    fs_name: &str,
    blend: bool,
) -> Result<Pipeline, String> {
    let library = device
        .newLibraryWithSource_options_error(&NSString::from_str(&(prelude(opacity) + source)), None)
        .map_err(|e| format!("shader compile: {e}"))?;

    let vs = library
        .newFunctionWithName(&NSString::from_str(vs_name))
        .ok_or_else(|| format!("{vs_name} not found"))?;

    let fs = library
        .newFunctionWithName(&NSString::from_str(fs_name))
        .ok_or_else(|| format!("{fs_name} not found"))?;

    let desc = MTLRenderPipelineDescriptor::new();
    desc.setVertexFunction(Some(&vs));
    desc.setFragmentFunction(Some(&fs));

    unsafe {
        let attachment = desc.colorAttachments().objectAtIndexedSubscript(0);
        attachment.setPixelFormat(pixel_format(opacity < 1.0));

        if blend {
            attachment.setBlendingEnabled(true);
            attachment.setSourceRGBBlendFactor(MTLBlendFactor::SourceAlpha);
            attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
            // the drawable is composited as premultiplied, so alpha is
            // a + dst*(1-a). SourceAlpha here gave a*a and thinned every
            // AA edge once the window went transparent
            attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
            attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        }
    }

    device
        .newRenderPipelineStateWithDescriptor_error(&desc)
        .map_err(|e| format!("pipeline: {e}"))
}
