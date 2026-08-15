use std::ptr::NonNull;

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::NSView;
use objc2_core_foundation::CGSize;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlendFactor, MTLBuffer, MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue,
    MTLDevice, MTLDrawable, MTLLibrary, MTLLoadAction, MTLOrigin, MTLPixelFormat, MTLPrimitiveType,
    MTLRegion, MTLRenderCommandEncoder, MTLRenderPassDescriptor, MTLRenderPipelineDescriptor,
    MTLRenderPipelineState, MTLResourceOptions, MTLSamplerDescriptor, MTLSamplerMinMagFilter,
    MTLSamplerState, MTLSize, MTLStoreAction, MTLTexture, MTLTextureDescriptor,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use winit::dpi::PhysicalSize;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::font::{Atlas, Metrics};

type Device = Retained<ProtocolObject<dyn MTLDevice>>;
type Queue = Retained<ProtocolObject<dyn MTLCommandQueue>>;
type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;
type Buffer = Retained<ProtocolObject<dyn MTLBuffer>>;
type Texture = Retained<ProtocolObject<dyn MTLTexture>>;
type Sampler = Retained<ProtocolObject<dyn MTLSamplerState>>;

const PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm_sRGB;

const QUAD_POSITIONS: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

const INSTANCE_OFFSETS: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]];

const INSTANCE_COLORS: [[f32; 4]; 3] = [
    [1.0, 0.3, 0.3, 1.0],
    [0.3, 1.0, 0.3, 1.0],
    [0.3, 0.3, 1.0, 1.0],
];

pub struct MetalCtx {
    device: Device,
    queue: Queue,
    layer: Retained<CAMetalLayer>,
    pipeline: Pipeline,
    vertex_buffer: Buffer,
    vertex_count: usize,
    instance_buffer: Buffer,
    instance_count: usize,
    color_buffer: Buffer,
    uniform_buffer: Buffer,
    texture: Texture,
    sampler: Sampler,
    uniforms: Uniforms,
    cell_buffer: Buffer,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Uniforms {
    cell: [f32; 2],
    screen: [f32; 2],
    atlas: [f32; 2],
    cols: u32,
    pad: u32,
}

impl MetalCtx {
    pub fn new(window: &Window, metrics: Metrics, atlas: &Atlas) -> Result<Self, String> {
        let device = objc2_metal::MTLCreateSystemDefaultDevice()
            .ok_or_else(|| "metal device not found".to_string())?;

        let queue = device
            .newCommandQueue()
            .ok_or_else(|| "couldn't create queue".to_string())?;

        let size = window.inner_size();
        let layer = Self::create_layer(&device, window, size);
        let pipeline = Self::create_pipeline(&device)?;

        let vertex_buffer = Self::make_buffer(&device, &QUAD_POSITIONS)?;
        let instance_buffer = Self::make_buffer(&device, &INSTANCE_OFFSETS)?;
        let color_buffer = Self::make_buffer(&device, &INSTANCE_COLORS)?;

        let uniforms = Uniforms {
            cell: [metrics.cell_width as f32, metrics.cell_height as f32],
            screen: [size.width as f32, size.height as f32],
            atlas: [atlas.stride() as f32, atlas.height() as f32],
            cols: atlas.cols(),
            pad: 0,
        };

        let uniform_buffer = Self::make_buffer(&device, &[uniforms])?;
        let cell_buffer = Self::make_buffer(&device, &[0u32])?;

        let texture = Self::create_texture(&device, atlas)?;
        let sampler = Self::create_sampler(&device)?;

        Ok(Self {
            device,
            queue,
            layer,
            pipeline,
            vertex_buffer,
            vertex_count: QUAD_POSITIONS.len(),
            instance_buffer,
            instance_count: INSTANCE_OFFSETS.len(),
            color_buffer,
            uniform_buffer,
            texture,
            sampler,
            uniforms,
            cell_buffer,
        })
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

        let view = Self::view_of(window);
        view.setLayer(Some(&layer));
        view.setWantsLayer(true);

        layer.setContentsScale(window.scale_factor());
        layer.setDrawableSize(CGSize::new(size.width as f64, size.height as f64));

        layer
    }

    fn create_pipeline(device: &Device) -> Result<Pipeline, String> {
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
            desc.colorAttachments()
                .objectAtIndexedSubscript(0)
                .setPixelFormat(PIXEL_FORMAT);

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

    fn create_texture(device: &Device, atlas: &Atlas) -> Result<Texture, String> {
        let desc = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::R8Unorm,
                atlas.stride() as usize,
                atlas.height() as usize,
                false,
            )
        };

