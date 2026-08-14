use std::ptr::NonNull;

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::NSView;
use objc2_core_foundation::CGSize;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBuffer, MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice,
    MTLDrawable, MTLLibrary, MTLLoadAction, MTLPixelFormat, MTLPrimitiveType,
    MTLRenderCommandEncoder, MTLRenderPassDescriptor, MTLRenderPipelineDescriptor,
    MTLRenderPipelineState, MTLResourceOptions, MTLStoreAction,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

type Device = Retained<ProtocolObject<dyn MTLDevice>>;
type Queue = Retained<ProtocolObject<dyn MTLCommandQueue>>;
type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;
type Buffer = Retained<ProtocolObject<dyn MTLBuffer>>;

const PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm_sRGB;

pub struct MetalCtx {
    device: Device,
    queue: Queue,
    layer: Retained<CAMetalLayer>,
    pipeline: Pipeline,
    vertex_buffer: Buffer,
    vertex_count: usize,
    instance_buffer: Buffer,
    instance_count: usize,
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

        let positions: [[f32; 2]; 3] = [[0.0, 0.5], [-0.5, -0.5], [0.5, -0.5]];

        let vertex_buffer = unsafe {
            device.newBufferWithBytes_length_options(
                NonNull::from(&positions).cast(),
                size_of_val(&positions),
                MTLResourceOptions::StorageModeShared,
            )
        }
        .ok_or_else(|| "couldn't create vertex buffer".to_string())?;

        let offsets: [[f32; 2]; 3] = [[-0.6, 0.0], [0.0, 0.0], [0.6, 0.0]];

        let instance_buffer = unsafe {
            device.newBufferWithBytes_length_options(
                NonNull::from(&offsets).cast(),
                size_of_val(&offsets),
                MTLResourceOptions::StorageModeShared,
            )
        }
        .ok_or_else(|| "couldn't create instance buffer".to_string())?;

        Ok(Self {
            device,
            queue,
            layer,
            pipeline,
            vertex_buffer,
            vertex_count: positions.len(),
            instance_buffer,
            instance_count: offsets.len(),
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

            encoder.setVertexBuffer_offset_atIndex(Some(&self.vertex_buffer), 0, 0);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.instance_buffer), 0, 1);
            encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                MTLPrimitiveType::Triangle,
                0,
                self.vertex_count,
                self.instance_count,
            );

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
