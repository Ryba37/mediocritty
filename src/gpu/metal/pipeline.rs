use objc2_foundation::NSString;
use objc2_metal::{MTLBlendFactor, MTLDevice, MTLLibrary, MTLRenderPipelineDescriptor};

use super::types::{Device, PIXEL_FORMAT, Pipeline};

pub fn create(device: &Device) -> Result<Pipeline, String> {
    let source = NSString::from_str(include_str!("shaders.metal"));
    let library = device
        .newLibraryWithSource_options_error(&source, None)
        .map_err(|e| format!("shader compile: {e}"))?;

    let vs = library
        .newFunctionWithName(&NSString::from_str("vs_main"))
        .ok_or_else(|| "vs_main not found".to_string())?;

    let fs = library
        .newFunctionWithName(&NSString::from_str("fs_main"))
        .ok_or_else(|| "fs_main not found".to_string())?;

    let desc = MTLRenderPipelineDescriptor::new();
    desc.setVertexFunction(Some(&vs));
    desc.setFragmentFunction(Some(&fs));

    unsafe {
        let attachment = desc.colorAttachments().objectAtIndexedSubscript(0);
        attachment.setPixelFormat(PIXEL_FORMAT);
        attachment.setBlendingEnabled(true);
        attachment.setSourceRGBBlendFactor(MTLBlendFactor::SourceAlpha);
        attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        attachment.setSourceAlphaBlendFactor(MTLBlendFactor::SourceAlpha);
        attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    }

    device
        .newRenderPipelineStateWithDescriptor_error(&desc)
        .map_err(|e| format!("pipeline: {e}"))
}
