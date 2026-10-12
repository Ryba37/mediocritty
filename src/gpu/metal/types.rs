use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_metal::{
    MTLBuffer, MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLRenderPipelineState, MTLSamplerState,
    MTLTexture,
};

pub type Device = Retained<ProtocolObject<dyn MTLDevice>>;
pub type Queue = Retained<ProtocolObject<dyn MTLCommandQueue>>;
pub type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;
pub type Buffer = Retained<ProtocolObject<dyn MTLBuffer>>;
pub type Texture = Retained<ProtocolObject<dyn MTLTexture>>;
pub type Sampler = Retained<ProtocolObject<dyn MTLSamplerState>>;

// opaque window blends in linear space through an sRGB target. a transparent
// one can't: WindowServer composites the layer as premultiplied gamma-space
// rgb, so linear-blended edges come out with rgb and alpha out of step and
// dark text on a light desktop gets eaten. there the target is plain unorm
// and the shaders encode to sRGB themselves (GAMMA_BLEND)
pub fn pixel_format(transparent: bool) -> MTLPixelFormat {
    if transparent {
        MTLPixelFormat::BGRA8Unorm
    } else {
        MTLPixelFormat::BGRA8Unorm_sRGB
    }
}
