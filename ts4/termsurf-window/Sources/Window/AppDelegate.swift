import AppKit
import IOSurface
import XIPC

/// Context object passed through XPC callbacks to identify which pane sent the frame.
class XPCContext {
    let delegate: AppDelegate
    let pane: String // "terminal" or "browser"

    init(delegate: AppDelegate, pane: String) {
        self.delegate = delegate
        self.pane = pane
    }
}

class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate {
    var window: NSWindow!
    var metalView: MetalView!
    // Retain XPC contexts to prevent deallocation
    var terminalContext: XPCContext!
    var browserContext: XPCContext!
    // XPC connection handles for sending resize messages
    var terminalConn: xipc_connection_t?
    var browserConn: xipc_connection_t?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let windowRect = NSRect(x: 100, y: 100, width: 800, height: 600)
        window = NSWindow(
            contentRect: windowRect,
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "TermSurf"
        window.minSize = NSSize(width: 400, height: 300)
        window.delegate = self

        metalView = MetalView(frame: windowRect)
        window.contentView = metalView

        window.center()
        window.makeKeyAndOrderFront(nil)

        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)

        let callback: xipc_frame_callback = { port, width, height, context in
            guard let context = context else { return }
            let xpcContext = Unmanaged<XPCContext>.fromOpaque(context).takeUnretainedValue()
            xpcContext.delegate.handleFrame(
                pane: xpcContext.pane, port: port, width: width, height: height
            )
        }

        // Connect to the Rust terminal service (blue, left pane)
        terminalContext = XPCContext(delegate: self, pane: "terminal")
        let terminalPtr = Unmanaged.passUnretained(terminalContext!).toOpaque()
        NSLog("[Window] Connecting to com.termsurf.ts4.terminal")
        terminalConn = xipc_connect("com.termsurf.ts4.terminal", callback, terminalPtr)

        // Connect to the C++ browser service (green, right pane)
        browserContext = XPCContext(delegate: self, pane: "browser")
        let browserPtr = Unmanaged.passUnretained(browserContext!).toOpaque()
        NSLog("[Window] Connecting to com.termsurf.ts4.browser")
        browserConn = xipc_connect("com.termsurf.ts4.browser", callback, browserPtr)
    }

    private func handleFrame(pane: String, port: mach_port_t, width: UInt32, height: UInt32) {
        NSLog("[Window] Received %@ frame: port=%u, %ux%u", pane, port, width, height)

        guard let surfacePtr = xipc_import_iosurface(port) else {
            NSLog("[Window] Failed to import IOSurface for %@", pane)
            return
        }

        // Deallocate the Mach port now that we've imported the IOSurface.
        // The IOSurface is referenced independently; the port is no longer needed.
        xipc_deallocate_port(port)

        // IOSurfaceLookupFromMachPort returns +1 retained reference
        let ioSurface = Unmanaged<IOSurface>.fromOpaque(surfacePtr).takeRetainedValue()

        NSLog("[Window] IOSurface imported for %@: %dx%d",
              pane, IOSurfaceGetWidth(ioSurface), IOSurfaceGetHeight(ioSurface))

        switch pane {
        case "terminal":
            metalView.setTerminalSurface(ioSurface)
        case "browser":
            metalView.setBrowserSurface(ioSurface)
        default:
            break
        }
    }

    // MARK: - NSWindowDelegate

    func windowDidEndLiveResize(_ notification: Notification) {
        sendResizeToChildren()
    }

    private func sendResizeToChildren() {
        guard let view = metalView else { return }
        let scale = window.backingScaleFactor
        let viewSize = view.frame.size

        // Each pane is half the window width, full height, in pixels.
        let panePixelWidth = UInt32(viewSize.width / 2.0 * scale)
        let panePixelHeight = UInt32(viewSize.height * scale)
        let scaleStr = String(format: "%.1f", scale)

        NSLog("[Window] Resize: pane=%ux%u scale=%@", panePixelWidth, panePixelHeight, scaleStr)

        if let conn = terminalConn {
            xipc_send_resize(conn, panePixelWidth, panePixelHeight, scaleStr)
        }
        if let conn = browserConn {
            xipc_send_resize(conn, panePixelWidth, panePixelHeight, scaleStr)
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }
}
