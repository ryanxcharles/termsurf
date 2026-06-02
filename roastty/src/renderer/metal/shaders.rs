use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{MTLDevice, MTLLibrary};

use crate::renderer::metal::api::MetalPixelFormat;
use crate::renderer::metal::pipeline::{
    standard_pipeline_build_values, MetalPipeline, MetalPipelineError, MetalPipelineOptions,
    MetalStandardPipelineDescription, STANDARD_PIPELINE_DESCRIPTIONS,
};

pub(crate) const STANDARD_METAL_SHADER_SOURCE: &str = include_str!("shaders.metal");

#[derive(Debug)]
pub(crate) struct MetalShaderLibrary {
    library: Retained<ProtocolObject<dyn MTLLibrary>>,
}

impl MetalShaderLibrary {
    pub(crate) fn compile(
        device: &ProtocolObject<dyn MTLDevice>,
    ) -> Result<Self, MetalShaderLibraryError> {
        Ok(Self {
            library: compile_source(device, STANDARD_METAL_SHADER_SOURCE)?,
        })
    }

    pub(crate) fn library(&self) -> &ProtocolObject<dyn MTLLibrary> {
        &self.library
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MetalShaderLibraryError {
    CompileFailed(String),
}

fn compile_source(
    device: &ProtocolObject<dyn MTLDevice>,
    source: &str,
) -> Result<Retained<ProtocolObject<dyn MTLLibrary>>, MetalShaderLibraryError> {
    let source = NSString::from_str(source);
    device
        .newLibraryWithSource_options_error(&source, None)
        .map_err(|error| MetalShaderLibraryError::CompileFailed(error.to_string()))
}

#[derive(Debug)]
pub(crate) struct MetalStandardPipelines {
    pub(crate) bg_color: MetalPipeline,
    pub(crate) cell_bg: MetalPipeline,
    pub(crate) cell_text: MetalPipeline,
    pub(crate) image: MetalPipeline,
    pub(crate) bg_image: MetalPipeline,
}

impl MetalStandardPipelines {
    pub(crate) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        pixel_format: MetalPixelFormat,
    ) -> Result<Self, MetalStandardPipelinesError> {
        let library = MetalShaderLibrary::compile(device)
            .map_err(MetalStandardPipelinesError::ShaderLibrary)?;
        build_from_library(device, library.library(), pixel_format)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MetalStandardPipelinesError {
    ShaderLibrary(MetalShaderLibraryError),
    MissingStandardPipeline(&'static str),
    Pipeline {
        name: &'static str,
        error: MetalPipelineError,
    },
}

fn build_from_library(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    pixel_format: MetalPixelFormat,
) -> Result<MetalStandardPipelines, MetalStandardPipelinesError> {
    Ok(MetalStandardPipelines {
        bg_color: build_standard_pipeline(device, library, "bg_color", pixel_format)?,
        cell_bg: build_standard_pipeline(device, library, "cell_bg", pixel_format)?,
        cell_text: build_standard_pipeline(device, library, "cell_text", pixel_format)?,
        image: build_standard_pipeline(device, library, "image", pixel_format)?,
        bg_image: build_standard_pipeline(device, library, "bg_image", pixel_format)?,
    })
}

fn build_standard_pipeline(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    name: &'static str,
    pixel_format: MetalPixelFormat,
) -> Result<MetalPipeline, MetalStandardPipelinesError> {
    let description = standard_pipeline_description(name)
        .ok_or(MetalStandardPipelinesError::MissingStandardPipeline(name))?;
    let values = standard_pipeline_build_values(description, pixel_format);
    MetalPipeline::new(MetalPipelineOptions {
        device,
        vertex_library: library,
        fragment_library: library,
        values,
    })
    .map_err(|error| MetalStandardPipelinesError::Pipeline { name, error })
}

fn standard_pipeline_description(name: &'static str) -> Option<MetalStandardPipelineDescription> {
    STANDARD_PIPELINE_DESCRIPTIONS
        .iter()
        .copied()
        .find(|description| description.name == name)
}

#[cfg(test)]
mod tests {
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_foundation::NSString;
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice, MTLLibrary};

    use super::{
        build_from_library, compile_source, MetalShaderLibrary, MetalShaderLibraryError,
        MetalStandardPipelines, MetalStandardPipelinesError, STANDARD_METAL_SHADER_SOURCE,
    };
    use crate::renderer::metal::api::MetalPixelFormat;
    use crate::renderer::metal::pipeline::{MetalPipelineError, STANDARD_PIPELINE_DESCRIPTIONS};

    const INCOMPATIBLE_STANDARD_SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct PositionIn {
    float2 position [[attribute(0)]];
};

struct VertexOut {
    float4 position [[position]];
};

vertex VertexOut full_screen_vertex(PositionIn in [[stage_in]]) {
    VertexOut out;
    out.position = float4(in.position, 0.0, 1.0);
    return out;
}

fragment float4 bg_color_fragment() {
    return float4(0.0, 0.0, 0.0, 1.0);
}

fragment float4 cell_bg_fragment() {
    return float4(0.0, 0.0, 0.0, 1.0);
}

struct CellTextIn {
    uint2 glyph_pos [[attribute(0)]];
    uint2 glyph_size [[attribute(1)]];
    short2 bearings [[attribute(2)]];
    ushort2 grid_pos [[attribute(3)]];
    uchar4 color [[attribute(4)]];
    uchar atlas [[attribute(5)]];
    uchar flags [[attribute(6)]];
};

vertex VertexOut cell_text_vertex(CellTextIn in [[stage_in]]) {
    VertexOut out;
    out.position = float4(float2(in.grid_pos), 0.0, 1.0);
    return out;
}

fragment float4 cell_text_fragment() {
    return float4(1.0, 1.0, 1.0, 1.0);
}

struct ImageIn {
    float2 grid_pos [[attribute(0)]];
    float2 cell_offset [[attribute(1)]];
    float4 source_rect [[attribute(2)]];
    float2 dest_size [[attribute(3)]];
};

vertex VertexOut image_vertex(ImageIn in [[stage_in]]) {
    VertexOut out;
    out.position = float4(in.grid_pos + in.cell_offset, 0.0, 1.0);
    return out;
}

fragment float4 image_fragment() {
    return float4(1.0, 1.0, 1.0, 1.0);
}

struct BgImageIn {
    float opacity [[attribute(0)]];
    uchar info [[attribute(1)]];
};

vertex VertexOut bg_image_vertex(BgImageIn in [[stage_in]]) {
    VertexOut out;
    out.position = float4(in.opacity, float(in.info), 0.0, 1.0);
    return out;
}

fragment float4 bg_image_fragment() {
    return float4(1.0, 1.0, 1.0, 1.0);
}
"#;

    fn metal_device() -> Retained<ProtocolObject<dyn MTLDevice>> {
        MTLCreateSystemDefaultDevice().expect("Roastty requires a Metal device")
    }

    fn compile_test_source(
        device: &ProtocolObject<dyn MTLDevice>,
        source: &str,
    ) -> Retained<ProtocolObject<dyn MTLLibrary>> {
        let source = NSString::from_str(source);
        device
            .newLibraryWithSource_options_error(&source, None)
            .expect("test shader source should compile")
    }

    #[test]
    fn standard_shader_source_contains_every_standard_function_name() {
        for description in STANDARD_PIPELINE_DESCRIPTIONS {
            assert!(
                STANDARD_METAL_SHADER_SOURCE.contains(description.vertex_function),
                "missing vertex function {}",
                description.vertex_function
            );
            assert!(
                STANDARD_METAL_SHADER_SOURCE.contains(description.fragment_function),
                "missing fragment function {}",
                description.fragment_function
            );
        }
    }

    #[test]
    fn standard_shader_library_compiles() {
        let device = metal_device();
        let library =
            MetalShaderLibrary::compile(&device).expect("standard shader source should compile");

        let _ = library.library();
    }

    #[test]
    fn standard_shader_library_resolves_every_pipeline_function() {
        let device = metal_device();
        let library =
            MetalShaderLibrary::compile(&device).expect("standard shader source should compile");

        for description in STANDARD_PIPELINE_DESCRIPTIONS {
            let vertex_name = NSString::from_str(description.vertex_function);
            assert!(
                library
                    .library()
                    .newFunctionWithName(&vertex_name)
                    .is_some(),
                "missing vertex function {}",
                description.vertex_function
            );

            let fragment_name = NSString::from_str(description.fragment_function);
            assert!(
                library
                    .library()
                    .newFunctionWithName(&fragment_name)
                    .is_some(),
                "missing fragment function {}",
                description.fragment_function
            );
        }
    }

    #[test]
    fn standard_pipelines_create_all_pipeline_states() {
        let device = metal_device();
        let pipelines = MetalStandardPipelines::new(&device, MetalPixelFormat::Bgra8UnormSrgb)
            .expect("standard pipelines should compile");

        let _ = (
            &pipelines.bg_color,
            &pipelines.cell_bg,
            &pipelines.cell_text,
            &pipelines.image,
            &pipelines.bg_image,
        );
    }

    #[test]
    fn invalid_shader_source_returns_compile_error() {
        let device = metal_device();
        let error = compile_source(&device, "this is not metal source")
            .expect_err("invalid source should fail");

        let MetalShaderLibraryError::CompileFailed(message) = error;
        assert!(!message.trim().is_empty());
    }

    #[test]
    fn named_pipeline_failure_preserves_pipeline_name() {
        let device = metal_device();
        let library = compile_test_source(&device, INCOMPATIBLE_STANDARD_SHADER_SOURCE);
        let error = build_from_library(&device, &library, MetalPixelFormat::Bgra8UnormSrgb)
            .expect_err("incompatible full screen vertex input should fail bg_color");

        let MetalStandardPipelinesError::Pipeline { name, error } = error else {
            panic!("expected named pipeline error");
        };
        assert_eq!(name, "bg_color");
        let MetalPipelineError::PipelineCreationFailed(message) = error else {
            panic!("expected pipeline creation error");
        };
        assert!(!message.trim().is_empty());
    }
}
