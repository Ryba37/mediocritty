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

use super::types::{Device, Queue, pixel_format};

pub struct Context {
    device: Device,
    queue: Queue,
    layer: Retained<CAMetalLayer>,
}

impl Context {
    pub fn new(window: &Window, bg: HexColor, opacity: f32, blur: u32) -> Result<Self, String> {
        let device = objc2_metal::MTLCreateSystemDefaultDevice()
            .ok_or_else(|| "metal device not found".to_string())?;

        let queue = device
            .newCommandQueue()
            .ok_or_else(|| "couldn't create queue".to_string())?;

        let transparent = opacity < 1.0;
        let layer = Self::create_layer(&device, window, window.inner_size(), transparent);
        layer.setOpaque(!transparent);

        // layer bg sits under the drawable: fine as a resize filler when
        // opaque, but with transparency it stacks on the clear color and
        // doubles the configured opacity
        let [r, g, b] = bg.0.map(|c| c as f64 / 255.0);
        if !transparent {
            layer.setBackgroundColor(Some(&CGColor::new_srgb(r, g, b, 1.0)));
        }

        if let Ok(RawWindowHandle::AppKit(h)) = window.window_handle().map(|h| h.as_raw()) {
            let view: &NSView = unsafe { h.ns_view.cast().as_ref() };
            if let Some(ns_window) = view.window() {
                ns_window.setOpaque(!transparent);
                let ns_color = if transparent {
                    NSColor::clearColor()
                } else {
                    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
                };
                ns_window.setBackgroundColor(Some(&ns_color));
            }
        }

        let ctx = Self {
            device,
            queue,
            layer,
        };
        ctx.set_blur(window, if transparent { blur } else { 0 });

        Ok(ctx)
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
        transparent: bool,
    ) -> Retained<CAMetalLayer> {
        let layer = CAMetalLayer::new();
        layer.setDevice(Some(device));
        layer.setPixelFormat(pixel_format(transparent));
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

    pub fn set_blur(&self, window: &Window, radius: u32) {
        unsafe extern "C" {
            fn CGSMainConnectionID() -> u32;
            fn CGSSetWindowBackgroundBlurRadius(cid: u32, wid: isize, radius: i32) -> i32;
        }

        if let Some(ns_window) = Self::view_of(window).window() {
            unsafe {
                CGSSetWindowBackgroundBlurRadius(
                    CGSMainConnectionID(),
                    ns_window.windowNumber(),
                    radius as i32,
                );
            }
        }
    }
}
