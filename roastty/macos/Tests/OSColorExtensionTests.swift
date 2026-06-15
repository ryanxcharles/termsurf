import AppKit
import Testing
@testable import Roastty

struct OSColorExtensionTests {
    @Test func darkenConvertsDynamicColorBeforeReadingHue() {
        let color = NSColor.windowBackgroundColor.darken(by: 0.4)

        #expect(color.usingColorSpace(.sRGB) != nil)
    }

    @Test func darkenLeavesUnconvertibleColorUnchanged() {
        let image = NSImage(size: NSSize(width: 1, height: 1))
        let color = NSColor(patternImage: image)

        #expect(color.darken(by: 0.4) === color)
    }
}
