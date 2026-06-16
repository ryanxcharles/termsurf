import Foundation

extension TermSurf {
    /// This is a delegate that should be applied to your global app delegate for TermSurfKit
    /// to perform app-global operations.
    protocol Delegate {
        /// Look up a surface within the application by ID.
        func termsurfSurface(id: UUID) -> SurfaceView?
    }
}
