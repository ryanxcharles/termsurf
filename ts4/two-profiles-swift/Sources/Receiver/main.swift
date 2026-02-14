// Copyright 2025 TermSurf
// Swift Metal receiver: receives IOSurface Mach ports via XPC and renders them.
// Part of Issue 415 Experiment 2: two-pane Swift receiver.

import Cocoa
import Metal
import QuartzCore
import IOSurface

// MARK: - Pane indices

enum Pane: Int {
    case left = 0
    case right = 1
    static let count = 2
}

func paneForSession(_ sessionId: String?) -> Pane {
    if sessionId == "profile-b" { return .right }
    return .left
}

// MARK: - XPC state (strong globals to prevent ARC release)

var gListener: xpc_connection_t?
var gPeers: [xpc_connection_t] = []

// MARK: - Shared state between XPC queue and main thread

let gSurfaceLock = NSLock()
var gPendingSurface: [IOSurfaceRef?] = [nil, nil]
var gFrameCount: [Int] = [0, 0]
var gLastLogTime: UInt64 = 0

// MARK: - Metal state

var gDevice: MTLDevice!
var gCommandQueue: MTLCommandQueue!
var gPipeline: MTLRenderPipelineState!
var gSampler: MTLSamplerState!
var gMetalLayer: CAMetalLayer!
var gCurrentTexture: [MTLTexture?] = [nil, nil]

// MARK: - XPC message handler

func handleMessage(_ msg: xpc_object_t) {
    guard let action = xpc_dictionary_get_string(msg, "action") else { return }
    let actionStr = String(cString: action)

    if actionStr == "display_surface" {
        let port = xpc_dictionary_copy_mach_send(msg, "iosurface_port")
        guard port != MACH_PORT_NULL else {
            fputs("[Receiver] null Mach port\n", stderr)
            return
        }

        guard let surface = IOSurfaceLookupFromMachPort(port) else {
            mach_port_deallocate(mach_task_self_, port)
            fputs("[Receiver] IOSurfaceLookupFromMachPort failed\n", stderr)
            return
        }
        mach_port_deallocate(mach_task_self_, port)

        // Map session_id to pane.
        let sessionIdPtr = xpc_dictionary_get_string(msg, "session_id")
        let sessionId = sessionIdPtr != nil ? String(cString: sessionIdPtr!) : nil
        let pane = paneForSession(sessionId)

        // Swap in the new surface for this pane.
        gSurfaceLock.lock()
        gPendingSurface[pane.rawValue] = surface
        gSurfaceLock.unlock()

        // FPS logging (per-pane counts, single log line).
        gFrameCount[pane.rawValue] += 1
        let now = mach_absolute_time()
        if gLastLogTime == 0 { gLastLogTime = now }
        var info = mach_timebase_info_data_t()
        mach_timebase_info(&info)
        let elapsedNs = (now - gLastLogTime) * UInt64(info.numer) / UInt64(info.denom)
        let elapsed = Double(elapsedNs) / 1_000_000_000.0
        if elapsed >= 1.0 {
            let w = IOSurfaceGetWidth(surface)
            let h = IOSurfaceGetHeight(surface)
            let fpsL = Double(gFrameCount[Pane.left.rawValue]) / elapsed
            let fpsR = Double(gFrameCount[Pane.right.rawValue]) / elapsed
            let fpsLStr = String(format: "%.1f", fpsL)
            let fpsRStr = String(format: "%.1f", fpsR)
            fputs("[Receiver] L: \(gFrameCount[Pane.left.rawValue]) (\(fpsLStr) fps) " +
                  "R: \(gFrameCount[Pane.right.rawValue]) (\(fpsRStr) fps) | " +
                  "IOSurface \(w)x\(h)\n", stderr)
            gFrameCount[Pane.left.rawValue] = 0
            gFrameCount[Pane.right.rawValue] = 0
            gLastLogTime = now
        }
    } else if actionStr == "register" {
        let sessionId = xpc_dictionary_get_string(msg, "session_id")
        let sid = sessionId != nil ? String(cString: sessionId!) : "(no session_id)"
        fputs("[Receiver] Profile server registered: \(sid)\n", stderr)
    }
}

