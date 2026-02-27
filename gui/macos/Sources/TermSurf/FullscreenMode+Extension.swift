import TermSurfKit

extension FullscreenMode {
    /// Initialize from a TermSurf fullscreen action.
    static func from(termsurf: termsurf_action_fullscreen_e) -> Self? {
        return switch termsurf {
        case TERMSURF_FULLSCREEN_NATIVE:
                .native

        case TERMSURF_FULLSCREEN_NON_NATIVE:
                .nonNative

        case TERMSURF_FULLSCREEN_NON_NATIVE_VISIBLE_MENU:
                .nonNativeVisibleMenu

        case TERMSURF_FULLSCREEN_NON_NATIVE_PADDED_NOTCH:
                .nonNativePaddedNotch

        default:
            nil
        }
    }
}
