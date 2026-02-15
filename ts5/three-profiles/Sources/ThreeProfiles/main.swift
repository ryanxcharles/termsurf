// Copyright 2025 TermSurf
// Swift Metal receiver: three-pane compositor with dynamic XPC tab creation.
// Part of Issue 503 Experiment 3: dynamic tab protocol.

import Cocoa
import Metal
import QuartzCore
import IOSurface

// MARK: - Pane indices

enum Pane: Int {
    case left = 0
    case center = 1
    case right = 2
    static let count = 3
}

func paneForTabId(_ tabId: String?) -> Pane? {
    switch tabId {
    case "left": return .left
    case "center": return .center
    case "right": return .right
    default: return nil
    }
}

// MARK: - XPC state (strong globals to prevent ARC release)

var gListener: xpc_connection_t?
var gPeers: [xpc_connection_t] = []

// Map from xpc_connection_t to pane index for tab connections.
// We use ObjectIdentifier of the connection wrapped in a helper since
// xpc_connection_t is not Hashable. Instead, we store an array of tuples.
var gTabConnections: [(conn: xpc_connection_t, pane: Pane)] = []

// Control connections: store per-profile control connections to send create_tab.
var gControlConnections: [xpc_connection_t] = []

// MARK: - Hardcoded topology for experiment

// When a profile registers, we send create_tab commands based on profile name.
func sendCreateTabCommands(controlConn: xpc_connection_t, profile: String) {
    switch profile {
    case "profile-a":
        sendCreateTab(conn: controlConn, url: "http://localhost:9407", tabId: "left")
        sendCreateTab(conn: controlConn, url: "http://localhost:9407", tabId: "center")
    case "profile-b":
        sendCreateTab(conn: controlConn, url: "http://localhost:9407", tabId: "right")
    default:
        fputs("[ThreeProfiles] Unknown profile: \(profile)\n", stderr)
    }
}

func sendCreateTab(conn: xpc_connection_t, url: String, tabId: String) {
    let msg = xpc_dictionary_create(nil, nil, 0)
    xpc_dictionary_set_string(msg, "action", "create_tab")
    xpc_dictionary_set_string(msg, "url", url)
    xpc_dictionary_set_string(msg, "tab_id", tabId)
    xpc_connection_send_message(conn, msg)
    fputs("[ThreeProfiles] Sent create_tab: tab_id=\(tabId) url=\(url)\n", stderr)
}

// MARK: - Shared state between XPC queue and main thread

let gSurfaceLock = NSLock()
var gPendingSurface: [IOSurfaceRef?] = [nil, nil, nil]
var gFrameCount: [Int] = [0, 0, 0]
var gLastLogTime: UInt64 = 0

// MARK: - Metal state

var gDevice: MTLDevice!
var gCommandQueue: MTLCommandQueue!
var gPipeline: MTLRenderPipelineState!
var gSampler: MTLSamplerState!
var gMetalLayer: CAMetalLayer!
var gCurrentTexture: [MTLTexture?] = [nil, nil, nil]

// MARK: - XPC message handler