// MARK: - XPC listener

func startXPCListener() {
    let queue = DispatchQueue(label: "com.termsurf.two-profiles-swift.xpc")
    let listener = xpc_connection_create_mach_service(
        "com.termsurf.two-profiles-swift",
        queue,
        UInt64(XPC_CONNECTION_MACH_SERVICE_LISTENER))

    gListener = listener

    xpc_connection_set_event_handler(listener) { peer in
        if xpc_get_type(peer) == XPC_TYPE_CONNECTION {
            let peerConn = peer as xpc_connection_t
            gPeers.append(peerConn)
            fputs("[Receiver] Profile server connected (\(gPeers.count) total)\n", stderr)

            xpc_connection_set_event_handler(peerConn) { event in
                if xpc_get_type(event) == XPC_TYPE_DICTIONARY {
                    handleMessage(event)
                } else if xpc_get_type(event) == XPC_TYPE_ERROR {
                    if event === XPC_ERROR_CONNECTION_INVALID {
                        fputs("[Receiver] Connection closed\n", stderr)
                    } else {
                        fputs("[Receiver] XPC error\n", stderr)
                    }
                }
            }
            xpc_connection_resume(peerConn)
        } else if xpc_get_type(peer) == XPC_TYPE_ERROR {
            fputs("[Receiver] Listener error\n", stderr)
        }
    }

    xpc_connection_resume(listener)
    fputs("[Receiver] Listening on com.termsurf.two-profiles-swift...\n", stderr)
}

// MARK: - App delegate

class ReceiverAppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var displayLink: CADisplayLink?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Create window: 1600x600 logical = two 800x600 panes side by side.
        let frame = NSRect(x: 100, y: 100, width: 1600, height: 600)
        window = NSWindow(
            contentRect: frame,
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Two Profiles Swift Receiver"
        window.makeKeyAndOrderFront(nil)

        setupMetal(view: window.contentView!)

        // Start CADisplayLink for vsync-driven rendering.
        displayLink = window.screen?.displayLink(
            target: self, selector: #selector(render))
        displayLink?.add(to: .main, forMode: .common)

        fputs("[Receiver] Window and Metal pipeline ready\n", stderr)
    }

    func applicationWillTerminate(_ notification: Notification) {
        displayLink?.invalidate()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }

    // MARK: - Metal setup

    func setupMetal(view: NSView) {
        gDevice = MTLCreateSystemDefaultDevice()!
        gCommandQueue = gDevice.makeCommandQueue()!

        gMetalLayer = CAMetalLayer()
        gMetalLayer.device = gDevice
        gMetalLayer.pixelFormat = .bgra8Unorm_srgb
        gMetalLayer.framebufferOnly = false
        gMetalLayer.displaySyncEnabled = true

        // Render at Retina resolution.
        let scale = NSScreen.main!.backingScaleFactor
        gMetalLayer.contentsScale = scale
        let viewSize = view.bounds.size
        gMetalLayer.drawableSize = CGSize(
            width: viewSize.width * scale,
            height: viewSize.height * scale)

        view.wantsLayer = true
        view.layer = gMetalLayer

        // Load shaders from metallib next to the binary.
        // SPM does not compile .metal files, so we compile manually with
        // xcrun metal / xcrun metallib and place the result next to the binary.
        let execPath = CommandLine.arguments[0]
        let execDir = (execPath as NSString).deletingLastPathComponent
        let libPath = "\(execDir)/shaders.metallib"
        let library: MTLLibrary
        do {
            library = try gDevice.makeLibrary(
                URL: URL(fileURLWithPath: libPath))
            fputs("[Receiver] Loaded shaders from \(libPath)\n", stderr)
        } catch {
            fputs("[Receiver] Failed to load \(libPath): \(error)\n", stderr)
            exit(1)
        }

        guard let vertexFunc = library.makeFunction(name: "vertex_main"),
              let fragmentFunc = library.makeFunction(name: "fragment_main") else {
            fputs("[Receiver] Failed to find shader functions\n", stderr)
            exit(1)
        }

        let pipelineDesc = MTLRenderPipelineDescriptor()
        pipelineDesc.vertexFunction = vertexFunc
        pipelineDesc.fragmentFunction = fragmentFunc
        pipelineDesc.colorAttachments[0].pixelFormat = .bgra8Unorm_srgb

        do {
            gPipeline = try gDevice.makeRenderPipelineState(descriptor: pipelineDesc)
        } catch {
            fputs("[Receiver] Failed to create pipeline: \(error)\n", stderr)
            exit(1)
        }

        let samplerDesc = MTLSamplerDescriptor()
        samplerDesc.magFilter = .linear
        samplerDesc.minFilter = .linear
        gSampler = gDevice.makeSamplerState(descriptor: samplerDesc)!
    }

    // MARK: - Render

    @objc func render() {
        // Grab the latest IOSurface for each pane.
        gSurfaceLock.lock()
        let surfaces = gPendingSurface
        gSurfaceLock.unlock()

        // Update Metal textures from new IOSurfaces.
        for i in 0..<Pane.count {
            if let surface = surfaces[i] {
                let desc = MTLTextureDescriptor.texture2DDescriptor(
                    pixelFormat: .bgra8Unorm_srgb,
                    width: IOSurfaceGetWidth(surface),
                    height: IOSurfaceGetHeight(surface),
                    mipmapped: false)
                desc.usage = MTLTextureUsage.shaderRead
                let newTexture = gDevice.makeTexture(
                    descriptor: desc,
                    iosurface: surface,
                    plane: 0)
                if let newTexture = newTexture {
                    gCurrentTexture[i] = newTexture
                }
            }
        }

        // Need at least one texture to render.
        guard gCurrentTexture[Pane.left.rawValue] != nil ||
              gCurrentTexture[Pane.right.rawValue] != nil else { return }
        guard let drawable = gMetalLayer.nextDrawable() else { return }

        let passDesc = MTLRenderPassDescriptor()
        passDesc.colorAttachments[0].texture = drawable.texture
        passDesc.colorAttachments[0].loadAction = .clear
        passDesc.colorAttachments[0].storeAction = .store
        passDesc.colorAttachments[0].clearColor = MTLClearColor(
            red: 0, green: 0, blue: 0, alpha: 1)

        guard let cmdBuf = gCommandQueue.makeCommandBuffer(),
              let encoder = cmdBuf.makeRenderCommandEncoder(descriptor: passDesc) else {
            return
        }

        encoder.setRenderPipelineState(gPipeline)
        encoder.setFragmentSamplerState(gSampler, index: 0)

        let drawableW = gMetalLayer.drawableSize.width
        let drawableH = gMetalLayer.drawableSize.height
        let halfW = drawableW / 2.0

        // Left pane (profile-a).
        if let tex = gCurrentTexture[Pane.left.rawValue] {
            let vp = MTLViewport(originX: 0, originY: 0,
                                 width: halfW, height: drawableH,
                                 znear: 0, zfar: 1)
            encoder.setViewport(vp)
            encoder.setFragmentTexture(tex, index: 0)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        }

        // Right pane (profile-b).
        if let tex = gCurrentTexture[Pane.right.rawValue] {
            let vp = MTLViewport(originX: halfW, originY: 0,
                                 width: halfW, height: drawableH,
                                 znear: 0, zfar: 1)
            encoder.setViewport(vp)
            encoder.setFragmentTexture(tex, index: 0)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        }

        encoder.endEncoding()
        cmdBuf.present(drawable)
        cmdBuf.commit()
    }
}

// MARK: - Main

// Start XPC listener before NSApplication — so it's ready the instant
// launchd delivers the pending connection.
startXPCListener()

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = ReceiverAppDelegate()
app.delegate = delegate
app.activate(ignoringOtherApps: true)
app.run()
