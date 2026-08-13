use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::NSView;
use objc2_core_foundation::CGSize;
use objc2_metal::MTLCommandBuffer;
use objc2_metal::MTLCommandEncoder;
use objc2_metal::MTLDrawable;
use objc2_metal::{MTLClearColor, MTLLoadAction, MTLStoreAction};
use objc2_metal::{MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLRenderPassDescriptor};
use objc2_quartz_core::CAMetalDrawable;
use objc2_quartz_core::CAMetalLayer;
use winit::raw_window_handle::HasWindowHandle;
use winit::{raw_window_handle::RawWindowHandle, window::Window};

type Device = Retained<ProtocolObject<dyn MTLDevice>>;
type Queue = Retained<ProtocolObject<dyn MTLCommandQueue>>;

pub struct MetalCtx {
    device: Device,
    queue: Queue,
    layer: Retained<CAMetalLayer>,
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
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm_sRGB);
        layer.setPresentsWithTransaction(true);

        let view = Self::view_of(window);
        view.setLayer(Some(&layer));
        view.setWantsLayer(true);

        layer.setContentsScale(window.scale_factor());
        let size = window.inner_size();
        layer.setDrawableSize(CGSize::new(size.width as f64, size.height as f64));

        Ok(Self {
            device,
            queue,
            layer,
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