        let texture = device
            .newTextureWithDescriptor(&desc)
            .ok_or_else(|| "couldn't create texture".to_string())?;

        Ok(texture)
    }

    fn upload(&self, atlas: &Atlas, dirty: &[u32]) {
        let stride = atlas.stride() as usize;
        let data = atlas.data();

        debug_assert_eq!(data.len(), stride * atlas.height() as usize);

        for &n in dirty {
            let (x, y, w, h) = atlas.cell_rect(n);
            let region = MTLRegion {
                origin: MTLOrigin {
                    x: x as usize,
                    y: y as usize,
                    z: 0,
                },
                size: MTLSize {
                    width: w as usize,
                    height: h as usize,
                    depth: 1,
                },
            };

            let offset = y as usize * stride + x as usize;
            let ptr = NonNull::from(&data[offset]).cast();

            unsafe {
                self.texture
                    .replaceRegion_mipmapLevel_withBytes_bytesPerRow(region, 0, ptr, stride);
            }
        }
    }

    pub fn sync_atlas(&mut self, atlas: &mut Atlas) {
        if atlas.take_resized() {
            self.texture = match Self::create_texture(&self.device, atlas) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("atlas texture: {e}");
                    return;
                }
            };
            self.upload_all(atlas);
            self.uniforms.atlas = [atlas.stride() as f32, atlas.height() as f32];
            self.write_uniforms();

            atlas.take_dirty();
            return;
        }

        let dirty = atlas.take_dirty();
        if !dirty.is_empty() {
            self.upload(atlas, &dirty);
        }
    }

    fn upload_all(&self, atlas: &Atlas) {
        let stride = atlas.stride() as usize;
        let data = atlas.data();

        let region = MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: stride,
                height: atlas.height() as usize,
                depth: 1,
            },
        };

        unsafe {
            self.texture
                .replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                    region,
                    0,
                    NonNull::from(data).cast(),
                    stride,
                );
        }
    }

    pub fn upload_instances(
        &mut self,
        offsets: &[[f32; 2]],
        colors: &[[f32; 4]],
        cells: &[u32],
    ) -> Result<(), String> {
        debug_assert_eq!(offsets.len(), colors.len());
        debug_assert_eq!(offsets.len(), cells.len());

        self.instance_buffer = Self::make_buffer(&self.device, offsets)?;
        self.color_buffer = Self::make_buffer(&self.device, colors)?;
        self.cell_buffer = Self::make_buffer(&self.device, cells)?;
        self.instance_count = offsets.len();

        Ok(())
    }

    fn create_sampler(device: &Device) -> Result<Sampler, String> {
        let desc = MTLSamplerDescriptor::new();
        desc.setMinFilter(MTLSamplerMinMagFilter::Nearest);
        desc.setMagFilter(MTLSamplerMinMagFilter::Nearest);

        device
            .newSamplerStateWithDescriptor(&desc)
            .ok_or_else(|| "couldn't create sampler".to_string())
    }

    fn write_uniforms(&self) {
        unsafe {
            self.uniform_buffer
                .contents()
                .cast::<Uniforms>()
                .write(self.uniforms);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.layer.setContentsScale(scale_factor);
        self.layer
            .setDrawableSize(CGSize::new(width as f64, height as f64));

        self.uniforms.screen = [width as f32, height as f32];
        self.write_uniforms();
    }

    pub fn render(&mut self) {
        let Some(drawable) = self.layer.nextDrawable() else {
            return;
        };

        unsafe {
            let descriptor = MTLRenderPassDescriptor::new();
            let attachment = descriptor.colorAttachments().objectAtIndexedSubscript(0);

            attachment.setTexture(Some(&drawable.texture()));
            attachment.setLoadAction(MTLLoadAction::Clear);
            attachment.setClearColor(MTLClearColor {
                red: 0.07,
                green: 0.08,
                blue: 0.1,
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
            encoder.setVertexBuffer_offset_atIndex(Some(&self.color_buffer), 0, 2);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.uniform_buffer), 0, 3);
            encoder.setVertexBuffer_offset_atIndex(Some(&self.cell_buffer), 0, 4);

            encoder.setFragmentTexture_atIndex(Some(&self.texture), 0);
            encoder.setFragmentSamplerState_atIndex(Some(&self.sampler), 0);

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

    fn make_buffer<T>(device: &Device, data: &[T]) -> Result<Buffer, String> {
        if data.is_empty() {
            return Err("empty buffer".to_string());
        }

        unsafe {
            device.newBufferWithBytes_length_options(
                NonNull::from(data).cast(),
                size_of_val(data),
                MTLResourceOptions::StorageModeShared,
            )
        }
        .ok_or_else(|| "couldn't create buffer".to_string())
    }
}
