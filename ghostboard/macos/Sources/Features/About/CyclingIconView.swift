import SwiftUI

/// The TermSurf application icon.
struct CyclingIconView: View {
    var body: some View {
        Image("AppIconImage")
            .resizable()
            .aspectRatio(contentMode: .fit)
            .frame(height: 128)
            .accessibilityLabel("TermSurf Application Icon")
    }
}
