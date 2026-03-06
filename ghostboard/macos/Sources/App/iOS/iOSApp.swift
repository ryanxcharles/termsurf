import SwiftUI
import TermSurfKit

@main
struct TermSurf_iOSApp: App {
    @StateObject private var termsurf_app: TermSurf.App

    init() {
        if termsurf_init(UInt(CommandLine.argc), CommandLine.unsafeArgv) != TERMSURF_SUCCESS {
            preconditionFailure("Initialize termsurf backend failed")
        }
        _termsurf_app = StateObject(wrappedValue: TermSurf.App())
    }

    var body: some Scene {
        WindowGroup {
            iOS_TermSurfTerminal()
                .environmentObject(termsurf_app)
        }
    }
}

struct iOS_TermSurfTerminal: View {
    @EnvironmentObject private var termsurf_app: TermSurf.App

    var body: some View {
        ZStack {
            // Make sure that our background color extends to all parts of the screen
            Color(termsurf_app.config.backgroundColor).ignoresSafeArea()

            TermSurf.Terminal()
        }
    }
}

struct iOS_TermSurfInitView: View {
    @EnvironmentObject private var termsurf_app: TermSurf.App

    var body: some View {
        VStack {
            Image("AppIconImage")
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(maxHeight: 96)
            Text("TermSurf Ghostboard")
            Text("State: \(termsurf_app.readiness.rawValue)")
        }
        .padding()
    }
}
