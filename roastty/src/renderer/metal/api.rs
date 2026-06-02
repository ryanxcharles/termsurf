#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub(crate) enum MetalPixelFormat {
    Invalid = 0,
    R8Unorm = 10,
    R8UnormSrgb = 11,
    Rgba8Unorm = 70,
    Rgba8UnormSrgb = 71,
    Bgra8Unorm = 80,
    Bgra8UnormSrgb = 81,
}

impl MetalPixelFormat {
    pub(crate) fn raw(self) -> u64 {
        self as u64
    }

    pub(crate) fn bytes_per_pixel(self) -> Option<usize> {
        match self {
            MetalPixelFormat::Invalid => None,
            MetalPixelFormat::R8Unorm | MetalPixelFormat::R8UnormSrgb => Some(1),
            MetalPixelFormat::Rgba8Unorm
            | MetalPixelFormat::Rgba8UnormSrgb
            | MetalPixelFormat::Bgra8Unorm
            | MetalPixelFormat::Bgra8UnormSrgb => Some(4),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MetalCpuCacheMode {
    Default = 0,
    WriteCombined = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MetalStorageMode {
    Shared = 0,
    Managed = 1,
    Private = 2,
    Memoryless = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MetalHazardTrackingMode {
    Default = 0,
    Untracked = 1,
    Tracked = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MetalResourceOptions {
    pub(crate) cpu_cache_mode: MetalCpuCacheMode,
    pub(crate) storage_mode: MetalStorageMode,
    pub(crate) hazard_tracking_mode: MetalHazardTrackingMode,
}

impl MetalResourceOptions {
    pub(crate) fn image(storage_mode: MetalStorageMode) -> Self {
        Self {
            cpu_cache_mode: MetalCpuCacheMode::WriteCombined,
            storage_mode,
            hazard_tracking_mode: MetalHazardTrackingMode::Default,
        }
    }

    pub(crate) fn raw(self) -> u64 {
        self.cpu_cache_mode as u64
            | ((self.storage_mode as u64) << 4)
            | ((self.hazard_tracking_mode as u64) << 8)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MetalTextureUsage {
    pub(crate) shader_read: bool,
    pub(crate) render_target: bool,
}

impl MetalTextureUsage {
    pub(crate) fn shader_read() -> Self {
        Self {
            shader_read: true,
            render_target: false,
        }
    }

    pub(crate) fn raw(self) -> u64 {
        (self.shader_read as u64) | ((self.render_target as u64) << 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metal_pixel_format_raw_values_match_upstream() {
        assert_eq!(MetalPixelFormat::Invalid.raw(), 0);
        assert_eq!(MetalPixelFormat::R8Unorm.raw(), 10);
        assert_eq!(MetalPixelFormat::R8UnormSrgb.raw(), 11);
        assert_eq!(MetalPixelFormat::Rgba8Unorm.raw(), 70);
        assert_eq!(MetalPixelFormat::Rgba8UnormSrgb.raw(), 71);
        assert_eq!(MetalPixelFormat::Bgra8Unorm.raw(), 80);
        assert_eq!(MetalPixelFormat::Bgra8UnormSrgb.raw(), 81);
    }

    #[test]
    fn metal_resource_options_raw_values_match_packed_layout() {
        assert_eq!(
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::WriteCombined,
                storage_mode: MetalStorageMode::Shared,
                hazard_tracking_mode: MetalHazardTrackingMode::Default,
            }
            .raw(),
            0x1
        );
        assert_eq!(
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::Default,
                storage_mode: MetalStorageMode::Managed,
                hazard_tracking_mode: MetalHazardTrackingMode::Default,
            }
            .raw(),
            0x10
        );
        assert_eq!(
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::Default,
                storage_mode: MetalStorageMode::Private,
                hazard_tracking_mode: MetalHazardTrackingMode::Default,
            }
            .raw(),
            0x20
        );
        assert_eq!(
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::Default,
                storage_mode: MetalStorageMode::Memoryless,
                hazard_tracking_mode: MetalHazardTrackingMode::Default,
            }
            .raw(),
            0x30
        );
        assert_eq!(
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::Default,
                storage_mode: MetalStorageMode::Shared,
                hazard_tracking_mode: MetalHazardTrackingMode::Untracked,
            }
            .raw(),
            0x100
        );
        assert_eq!(
            MetalResourceOptions {
                cpu_cache_mode: MetalCpuCacheMode::Default,
                storage_mode: MetalStorageMode::Shared,
                hazard_tracking_mode: MetalHazardTrackingMode::Tracked,
            }
            .raw(),
            0x200
        );
    }

    #[test]
    fn metal_texture_usage_raw_values_match_packed_layout() {
        assert_eq!(MetalTextureUsage::shader_read().raw(), 0x1);
        assert_eq!(
            MetalTextureUsage {
                shader_read: false,
                render_target: true,
            }
            .raw(),
            0x4
        );
    }

    #[test]
    fn metal_pixel_format_bytes_per_pixel_covers_supported_formats() {
        assert_eq!(MetalPixelFormat::Invalid.bytes_per_pixel(), None);
        assert_eq!(MetalPixelFormat::R8Unorm.bytes_per_pixel(), Some(1));
        assert_eq!(MetalPixelFormat::R8UnormSrgb.bytes_per_pixel(), Some(1));
        assert_eq!(MetalPixelFormat::Rgba8Unorm.bytes_per_pixel(), Some(4));
        assert_eq!(MetalPixelFormat::Rgba8UnormSrgb.bytes_per_pixel(), Some(4));
        assert_eq!(MetalPixelFormat::Bgra8Unorm.bytes_per_pixel(), Some(4));
        assert_eq!(MetalPixelFormat::Bgra8UnormSrgb.bytes_per_pixel(), Some(4));
    }
}
