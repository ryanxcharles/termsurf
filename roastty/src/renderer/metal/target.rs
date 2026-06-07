use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFType};
use objc2_core_graphics::{kCGColorSpaceDisplayP3, CGColorSpace};
use objc2_io_surface::{
    kIOSurfaceBytesPerElement, kIOSurfaceColorSpace, kIOSurfaceHeight, kIOSurfacePixelFormat,
    kIOSurfaceWidth, IOSurfacePurgeabilityState, IOSurfaceRef,
};
use objc2_metal::{MTLDevice, MTLOrigin, MTLRegion, MTLSize, MTLTexture, MTLTextureDescriptor};

use crate::renderer::metal::api::{
    MetalPixelFormat, MetalResourceOptions, MetalStorageMode, MetalTextureUsage,
};

const IOSURFACE_PIXEL_FORMAT_32_BGRA: u32 = u32::from_be_bytes(*b"BGRA");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MetalTargetOptions<'a> {
    pub(crate) device: &'a ProtocolObject<dyn MTLDevice>,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixel_format: MetalPixelFormat,
    pub(crate) storage_mode: MetalStorageMode,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MetalTargetError {
    InvalidDimensions { width: usize, height: usize },
    UnsupportedPixelFormat(MetalPixelFormat),
    DisplayP3ColorSpaceCreationFailed,
    DisplayP3PropertyListCreationFailed,
    SurfaceCreationFailed,
    TextureCreationFailed,
}

pub(crate) struct MetalTarget {
    surface: CFRetained<IOSurfaceRef>,
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    width: usize,
    height: usize,
}

impl MetalTarget {
    pub(crate) fn new(options: MetalTargetOptions<'_>) -> Result<Self, MetalTargetError> {
        if options.width == 0 || options.height == 0 {
            return Err(MetalTargetError::InvalidDimensions {
                width: options.width,
                height: options.height,
            });
        }
        if !is_supported_target_format(options.pixel_format) {
            return Err(MetalTargetError::UnsupportedPixelFormat(
                options.pixel_format,
            ));
        }

        let surface_properties = iosurface_properties(options.width, options.height)?;
        let surface_properties_erased: &CFDictionary = surface_properties.as_ref();
        let surface = unsafe { IOSurfaceRef::new(surface_properties_erased) }
            .ok_or(MetalTargetError::SurfaceCreationFailed)?;
        let color_space_plist = display_p3_property_list()?;
        unsafe {
            surface.set_value(kIOSurfaceColorSpace, color_space_plist.as_ref());
        }

        let descriptor = MTLTextureDescriptor::new();
        descriptor.setPixelFormat(options.pixel_format.to_objc());
        unsafe {
            descriptor.setWidth(options.width);
            descriptor.setHeight(options.height);
        }
        descriptor.setResourceOptions(MetalResourceOptions::image(options.storage_mode).to_objc());
        descriptor.setUsage(
            MetalTextureUsage {
                shader_read: false,
                render_target: true,
            }
            .to_objc(),
        );

        let texture = options
            .device
            .newTextureWithDescriptor_iosurface_plane(&descriptor, &surface, 0)
            .ok_or(MetalTargetError::TextureCreationFailed)?;

        Ok(Self {
            surface,
            texture,
            width: options.width,
            height: options.height,
        })
    }

    pub(crate) fn surface(&self) -> &IOSurfaceRef {
        &self.surface
    }

    pub(crate) fn texture(&self) -> &ProtocolObject<dyn MTLTexture> {
        &self.texture
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn read_bytes(&self) -> Vec<u8> {
        let bytes_per_pixel = 4;
        let mut bytes = vec![0; self.width * self.height * bytes_per_pixel];
        if !bytes.is_empty() {
            let region = MTLRegion {
                origin: MTLOrigin { x: 0, y: 0, z: 0 },
                size: MTLSize {
                    width: self.width,
                    height: self.height,
                    depth: 1,
                },
            };
            let ptr = NonNull::new(bytes.as_mut_ptr().cast()).expect("non-empty bytes");
            unsafe {
                self.texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
                    ptr,
                    self.width * bytes_per_pixel,
                    region,
                    0,
                );
            }
        }
        bytes
    }
}

impl Drop for MetalTarget {
    fn drop(&mut self) {
        unsafe {
            let _ = self.surface.set_purgeable(
                IOSurfacePurgeabilityState::PurgeableEmpty.bits(),
                std::ptr::null_mut(),
            );
        }
    }
}

fn is_supported_target_format(pixel_format: MetalPixelFormat) -> bool {
    matches!(
        pixel_format,
        MetalPixelFormat::Bgra8Unorm | MetalPixelFormat::Bgra8UnormSrgb
    )
}

