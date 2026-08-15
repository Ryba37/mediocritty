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

pub const PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm_sRGB;
