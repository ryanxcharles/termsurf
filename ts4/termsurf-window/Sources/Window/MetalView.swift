import AppKit
import Metal
import IOSurface
import QuartzCore

class MetalView: NSView {
    private var device: MTLDevice!
    private var commandQueue: MTLCommandQueue!
    private var metalLayer: CAMetalLayer!
    private var displayLink: CVDisplayLink?
    private var pipelineState: MTLRenderPipelineState?
    private var terminalTexture: MTLTexture?
    private var browserTexture: MTLTexture?
    private var frameCount: UInt64 = 0

    override init(frame: NSRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    override var wantsUpdateLayer: Bool { true }

    override func makeBackingLayer() -> CALayer {
        let layer = CAMetalLayer()
        layer.device = MTLCreateSystemDefaultDevice()
        layer.pixelFormat = .bgra8Unorm
        layer.framebufferOnly = true
        layer.contentsScale = window?.backingScaleFactor ?? 2.0
        return layer
    }

    private func setup() {
        wantsLayer = true
        layerContentsRedrawPolicy = .duringViewResize

        metalLayer = layer as? CAMetalLayer
        guard metalLayer != nil else {
            fatalError("Failed to create CAMetalLayer")
        }

        device = metalLayer.device
        guard device != nil else {
            fatalError("No Metal device available")
        }

        commandQueue = device.makeCommandQueue()

        setupPipeline()
        startDisplayLink()
    }

    private func setupPipeline() {
        let shaderSource = """
        #include <metal_stdlib>
        using namespace metal;

        struct VertexOut {
            float4 position [[position]];
            float2 texcoord;
        };

        vertex VertexOut vertex_fullscreen(uint vid [[vertex_id]]) {
            // Full-screen triangle: 3 vertices cover the entire viewport
            float2 positions[3] = {
                float2(-1.0, -1.0),
                float2( 3.0, -1.0),
                float2(-1.0,  3.0)
            };
            // Map clip space to texture coordinates (flip Y for top-left origin)
            float2 texcoords[3] = {
                float2(0.0, 1.0),
                float2(2.0, 1.0),
                float2(0.0, -1.0)
            };
            VertexOut out;
            out.position = float4(positions[vid], 0.0, 1.0);
            out.texcoord = texcoords[vid];
            return out;
        }

        fragment float4 fragment_textured(VertexOut in [[stage_in]],
                                          texture2d<float> tex [[texture(0)]]) {
            constexpr sampler s(mag_filter::linear, min_filter::linear);
            return tex.sample(s, in.texcoord);
        }
        """

        do {
            let library = try device.makeLibrary(source: shaderSource, options: nil)
            let vertexFunc = library.makeFunction(name: "vertex_fullscreen")
            let fragmentFunc = library.makeFunction(name: "fragment_textured")

            let desc = MTLRenderPipelineDescriptor()
            desc.vertexFunction = vertexFunc
            desc.fragmentFunction = fragmentFunc
            desc.colorAttachments[0].pixelFormat = metalLayer.pixelFormat

            pipelineState = try device.makeRenderPipelineState(descriptor: desc)
            NSLog("[MetalView] Shader pipeline created")
        } catch {
            NSLog("[MetalView] Failed to create pipeline: %@", error.localizedDescription)
        }
    }

    /// Create a Metal texture backed by an IOSurface (zero-copy).
    private func makeTextureFromSurface(_ surface: IOSurface) -> MTLTexture? {
        let width = IOSurfaceGetWidth(surface)
        let height = IOSurfaceGetHeight(surface)

        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .bgra8Unorm,
            width: width,
            height: height,
            mipmapped: false
        )
        descriptor.usage = [.shaderRead]
        descriptor.storageMode = .shared

        let texture = device.makeTexture(
            descriptor: descriptor,
            iosurface: surface,
            plane: 0
        )

        if let texture = texture {
            NSLog("[MetalView] Created texture from IOSurface: %dx%d", width, height)
        } else {
            NSLog("[MetalView] Failed to create texture from IOSurface")
        }
        return texture
    }

    /// Set the terminal pane's IOSurface (left, blue).
    func setTerminalSurface(_ surface: IOSurface) {
        terminalTexture = makeTextureFromSurface(surface)
    }

    /// Set the browser pane's IOSurface (right, green).
    func setBrowserSurface(_ surface: IOSurface) {
        browserTexture = makeTextureFromSurface(surface)
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        metalLayer?.contentsScale = window?.backingScaleFactor ?? 2.0
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        let scale = window?.backingScaleFactor ?? 2.0
        metalLayer?.drawableSize = CGSize(
            width: newSize.width * scale,
            height: newSize.height * scale
        )
    }

    private func startDisplayLink() {
        CVDisplayLinkCreateWithActiveCGDisplays(&displayLink)
        guard let displayLink = displayLink else { return }

        let callback: CVDisplayLinkOutputCallback = {
            (_, _, _, _, _, userInfo) -> CVReturn in
            let view = Unmanaged<MetalView>.fromOpaque(userInfo!).takeUnretainedValue()
            DispatchQueue.main.async { view.render() }
            return kCVReturnSuccess
        }

        CVDisplayLinkSetOutputCallback(
            displayLink,
            callback,
            Unmanaged.passUnretained(self).toOpaque()
        )
        CVDisplayLinkStart(displayLink)
    }

    private func render() {
        guard let drawable = metalLayer.nextDrawable() else { return }

        let startTime = CACurrentMediaTime()

        let passDescriptor = MTLRenderPassDescriptor()
        passDescriptor.colorAttachments[0].texture = drawable.texture
        passDescriptor.colorAttachments[0].loadAction = .clear
        passDescriptor.colorAttachments[0].storeAction = .store
        passDescriptor.colorAttachments[0].clearColor = MTLClearColor(
            red: 0.15, green: 0.15, blue: 0.15, alpha: 1.0
        )

        guard let commandBuffer = commandQueue.makeCommandBuffer(),
              let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: passDescriptor)
        else { return }

        let drawableWidth = Double(drawable.texture.width)
        let drawableHeight = Double(drawable.texture.height)
        let halfWidth = drawableWidth / 2.0

        if let pipeline = pipelineState {
            encoder.setRenderPipelineState(pipeline)

            // Left pane: terminal (blue)
            if let texture = terminalTexture {
                encoder.setViewport(MTLViewport(
                    originX: 0, originY: 0,
                    width: halfWidth, height: drawableHeight,
                    znear: 0, zfar: 1
                ))
                encoder.setFragmentTexture(texture, index: 0)
                encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
            }

            // Right pane: browser (green)
            if let texture = browserTexture {
                encoder.setViewport(MTLViewport(
                    originX: halfWidth, originY: 0,
                    width: drawableWidth - halfWidth, height: drawableHeight,
                    znear: 0, zfar: 1
                ))
                encoder.setFragmentTexture(texture, index: 0)
                encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
            }
        }

        encoder.endEncoding()
        commandBuffer.present(drawable)
        commandBuffer.commit()

        // Log frame time every 60 frames
        frameCount += 1
        if frameCount % 60 == 0 {
            let elapsed = (CACurrentMediaTime() - startTime) * 1000.0
            NSLog("[MetalView] Frame %llu composite: %.2fms", frameCount, elapsed)
        }
    }

    deinit {
        if let displayLink = displayLink {
            CVDisplayLinkStop(displayLink)
        }
    }
}
