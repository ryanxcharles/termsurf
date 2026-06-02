#![allow(dead_code)]
// Pipeline descriptor values are consumed by later renderer slices.

use crate::renderer::metal::api::{MetalVertexFormat, MetalVertexStepFunction};
use crate::renderer::shader::ImageVertex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MetalVertexAttribute {
    pub(crate) format: MetalVertexFormat,
    pub(crate) offset: usize,
    pub(crate) buffer_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MetalVertexLayout {
    pub(crate) stride: usize,
    pub(crate) step_function: MetalVertexStepFunction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MetalVertexDescriptor {
    pub(crate) attributes: Vec<MetalVertexAttribute>,
    pub(crate) layout: MetalVertexLayout,
}

pub(crate) trait MetalVertexInput {
    fn vertex_descriptor(step_function: MetalVertexStepFunction) -> MetalVertexDescriptor;
}

impl MetalVertexInput for ImageVertex {
    fn vertex_descriptor(step_function: MetalVertexStepFunction) -> MetalVertexDescriptor {
        MetalVertexDescriptor {
            attributes: vec![
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float2,
                    offset: std::mem::offset_of!(ImageVertex, grid_pos),
                    buffer_index: 0,
                },
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float2,
                    offset: std::mem::offset_of!(ImageVertex, cell_offset),
                    buffer_index: 0,
                },
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float4,
                    offset: std::mem::offset_of!(ImageVertex, source_rect),
                    buffer_index: 0,
                },
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float2,
                    offset: std::mem::offset_of!(ImageVertex, dest_size),
                    buffer_index: 0,
                },
            ],
            layout: MetalVertexLayout {
                stride: std::mem::size_of::<ImageVertex>(),
                step_function,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_vertex_descriptor_maps_fields_to_upstream_attributes() {
        let descriptor = ImageVertex::vertex_descriptor(MetalVertexStepFunction::PerVertex);

        assert_eq!(
            descriptor.attributes,
            vec![
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float2,
                    offset: std::mem::offset_of!(ImageVertex, grid_pos),
                    buffer_index: 0,
                },
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float2,
                    offset: std::mem::offset_of!(ImageVertex, cell_offset),
                    buffer_index: 0,
                },
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float4,
                    offset: std::mem::offset_of!(ImageVertex, source_rect),
                    buffer_index: 0,
                },
                MetalVertexAttribute {
                    format: MetalVertexFormat::Float2,
                    offset: std::mem::offset_of!(ImageVertex, dest_size),
                    buffer_index: 0,
                },
            ]
        );
        assert_eq!(
            descriptor.layout,
            MetalVertexLayout {
                stride: std::mem::size_of::<ImageVertex>(),
                step_function: MetalVertexStepFunction::PerVertex,
            }
        );
    }

    #[test]
    fn image_vertex_descriptor_preserves_attributes_for_per_instance_step() {
        let per_vertex = ImageVertex::vertex_descriptor(MetalVertexStepFunction::PerVertex);
        let per_instance = ImageVertex::vertex_descriptor(MetalVertexStepFunction::PerInstance);

        assert_eq!(per_instance.attributes, per_vertex.attributes);
        assert_eq!(
            per_instance.layout.stride,
            std::mem::size_of::<ImageVertex>()
        );
        assert_eq!(
            per_instance.layout.step_function,
            MetalVertexStepFunction::PerInstance
        );
    }
}
