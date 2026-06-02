use crate::renderer::image::PixelFormat;
use crate::renderer::metal::api::{
    MetalPixelFormat, MetalResourceOptions, MetalStorageMode, MetalTextureUsage,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImageTextureFormat {
    Gray,
    Rgba,
    Bgra,
}

impl ImageTextureFormat {
    pub(crate) fn to_pixel_format(self, srgb: bool) -> MetalPixelFormat {
        match (self, srgb) {
            (ImageTextureFormat::Gray, false) => MetalPixelFormat::R8Unorm,
            (ImageTextureFormat::Gray, true) => MetalPixelFormat::R8UnormSrgb,
            (ImageTextureFormat::Rgba, false) => MetalPixelFormat::Rgba8Unorm,
            (ImageTextureFormat::Rgba, true) => MetalPixelFormat::Rgba8UnormSrgb,
            (ImageTextureFormat::Bgra, false) => MetalPixelFormat::Bgra8Unorm,
            (ImageTextureFormat::Bgra, true) => MetalPixelFormat::Bgra8UnormSrgb,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImageTextureOptions {
    pub(crate) pixel_format: MetalPixelFormat,
    pub(crate) resource_options: MetalResourceOptions,
    pub(crate) usage: MetalTextureUsage,
}

pub(crate) fn image_texture_options(
    format: ImageTextureFormat,
    srgb: bool,
    storage_mode: MetalStorageMode,
) -> ImageTextureOptions {
    ImageTextureOptions {
        pixel_format: format.to_pixel_format(srgb),
        resource_options: MetalResourceOptions::image(storage_mode),
        usage: MetalTextureUsage::shader_read(),
    }
}

pub(crate) fn image_texture_format_for_upload_pixel_format(
    format: PixelFormat,
) -> Option<ImageTextureFormat> {
    match format {
        PixelFormat::Rgba => Some(ImageTextureFormat::Rgba),
        PixelFormat::Gray | PixelFormat::GrayAlpha | PixelFormat::Rgb => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::metal::api::{
        MetalCpuCacheMode, MetalHazardTrackingMode, MetalStorageMode,
    };

    #[test]
    fn image_texture_format_maps_to_metal_pixel_formats() {
        assert_eq!(
            ImageTextureFormat::Gray.to_pixel_format(false),
            MetalPixelFormat::R8Unorm
        );
        assert_eq!(
            ImageTextureFormat::Gray.to_pixel_format(true),
            MetalPixelFormat::R8UnormSrgb
        );
        assert_eq!(
            ImageTextureFormat::Rgba.to_pixel_format(false),
            MetalPixelFormat::Rgba8Unorm
        );
        assert_eq!(
            ImageTextureFormat::Rgba.to_pixel_format(true),
            MetalPixelFormat::Rgba8UnormSrgb
        );
        assert_eq!(
            ImageTextureFormat::Bgra.to_pixel_format(false),
            MetalPixelFormat::Bgra8Unorm
        );
        assert_eq!(
            ImageTextureFormat::Bgra.to_pixel_format(true),
            MetalPixelFormat::Bgra8UnormSrgb
        );
    }

    #[test]
    fn image_texture_options_match_upstream_intent() {
        let options =
            image_texture_options(ImageTextureFormat::Rgba, true, MetalStorageMode::Managed);
        assert_eq!(options.pixel_format, MetalPixelFormat::Rgba8UnormSrgb);
        assert_eq!(
            options.resource_options,
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::WriteCombined,
                storage_mode: MetalStorageMode::Managed,
                hazard_tracking_mode: MetalHazardTrackingMode::Default,
            }
        );
        assert_eq!(options.resource_options.raw(), 0x11);
        assert_eq!(options.usage, MetalTextureUsage::shader_read());
        assert_eq!(options.usage.raw(), 0x1);
    }

    #[test]
    fn upload_pixel_format_bridge_accepts_only_prepared_rgba() {
        assert_eq!(
            image_texture_format_for_upload_pixel_format(PixelFormat::Rgba),
            Some(ImageTextureFormat::Rgba)
        );
        assert_eq!(
            image_texture_format_for_upload_pixel_format(PixelFormat::Gray),
            None
        );
        assert_eq!(
            image_texture_format_for_upload_pixel_format(PixelFormat::GrayAlpha),
            None
        );
        assert_eq!(
            image_texture_format_for_upload_pixel_format(PixelFormat::Rgb),
            None
        );
    }
}