func handleMessage(_ msg: xpc_object_t, peer: xpc_connection_t) {
    guard let action = xpc_dictionary_get_string(msg, "action") else { return }
    let actionStr = String(cString: action)

    if actionStr == "register" {
        // Control connection: profile server registered.
        let profilePtr = xpc_dictionary_get_string(msg, "profile")
        let profile = profilePtr != nil ? String(cString: profilePtr!) : "(unknown)"
        fputs("[ThreeProfiles] Profile server registered: \(profile)\n", stderr)

        gControlConnections.append(peer)

        // Send create_tab commands for this profile.
        sendCreateTabCommands(controlConn: peer, profile: profile)

    } else if actionStr == "tab_ready" {
        // Tab connection: map tab_id to pane.
        let tabIdPtr = xpc_dictionary_get_string(msg, "tab_id")
        let tabId = tabIdPtr != nil ? String(cString: tabIdPtr!) : nil
        if let pane = paneForTabId(tabId) {
            gTabConnections.append((conn: peer, pane: pane))
            fputs("[ThreeProfiles] Tab ready: tab_id=\(tabId ?? "nil") -> pane \(pane)\n", stderr)
        } else {
            fputs("[ThreeProfiles] Tab ready with unknown tab_id: \(tabId ?? "nil")\n", stderr)
        }

    } else if actionStr == "display_surface" {
        // Frame data on a tab connection. Find the pane for this connection.
        var pane: Pane? = nil
        for entry in gTabConnections {
            if entry.conn === peer {
                pane = entry.pane
                break
            }
        }
        guard let targetPane = pane else {
            fputs("[ThreeProfiles] display_surface from unknown connection\n", stderr)
            return
        }

        let port = xpc_dictionary_copy_mach_send(msg, "iosurface_port")
        guard port != MACH_PORT_NULL else {
            fputs("[ThreeProfiles] null Mach port\n", stderr)
            return
        }

        guard let surface = IOSurfaceLookupFromMachPort(port) else {
            mach_port_deallocate(mach_task_self_, port)
            fputs("[ThreeProfiles] IOSurfaceLookupFromMachPort failed\n", stderr)
            return
        }
        mach_port_deallocate(mach_task_self_, port)

        // Swap in the new surface for this pane.
        gSurfaceLock.lock()
        gPendingSurface[targetPane.rawValue] = surface
        gSurfaceLock.unlock()

        // FPS logging (per-pane counts, single log line).
        gFrameCount[targetPane.rawValue] += 1
        let now = mach_absolute_time()
        if gLastLogTime == 0 { gLastLogTime = now }
        var info = mach_timebase_info_data_t()
        mach_timebase_info(&info)
        let elapsedNs = (now - gLastLogTime) * UInt64(info.numer) / UInt64(info.denom)
        let elapsed = Double(elapsedNs) / 1_000_000_000.0
        if elapsed >= 1.0 {
            let w = IOSurfaceGetWidth(surface)
            let h = IOSurfaceGetHeight(surface)
            let fpsL = String(format: "%.1f", Double(gFrameCount[Pane.left.rawValue]) / elapsed)
            let fpsC = String(format: "%.1f", Double(gFrameCount[Pane.center.rawValue]) / elapsed)
            let fpsR = String(format: "%.1f", Double(gFrameCount[Pane.right.rawValue]) / elapsed)
            fputs("[ThreeProfiles] L: \(gFrameCount[Pane.left.rawValue]) (\(fpsL) fps) " +
                  "C: \(gFrameCount[Pane.center.rawValue]) (\(fpsC) fps) " +
                  "R: \(gFrameCount[Pane.right.rawValue]) (\(fpsR) fps) | " +
                  "IOSurface \(w)x\(h)\n", stderr)
            gFrameCount[Pane.left.rawValue] = 0
            gFrameCount[Pane.center.rawValue] = 0
            gFrameCount[Pane.right.rawValue] = 0
            gLastLogTime = now
        }
    }
}

// MARK: - XPC listener

func startXPCListener() {
    let queue = DispatchQueue(label: "com.termsurf.three-profiles.xpc")
    let listener = xpc_connection_create_mach_service(
        "com.termsurf.three-profiles",
        queue,
        UInt64(XPC_CONNECTION_MACH_SERVICE_LISTENER))

    gListener = listener

    xpc_connection_set_event_handler(listener) { peer in
        if xpc_get_type(peer) == XPC_TYPE_CONNECTION {
            let peerConn = peer as xpc_connection_t
            gPeers.append(peerConn)
            fputs("[ThreeProfiles] New connection (\(gPeers.count) total)\n", stderr)

            xpc_connection_set_event_handler(peerConn) { event in
                if xpc_get_type(event) == XPC_TYPE_DICTIONARY {
                    handleMessage(event, peer: peerConn)
                } else if xpc_get_type(event) == XPC_TYPE_ERROR {
                    if event === XPC_ERROR_CONNECTION_INVALID {
                        fputs("[ThreeProfiles] Connection closed\n", stderr)
                    } else {
                        fputs("[ThreeProfiles] XPC error\n", stderr)
                    }
                }
            }
            xpc_connection_resume(peerConn)
        } else if xpc_get_type(peer) == XPC_TYPE_ERROR {
            fputs("[ThreeProfiles] Listener error\n", stderr)
        }
    }

    xpc_connection_resume(listener)
    fputs("[ThreeProfiles] Listening on com.termsurf.three-profiles...\n", stderr)
}

