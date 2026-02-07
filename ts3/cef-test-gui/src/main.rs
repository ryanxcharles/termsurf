//! cef-test GUI — window with split-view wgpu rendering.
//!
//! Creates a single window and renders two colored halves side by side.
//! Phase 3: validates the wgpu pipeline and quad geometry before adding
//! IOSurface import complexity in later phases.

use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

// ============================================================================
// Vertex
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Left half vertices: NDC x in [-1, 0]
const LEFT_QUAD: [Vertex; 4] = [
    Vertex { position: [-1.0,  1.0, 1.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [ 0.0,  1.0, 1.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [-1.0, -1.0, 1.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [ 0.0, -1.0, 1.0], tex_coords: [1.0, 1.0] },
];

/// Right half vertices: NDC x in [0, +1]
const RIGHT_QUAD: [Vertex; 4] = [
    Vertex { position: [ 0.0,  1.0, 1.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [ 1.0,  1.0, 1.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [ 0.0, -1.0, 1.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [ 1.0, -1.0, 1.0], tex_coords: [1.0, 1.0] },
];

// ============================================================================
// GPU State
// ============================================================================

struct GpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    left_vbuf: wgpu::Buffer,
    right_vbuf: wgpu::Buffer,
    left_bind_group: wgpu::BindGroup,
    right_bind_group: wgpu::BindGroup,
    size: winit::dpi::PhysicalSize<u32>,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(target_os = "macos")]
            backends: wgpu::Backends::from_comma_list("metal"),
            #[cfg(not(target_os = "macos"))]
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();
        let surface = instance.create_surface(window).unwrap();
        let surface_format = wgpu::TextureFormat::Bgra8Unorm;

        // Bind group layout: texture + sampler
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::OVER,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // Vertex buffers
        let left_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Left Quad"),
            contents: bytemuck::cast_slice(&LEFT_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let right_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Right Quad"),
            contents: bytemuck::cast_slice(&RIGHT_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Shared sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // Placeholder solid-color textures (BGRA format: [B, G, R, A])
        let left_bind_group = create_solid_bind_group(
            &device, &queue, &bind_group_layout, &sampler,
            [128, 0, 0, 255], // dark blue
        );
        let right_bind_group = create_solid_bind_group(
            &device, &queue, &bind_group_layout, &sampler,
            [0, 128, 0, 255], // dark green
        );

        let state = Self {
            device,
            queue,
            surface,
            surface_format,
            pipeline,
            left_vbuf,
            right_vbuf,
            left_bind_group,
            right_bind_group,
            size,
        };
        state.configure_surface();
        state
    }

    fn configure_surface(&self) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.surface_format,
                view_formats: vec![self.surface_format],
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                width: self.size.width,
                height: self.size.height,
                desired_maximum_frame_latency: 2,
                present_mode: wgpu::PresentMode::AutoVsync,
            },
        );
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.configure_surface();
        }
    }

    fn render(&self) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire swapchain texture");

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format),
                ..Default::default()
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            pass.set_pipeline(&self.pipeline);

            // Draw left half (dark blue)
            pass.set_bind_group(0, &self.left_bind_group, &[]);
            pass.set_vertex_buffer(0, self.left_vbuf.slice(..));
            pass.draw(0..4, 0..1);

            // Draw right half (dark green)
            pass.set_bind_group(0, &self.right_bind_group, &[]);
            pass.set_vertex_buffer(0, self.right_vbuf.slice(..));
            pass.draw(0..4, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

/// Create a bind group from a 1x1 solid-color texture (BGRA format).
fn create_solid_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    color_bgra: [u8; 4],
) -> wgpu::BindGroup {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Solid Color"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &color_bgra,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Solid Color Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

// ============================================================================
// App
// ============================================================================

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("cef-test")
                        .with_inner_size(winit::dpi::LogicalSize::new(1600.0, 800.0)),
                )
                .unwrap(),
        );

        let gpu = pollster::block_on(GpuState::new(window.clone()));
        println!(
            "cef-test-gui: Window created ({}x{} physical)",
            gpu.size.width, gpu.size.height
        );

        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("cef-test-gui: Window closed");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu) = &self.gpu {
                    gpu.render();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size);
                    println!("cef-test-gui: Resized to {}x{}", size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    println!("cef-test-gui: Starting...");
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        window: None,
        gpu: None,
    };
    event_loop.run_app(&mut app).unwrap();
    println!("cef-test-gui: Done");
}
