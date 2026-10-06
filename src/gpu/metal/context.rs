use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSView};
use objc2_core_foundation::CGSize;
use objc2_core_graphics::CGColor;
use objc2_foundation::{NSDictionary, NSNull, NSString};
use objc2_metal::MTLDevice;
use objc2_quartz_core::{CAMetalLayer, CATransaction, kCAGravityTopLeft};
use winit::dpi::PhysicalSize;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::color::HexColor;

use super::types::{Device, PIXEL_FORMAT, Queue};

pub struct Context {
    device: Device,
    queue: Queue,
    layer: Retained<CAMetalLayer>,
}

impl Context {
    pub fn new(window: &Window, bg: HexColor) -> Result<Self, String> {
        let device = objc2_metal::MTLCreateSystemDefaultDevice()
            .ok_or_else(|| "metal device not found".to_string())?;

        let queue = device
            .newCommandQueue()
            .ok_or_else(|| "couldn't create queue".to_string())?;

        let layer = Self::create_layer(&device, window, window.inner_size());

        // setting bg color for default winit window
        let [r, g, b] = bg.0.map(|c| c as f64 / 255.0);
        let color = CGColor::new_srgb(r, g, b, 1.0);
        layer.setBackgroundColor(Some(&color));

        if let Ok(RawWindowHandle::AppKit(h)) = window.window_handle().map(|h| h.as_raw()) {
            let view: &NSView = unsafe { h.ns_view.cast().as_ref() };
            if let Some(ns_window) = view.window() {
                let ns_color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0);
                ns_window.setBackgroundColor(Some(&ns_color));
            }
        }

        Ok(Self {
            device,
            queue,
            layer,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn layer(&self) -> &CAMetalLayer {
        &self.layer
    }

    pub fn resize(&self, width: u32, height: u32, scale_factor: f64) {
        self.layer.setContentsScale(scale_factor);
        self.layer
            .setDrawableSize(CGSize::new(width as f64, height as f64));
        CATransaction::commit();
    }

    fn create_layer(
        device: &Device,
        window: &Window,
        size: PhysicalSize<u32>,
    ) -> Retained<CAMetalLayer> {
        let layer = CAMetalLayer::new();
        layer.setDevice(Some(device));
        layer.setPixelFormat(PIXEL_FORMAT);
        layer.setPresentsWithTransaction(true);

        unsafe {
            layer.setContentsGravity(kCAGravityTopLeft);
        }

        let keys = [
            "bounds",
            "position",
            "frame",
            "contents",
            "contentsScale",
            "anchorPoint",
        ]
        .map(NSString::from_str);
        let key_refs: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
        let nulls: Vec<_> = (0..keys.len()).map(|_| NSNull::null()).collect();
        let actions = NSDictionary::from_retained_objects(&key_refs, &nulls);
        // NSNull in actions = "no animation for this key"
        let _: () = unsafe { msg_send![&*layer, setActions: &*actions] };

        let view = Self::view_of(window);
        view.setLayer(Some(&layer));
        view.setWantsLayer(true);

        layer.setContentsScale(window.scale_factor());
        layer.setDrawableSize(CGSize::new(size.width as f64, size.height as f64));

        layer
    }

    fn view_of(window: &Window) -> Retained<NSView> {
        let handle = window.window_handle().unwrap().as_raw();
        let RawWindowHandle::AppKit(handle) = handle else {
            panic!("not macos");
        };
        unsafe { Retained::retain(handle.ns_view.as_ptr().cast()).unwrap() }
    }
}