// MARK: - App delegate

class ThreeProfilesAppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var displayLink: CADisplayLink?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Create window: 2400x600 logical = three 800x600 panes side by side.
        let frame = NSRect(x: 50, y: 100, width: 2400, height: 600)
        window = NSWindow(
            contentRect: frame,
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Three Profiles"
        window.makeKeyAndOrderFront(nil)

        setupMetal(view: window.contentView!)

        // Start CADisplayLink for vsync-driven rendering.
        displayLink = window.screen?.displayLink(
            target: self, selector: #selector(render))
        displayLink?.add(to: .main, forMode: .common)

        fputs("[ThreeProfiles] Window and Metal pipeline ready\n", stderr)
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
        let execPath = CommandLine.arguments[0]
        let execDir = (execPath as NSString).deletingLastPathComponent
        let libPath = "\(execDir)/shaders.metallib"
        let library: MTLLibrary
        do {
            library = try gDevice.makeLibrary(
                URL: URL(fileURLWithPath: libPath))
            fputs("[ThreeProfiles] Loaded shaders from \(libPath)\n", stderr)
        } catch {
            fputs("[ThreeProfiles] Failed to load \(libPath): \(error)\n", stderr)
            exit(1)
        }

        guard let vertexFunc = library.makeFunction(name: "vertex_main"),
              let fragmentFunc = library.makeFunction(name: "fragment_main") else {
            fputs("[ThreeProfiles] Failed to find shader functions\n", stderr)
            exit(1)
        }

        let pipelineDesc = MTLRenderPipelineDescriptor()
        pipelineDesc.vertexFunction = vertexFunc
        pipelineDesc.fragmentFunction = fragmentFunc
        pipelineDesc.colorAttachments[0].pixelFormat = .bgra8Unorm_srgb

        do {
            gPipeline = try gDevice.makeRenderPipelineState(descriptor: pipelineDesc)
        } catch {
            fputs("[ThreeProfiles] Failed to create pipeline: \(error)\n", stderr)
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
              gCurrentTexture[Pane.center.rawValue] != nil ||
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
        let thirdW = drawableW / 3.0

        // Left pane (profile-a, tab 1).
        if let tex = gCurrentTexture[Pane.left.rawValue] {
            let vp = MTLViewport(originX: 0, originY: 0,
                                 width: thirdW, height: drawableH,
                                 znear: 0, zfar: 1)
            encoder.setViewport(vp)
            encoder.setFragmentTexture(tex, index: 0)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        }

        // Center pane (profile-a, tab 2).
        if let tex = gCurrentTexture[Pane.center.rawValue] {
            let vp = MTLViewport(originX: thirdW, originY: 0,
                                 width: thirdW, height: drawableH,
                                 znear: 0, zfar: 1)
            encoder.setViewport(vp)
            encoder.setFragmentTexture(tex, index: 0)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        }

        // Right pane (profile-b, tab 1).
        if let tex = gCurrentTexture[Pane.right.rawValue] {
            let vp = MTLViewport(originX: thirdW * 2, originY: 0,
                                 width: thirdW, height: drawableH,
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
let delegate = ThreeProfilesAppDelegate()
app.delegate = delegate
app.activate(ignoringOtherApps: true)
app.run()
