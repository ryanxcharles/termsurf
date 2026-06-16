import AppKit

// MARK: TermSurf Delegate

/// This implements the TermSurf app delegate protocol which is used by the TermSurf
/// APIs for app-global information.
extension AppDelegate: TermSurf.Delegate {
    func termsurfSurface(id: UUID) -> TermSurf.SurfaceView? {
        for window in NSApp.windows {
            guard let controller = window.windowController as? BaseTerminalController else {
                continue
            }
            
            for surface in controller.surfaceTree {
                if surface.id == id {
                    return surface
                }
            }
        }
        
        return nil
    }
}
