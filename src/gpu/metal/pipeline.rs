use objc2_foundation::NSString;
use objc2_metal::{MTLBlendFactor, MTLDevice, MTLLibrary, MTLRenderPipelineDescriptor};

use super::types::{Device, PIXEL_FORMAT, Pipeline};

pub fn glyph(device: &Device) -> Result<Pipeline, String> {
    build(
        device,
        include_str!("glyph.metal"),
        "vs_main",
        "fs_main",
        true,
    )
}

pub fn bg(device: &Device) -> Result<Pipeline, String> {
    build(device, include_str!("bg.metal"), "vs_bg", "fs_bg", false)
}

fn build(
    device: &Device,
    source: &str,
    vs_name: &str,
    fs_name: &str,
    blend: bool,
) -> Result<Pipeline, String> {
    let library = device
        .newLibraryWithSource_options_error(&NSString::from_str(source), None)
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
        attachment.setPixelFormat(PIXEL_FORMAT);

        if blend {
            attachment.setBlendingEnabled(true);
            attachment.setSourceRGBBlendFactor(MTLBlendFactor::SourceAlpha);
            attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
            attachment.setSourceAlphaBlendFactor(MTLBlendFactor::SourceAlpha);
            attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        }
    }

    device
        .newRenderPipelineStateWithDescriptor_error(&desc)
        .map_err(|e| format!("pipeline: {e}"))
}
