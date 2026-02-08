use std::sync::{Arc, Mutex};
use termsurf_xpc::iosurface;
use termsurf_xpc::*;

fn main() {
    eprintln!("[Terminal] Starting...");

    // Step 1: Create wgpu device (reused across resizes).
    let (device, queue) = create_wgpu_device();

    // Step 2: Render initial blue IOSurface.
    let width: u32 = 800;
    let height: u32 = 600;
    let surface_addr = render_blue(&device, &queue, width, height) as usize;

    // Step 3: Set up XPC listener.
    let service_name = "com.termsurf.ts4.terminal";
    let listener =
        XpcListener::new_mach_service(service_name).expect("Failed to create XPC listener");

    let peers: Arc<Mutex<Vec<XpcConnection>>> = Arc::new(Mutex::new(Vec::new()));
    let peers_clone = peers.clone();

    // Share device and queue with event handlers.
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    set_new_connection_handler(&listener, move |peer| {
        eprintln!("[Terminal] New client connected");

        let dev = device.clone();
        let q = queue.clone();

        set_event_handler(&peer, move |event| match event {
            Ok(dict) => {
                if let Some(action) = dict.get_string("action") {
                    eprintln!("[Terminal] Received: {}", action);

                    if action == "resize" {
                        let w = dict.get_u64("width") as u32;
                        let h = dict.get_u64("height") as u32;
                        eprintln!("[Terminal] Resizing to {}x{}", w, h);

                        let new_surface = render_blue(&dev, &q, w, h);
                        let port = iosurface::create_mach_port(new_surface);

                        let msg = XpcDictionary::new();
                        msg.set_string("action", "frame");
                        msg.set_mach_send("iosurface_port", port);
                        msg.set_u64("width", w as u64);
                        msg.set_u64("height", h as u64);

                        // Send back via the connection that sent this message.
                        let remote = unsafe {
                            ffi::xpc_dictionary_get_remote_connection(dict.as_raw())
                        };
                        if !remote.is_null() {
                            unsafe {
                                ffi::xpc_connection_send_message(remote, msg.as_raw());
                            }
                        }
                        eprintln!("[Terminal] Resized frame sent: {}x{}", w, h);
                    }
                }
            }
            Err(e) => eprintln!("[Terminal] Event error: {}", e),
        });
        peer.resume();

        // Send initial frame.
        let surface = surface_addr as iosurface::IOSurfaceRef;
        let port = iosurface::create_mach_port(surface);
        eprintln!("[Terminal] Created Mach port: {}", port);

        let msg = XpcDictionary::new();
        msg.set_string("action", "frame");
        msg.set_mach_send("iosurface_port", port);
        msg.set_u64("width", width as u64);
        msg.set_u64("height", height as u64);
        peer.send(&msg);

        eprintln!("[Terminal] Frame sent: {}x{}", width, height);

        // Store peer to keep connection alive.
        peers_clone.lock().unwrap().push(peer);
    });

    listener.resume();
    eprintln!("[Terminal] Listening on {}", service_name);

    // Step 4: Block forever, processing XPC events.
    dispatch_main();
}

/// Create wgpu device and queue (Metal backend).
fn create_wgpu_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("Failed to find Metal adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("termsurf-terminal"),
            ..Default::default()
        },
        None,
    ))
    .expect("Failed to create wgpu device");

    eprintln!("[Terminal] wgpu device: {:?}", adapter.get_info().name);

    (device, queue)
}

/// Create an IOSurface, render it blue via wgpu, and return the handle.
fn render_blue(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> iosurface::IOSurfaceRef {
    let surface =
        iosurface::create_iosurface(width, height).expect("Failed to create IOSurface");
    eprintln!(
        "[Terminal] IOSurface created: {}x{}",
        iosurface::get_width(surface),
        iosurface::get_height(surface)
    );

    // Create render target
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blue-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Render blue
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("blue-render"),
    });

    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear-blue"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    // Copy GPU texture to readback buffer
    let bytes_per_row = width * 4;
    let padded_bytes_per_row = (bytes_per_row + 255) & !255;

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded_bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    // Map and copy to IOSurface
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().expect("Failed to map buffer");

    {
        let data = slice.get_mapped_range();
        iosurface::write_pixels(
            surface,
            &data,
            bytes_per_row as usize,
            padded_bytes_per_row as usize,
        );
    }
    buffer.unmap();

    // Verify
    let pixel = iosurface::read_pixel(surface, 0, 0);
    let r = (pixel >> 24) & 0xFF;
    let g = (pixel >> 16) & 0xFF;
    let b = (pixel >> 8) & 0xFF;
    let a = pixel & 0xFF;
    eprintln!("[Terminal] Rendered blue, pixel (0,0): ({r}, {g}, {b}, {a})");

    surface
}
