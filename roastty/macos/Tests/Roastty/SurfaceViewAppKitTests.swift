@testable import Roastty
import RoasttyKit
import Testing

struct SurfaceViewAppKitTests {
    @Test(arguments: [
        ("\u{0008}", true),
        ("\u{001F}", true),
        ("\u{007F}", false),
        (" ", false),
        ("h", false),
        ("", false),
        ("\u{0009}x", false),
        ("\u{0009}\u{0009}", false),
    ])
    func suppressesOnlySingleC0ControlTextWhileComposing(
        text: String,
        expected: Bool
    ) {
        #expect(
            Roastty.SurfaceView.shouldSuppressComposingControlInput(
                text,
                composing: true
            ) == expected
        )
    }

    @Test func doesNotSuppressControlTextWhenNotComposing() {
        #expect(
            Roastty.SurfaceView.shouldSuppressComposingControlInput(
                "\u{0008}",
                composing: false
            ) == false
        )
    }

    @Test func doesNotSuppressMissingText() {
        #expect(
            Roastty.SurfaceView.shouldSuppressComposingControlInput(
                nil,
                composing: true
            ) == false
        )
    }

    @MainActor
    @Test func markedTextUpdatesPreeditImeWidthAndClears() throws {
        let app = try TestRoasttyApp()
        let surfaceView = Roastty.SurfaceView(app.app)

        surfaceView.sizeDidChange(CGSize(width: 720, height: 432))
        let cellWidth = surfaceCellWidth(surfaceView)
        #expect(cellWidth > 0)

        var width = imeWidth(surfaceView)
        #expect(width == 0)
        #expect(!surfaceView.hasMarkedText())

        surfaceView.setMarkedText(
            "かな",
            selectedRange: NSRange(location: 0, length: 2),
            replacementRange: NSRange(location: NSNotFound, length: 0)
        )

        #expect(surfaceView.hasMarkedText())
        #expect(surfaceView.markedRange() == NSRange(location: 0, length: 2))
        width = imeWidth(surfaceView)
        #expect(width == cellWidth * 4)

        surfaceView.unmarkText()

        #expect(!surfaceView.hasMarkedText())
        #expect(surfaceView.markedRange() == NSRange())
        width = imeWidth(surfaceView)
        #expect(width == 0)
    }

    @MainActor
    @Test func attributedMarkedTextUpdatesPreeditImeWidth() throws {
        let app = try TestRoasttyApp()
        let surfaceView = Roastty.SurfaceView(app.app)

        surfaceView.sizeDidChange(CGSize(width: 560, height: 336))
        let cellWidth = surfaceCellWidth(surfaceView)
        #expect(cellWidth > 0)

        surfaceView.setMarkedText(
            NSAttributedString(string: "abc"),
            selectedRange: NSRange(location: 0, length: 3),
            replacementRange: NSRange(location: NSNotFound, length: 0)
        )

        #expect(surfaceView.hasMarkedText())
        #expect(surfaceView.markedRange() == NSRange(location: 0, length: 3))
        #expect(imeWidth(surfaceView) == cellWidth * 3)
    }

    @MainActor
    private func imeWidth(_ surfaceView: Roastty.SurfaceView) -> Double {
        var x: Double = -1
        var y: Double = -1
        var width: Double = -1
        var height: Double = -1
        roastty_surface_ime_point(
            surfaceView.surfaceModel!.unsafeCValue,
            &x,
            &y,
            &width,
            &height
        )
        return width
    }

    @MainActor
    private func surfaceCellWidth(_ surfaceView: Roastty.SurfaceView) -> Double {
        let size = roastty_surface_size(surfaceView.surfaceModel!.unsafeCValue)
        return Double(size.cell_width_px)
    }

    private final class TestRoasttyApp {
        let config: TemporaryConfig
        let app: roastty_app_t

        init() throws {
            let config = try TemporaryConfig("")
            guard let rawConfig = config.config else {
                throw TestError.appCreationFailed
            }
            guard let app = roastty_app_new(nil, rawConfig) else {
                throw TestError.appCreationFailed
            }

            self.config = config
            self.app = app
        }

        deinit {
            roastty_app_free(app)
        }
    }

    private enum TestError: Error {
        case appCreationFailed
    }
}
