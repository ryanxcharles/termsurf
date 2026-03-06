import SwiftUI
import Cocoa

// For testing.
struct ColorizedTermSurfIconView: View {
    var body: some View {
        Image(nsImage: ColorizedTermSurfIcon(
            screenColors: [.purple, .blue],
            ghostColor: .yellow,
            frame: .aluminum
        ).makeImage()!)
    }
}
