use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::NSView;
use objc2_core_foundation::CGSize;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice, MTLDrawable,
    MTLLibrary, MTLLoadAction, MTLPixelFormat, MTLPrimitiveType, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLStoreAction,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

type Device = Retained<ProtocolObject<dyn MTLDevice>>;
type Queue = Retained<ProtocolObject<dyn MTLCommandQueue>>;
type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;

const PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm_sRGB;

pub struct MetalCtx {
    device: Device,
    queue: Queue,
    layer: Retained<CAMetalLayer>,
    pipeline: Pipeline,
}

impl MetalCtx {
    pub fn new(window: &Window) -> Result<Self, String> {
        let device = objc2_metal::MTLCreateSystemDefaultDevice()
            .ok_or_else(|| "metal device not found".to_string())?;

        let queue = device
            .newCommandQueue()
            .ok_or_else(|| "couldn't create queue".to_string())?;

        let layer = CAMetalLayer::new();
        layer.setDevice(Some(&device));
        layer.setPixelFormat(PIXEL_FORMAT);
        layer.setPresentsWithTransaction(true);

        let view = Self::view_of(window);
        view.setLayer(Some(&layer));
        view.setWantsLayer(true);

        layer.setContentsScale(window.scale_factor());
        let size = window.inner_size();
        layer.setDrawableSize(CGSize::new(size.width as f64, size.height as f64));

        let source = NSString::from_str(include_str!("shaders.metal"));
        let library = device
            .newLibraryWithSource_options_error(&source, None)
            .map_err(|e| format!("shader compile: {e}"))?;

        let vs = library
            .newFunctionWithName(&NSString::from_str("vs_main"))
            .ok_or_else(|| "vs_main not found".to_string())?;

        let fs = library
            .newFunctionWithName(&NSString::from_str("fs_main"))
            .ok_or_else(|| "vf_main not found".to_string())?;

        let desc = MTLRenderPipelineDescriptor::new();
        desc.setVertexFunction(Some(&vs));
        desc.setFragmentFunction(Some(&fs));

        unsafe {
            desc.colorAttachments()
                .objectAtIndexedSubscript(0)
                .setPixelFormat(PIXEL_FORMAT);
        }

        let pipeline = device
            .newRenderPipelineStateWithDescriptor_error(&desc)
            .map_err(|e| format!("pipeline: {e}"))?;

        Ok(Self {
            device,
            queue,
            layer,
            pipeline,
        })
    }

    pub fn resize(&self, width: u32, height: u32, scale_factor: f64) {
        self.layer.setContentsScale(scale_factor);
        self.layer
            .setDrawableSize(CGSize::new(width as f64, height as f64));
    }

    pub fn render(&self) {
        let Some(drawable) = self.layer.nextDrawable() else {
            return;
        };

        unsafe {
            let descriptor = MTLRenderPassDescriptor::new();
            let attachment = descriptor.colorAttachments().objectAtIndexedSubscript(0);

            attachment.setTexture(Some(&drawable.texture()));
            attachment.setLoadAction(MTLLoadAction::Clear);
            attachment.setClearColor(MTLClearColor {
                red: 0.1,
                blue: 0.2,
                green: 0.3,
                alpha: 1.0,
            });
            attachment.setStoreAction(MTLStoreAction::Store);

            let Some(cmd) = self.queue.commandBuffer() else {
                return;
            };
            let Some(encoder) = cmd.renderCommandEncoderWithDescriptor(&descriptor) else {
                return;
            };

            encoder.setRenderPipelineState(&self.pipeline);
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);

            encoder.endEncoding();
            cmd.commit();
            cmd.waitUntilScheduled();
            drawable.present();
        }
    }

    fn view_of(window: &Window) -> Retained<NSView> {
        let handle = window.window_handle().unwrap().as_raw();
        let RawWindowHandle::AppKit(handle) = handle else {
            panic!("not macos");
        };
        unsafe { Retained::retain(handle.ns_view.as_ptr().cast()).unwrap() }
    }
}