fn iosurface_properties(
    width: usize,
    height: usize,
) -> Result<CFRetained<CFDictionary<CFType, CFType>>, MetalTargetError> {
    let width = CFNumber::new_isize(width as isize);
    let height = CFNumber::new_isize(height as isize);
    let pixel_format = CFNumber::new_i32(IOSURFACE_PIXEL_FORMAT_32_BGRA as i32);
    let bytes_per_element = CFNumber::new_isize(4);

    Ok(CFDictionary::from_slices(
        &[
            unsafe { kIOSurfaceWidth }.as_ref(),
            unsafe { kIOSurfaceHeight }.as_ref(),
            unsafe { kIOSurfacePixelFormat }.as_ref(),
            unsafe { kIOSurfaceBytesPerElement }.as_ref(),
        ],
        &[
            width.as_ref(),
            height.as_ref(),
            pixel_format.as_ref(),
            bytes_per_element.as_ref(),
        ],
    ))
}

fn display_p3_property_list() -> Result<CFRetained<CFType>, MetalTargetError> {
    let color_space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceDisplayP3 }))
        .ok_or(MetalTargetError::DisplayP3ColorSpaceCreationFailed)?;
    color_space
        .property_list()
        .ok_or(MetalTargetError::DisplayP3PropertyListCreationFailed)
}

#[cfg(test)]
mod tests {
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};

    use super::*;
    use crate::renderer::metal::api::MetalClearColor;
    use crate::renderer::metal::render_pass::{MetalCommandFrame, MetalRenderPassAttachment};

    fn metal_device() -> Retained<ProtocolObject<dyn MTLDevice>> {
        MTLCreateSystemDefaultDevice().expect("Roastty requires a Metal device")
    }

    fn target_options(
        device: &ProtocolObject<dyn MTLDevice>,
        width: usize,
        height: usize,
        pixel_format: MetalPixelFormat,
    ) -> MetalTargetOptions<'_> {
        MetalTargetOptions {
            device,
            width,
            height,
            pixel_format,
            storage_mode: MetalStorageMode::Shared,
        }
    }

    #[test]
    fn target_rejects_zero_width() {
        let device = metal_device();
        assert_eq!(
            MetalTarget::new(target_options(&device, 0, 1, MetalPixelFormat::Bgra8Unorm)).err(),
            Some(MetalTargetError::InvalidDimensions {
                width: 0,
                height: 1
            })
        );
    }

    #[test]
    fn target_rejects_zero_height() {
        let device = metal_device();
        assert_eq!(
            MetalTarget::new(target_options(&device, 1, 0, MetalPixelFormat::Bgra8Unorm)).err(),
            Some(MetalTargetError::InvalidDimensions {
                width: 1,
                height: 0
            })
        );
    }

    #[test]
    fn target_rejects_non_bgra_pixel_format() {
        let device = metal_device();
        assert_eq!(
            MetalTarget::new(target_options(&device, 1, 1, MetalPixelFormat::Rgba8Unorm)).err(),
            Some(MetalTargetError::UnsupportedPixelFormat(
                MetalPixelFormat::Rgba8Unorm
            ))
        );
    }

    #[test]
    fn target_creates_iosurface_backed_metal_texture() {
        let device = metal_device();
        let target = MetalTarget::new(target_options(&device, 3, 2, MetalPixelFormat::Bgra8Unorm))
            .expect("target should be created");

        assert_eq!(target.width(), 3);
        assert_eq!(target.height(), 2);
        assert_eq!(target.surface().width(), 3);
        assert_eq!(target.surface().height(), 2);
        assert_eq!(target.surface().bytes_per_element(), 4);
        assert!(target
            .surface()
            .value(unsafe { kIOSurfaceColorSpace })
            .is_some());
        assert_eq!(target.texture().width(), 3);
        assert_eq!(target.texture().height(), 2);
    }

    #[test]
    fn target_texture_can_be_render_pass_attachment() {
        let device = metal_device();
        let target = MetalTarget::new(target_options(&device, 1, 1, MetalPixelFormat::Bgra8Unorm))
            .expect("target should be created");

        let queue = device
            .newCommandQueue()
            .expect("command queue should be created");
        let frame = MetalCommandFrame::begin(&queue).expect("command frame should be created");
        let pass = frame
            .render_pass(&[MetalRenderPassAttachment {
                texture: target.texture(),
                clear_color: Some(MetalClearColor {
                    red: 0.25,
                    green: 0.5,
                    blue: 0.75,
                    alpha: 1.0,
                }),
            }])
            .expect("render pass should be created");
        pass.complete();
        frame.commit_and_wait().expect("frame should complete");

        assert_eq!(target.read_bytes(), [191, 128, 64, 255]);
    }
}
