import os
import SwiftUI
import TermSurfKit

struct TermSurf {
    // The primary logger used by the TermSurfKit libraries.
    static let logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!,
        category: "termsurf"
    )

    // All the notifications that will be emitted will be put here.
    struct Notification {}

    // The user notification category identifier
    static let userNotificationCategory = "com.termsurf.userNotification"

    // The user notification "Show" action
    static let userNotificationActionShow = "com.termsurf.userNotification.Show"
}

// MARK: C Extensions

/// A command is fully self-contained so it is Sendable.
extension termsurf_command_s: @unchecked @retroactive Sendable {}

/// A surface is sendable because it is just a reference type. Using the surface in parameters
/// may be unsafe but the value itself is safe to send across threads.
extension termsurf_surface_t: @unchecked @retroactive Sendable {}

// MARK: Build Info

extension TermSurf {
    struct Info {
        var mode: termsurf_build_mode_e
        var version: String
    }

    static var info: Info {
        let raw = termsurf_info()
        let version = NSString(
            bytes: raw.version,
            length: Int(raw.version_len),
            encoding: NSUTF8StringEncoding
        ) ?? "unknown"

        return Info(mode: raw.build_mode, version: String(version))
    }
}

// MARK: General Helpers

extension TermSurf {
    enum LaunchSource: String {
        case cli
        case app
        case zig_run
    }

    /// Returns the mechanism that launched the app. This is based on an env var so
    /// its up to the env var being set in the correct circumstance.
    static var launchSource: LaunchSource {
        guard let envValue = ProcessInfo.processInfo.environment["TERMSURF_MAC_LAUNCH_SOURCE"] else {
            // We default to the CLI because the app bundle always sets the
            // source. If its unset we assume we're in a CLI environment.
            return .cli
        }

        // If the env var is set but its unknown then we default back to the app.
        return LaunchSource(rawValue: envValue) ?? .app
    }
}

// MARK: Swift Types for C Types

extension TermSurf {
    class AllocatedString {
        private let cString: termsurf_string_s

        init(_ c: termsurf_string_s) {
            self.cString = c
        }

        var string: String {
            guard let ptr = cString.ptr else { return "" }
            let data = Data(bytes: ptr, count: Int(cString.len))
            return String(data: data, encoding: .utf8) ?? ""
        }

        deinit {
            termsurf_string_free(cString)
        }
    }
}

extension TermSurf {
    enum SetFloatWIndow {
        case on
        case off
        case toggle

        static func from(_ c: termsurf_action_float_window_e) -> Self? {
            switch (c) {
            case TERMSURF_FLOAT_WINDOW_ON:
                return .on

            case TERMSURF_FLOAT_WINDOW_OFF:
                return .off

            case TERMSURF_FLOAT_WINDOW_TOGGLE:
                return .toggle

            default:
                return nil
            }
        }
    }

    enum SetSecureInput {
        case on
        case off
        case toggle

        static func from(_ c: termsurf_action_secure_input_e) -> Self? {
            switch (c) {
            case TERMSURF_SECURE_INPUT_ON:
                return .on

            case TERMSURF_SECURE_INPUT_OFF:
                return .off

            case TERMSURF_SECURE_INPUT_TOGGLE:
                return .toggle

            default:
                return nil
            }
        }
    }

    /// An enum that is used for the directions that a split focus event can change.
    enum SplitFocusDirection {
        case previous, next, up, down, left, right

        /// Initialize from a TermSurf API enum.
        static func from(direction: termsurf_action_goto_split_e) -> Self? {
            switch (direction) {
            case TERMSURF_GOTO_SPLIT_PREVIOUS:
                return .previous

            case TERMSURF_GOTO_SPLIT_NEXT:
                return .next

            case TERMSURF_GOTO_SPLIT_UP:
                return .up

            case TERMSURF_GOTO_SPLIT_DOWN:
                return .down

            case TERMSURF_GOTO_SPLIT_LEFT:
                return .left

            case TERMSURF_GOTO_SPLIT_RIGHT:
                return .right

            default:
                return nil
            }
        }

        func toNative() -> termsurf_action_goto_split_e {
            switch (self) {
            case .previous:
                return TERMSURF_GOTO_SPLIT_PREVIOUS

            case .next:
                return TERMSURF_GOTO_SPLIT_NEXT

            case .up:
                return TERMSURF_GOTO_SPLIT_UP

            case .down:
                return TERMSURF_GOTO_SPLIT_DOWN

            case .left:
                return TERMSURF_GOTO_SPLIT_LEFT

            case .right:
                return TERMSURF_GOTO_SPLIT_RIGHT
            }
        }
    }

    /// Enum used for resizing splits. This is the direction the split divider will move.
    enum SplitResizeDirection {
        case up, down, left, right

        static func from(direction: termsurf_action_resize_split_direction_e) -> Self? {
            switch (direction) {
            case TERMSURF_RESIZE_SPLIT_UP:
                return .up;
            case TERMSURF_RESIZE_SPLIT_DOWN:
                return .down;
            case TERMSURF_RESIZE_SPLIT_LEFT:
                return .left;
            case TERMSURF_RESIZE_SPLIT_RIGHT:
                return .right;
            default:
                return nil
            }
        }

        func toNative() -> termsurf_action_resize_split_direction_e {
            switch (self) {
            case .up:
                return TERMSURF_RESIZE_SPLIT_UP;
            case .down:
                return TERMSURF_RESIZE_SPLIT_DOWN;
            case .left:
                return TERMSURF_RESIZE_SPLIT_LEFT;
            case .right:
                return TERMSURF_RESIZE_SPLIT_RIGHT;
            }
        }
    }
}

#if canImport(AppKit)
// MARK: SplitFocusDirection Extensions

extension TermSurf.SplitFocusDirection {
    /// Convert to a SplitTree.FocusDirection for the given ViewType.
    func toSplitTreeFocusDirection<ViewType>() -> SplitTree<ViewType>.FocusDirection {
        switch self {
        case .previous:
            return .previous

        case .next:
            return .next

        case .up:
            return .spatial(.up)

        case .down:
            return .spatial(.down)

        case .left:
            return .spatial(.left)

        case .right:
            return .spatial(.right)
        }
    }
}
#endif

extension TermSurf {
    /// The type of a clipboard request
    enum ClipboardRequest {
        /// A direct paste of clipboard contents
        case paste

        /// An application is attempting to read from the clipboard using OSC 52
        case osc_52_read

        /// An application is attempting to write to the clipboard using OSC 52
        case osc_52_write(OSPasteboard?)

        /// The text to show in the clipboard confirmation prompt for a given request type
        func text() -> String {
            switch (self) {
            case .paste:
                return """
                Pasting this text to the terminal may be dangerous as it looks like some commands may be executed.
                """
            case .osc_52_read:
                return """
                An application is attempting to read from the clipboard.
                The current clipboard contents are shown below.
                """
            case .osc_52_write:
                return """
                An application is attempting to write to the clipboard.
                The content to write is shown below.
                """
            }
        }

        static func from(request: termsurf_clipboard_request_e) -> ClipboardRequest? {
            switch (request) {
            case TERMSURF_CLIPBOARD_REQUEST_PASTE:
                return .paste
            case TERMSURF_CLIPBOARD_REQUEST_OSC_52_READ:
                return .osc_52_read
            case TERMSURF_CLIPBOARD_REQUEST_OSC_52_WRITE:
                return .osc_52_write(nil)
            default:
                return nil
            }
        }
    }
    
    struct ClipboardContent {
        let mime: String
        let data: String
        
        static func from(content: termsurf_clipboard_content_s) -> ClipboardContent? {
            guard let mimePtr = content.mime,
                  let dataPtr = content.data else {
                return nil
            }
            
            return ClipboardContent(
                mime: String(cString: mimePtr),
                data: String(cString: dataPtr)
            )
        }
    }

    /// macos-icon
    enum MacOSIcon: String, Sendable {
        case official
        case blueprint
        case chalkboard
        case glass
        case holographic
        case microchip
        case paper
        case retro
        case xray
        case custom
        case customStyle = "custom-style"

        /// Bundled asset name for built-in icons
        var assetName: String? {
            switch self {
            case .official: return nil
            case .blueprint: return "BlueprintImage"
            case .chalkboard: return "ChalkboardImage"
            case .microchip: return "MicrochipImage"
            case .glass: return "GlassImage"
            case .holographic: return "HolographicImage"
            case .paper: return "PaperImage"
            case .retro: return "RetroImage"
            case .xray: return "XrayImage"
            case .custom, .customStyle: return nil
            }
        }
    }

    /// macos-icon-frame
    enum MacOSIconFrame: String {
        case aluminum
        case beige
        case plastic
        case chrome
    }

    /// Enum for the macos-window-buttons config option
    enum MacOSWindowButtons: String {
        case visible
        case hidden
    }

    /// Enum for the macos-titlebar-proxy-icon config option
    enum MacOSTitlebarProxyIcon: String {
        case visible
        case hidden
    }

    /// Enum for auto-update-channel config option
    enum AutoUpdateChannel: String {
        case tip
        case stable
    }
}

// MARK: Surface Notification

extension Notification.Name {
    /// Configuration change. If the object is nil then it is app-wide. Otherwise its surface-specific.
    static let termsurfConfigDidChange = Notification.Name("com.termsurf.configDidChange")
    static let TermSurfConfigChangeKey = termsurfConfigDidChange.rawValue

    /// Color change. Object is the surface changing.
    static let termsurfColorDidChange = Notification.Name("com.termsurf.termsurfColorDidChange")
    static let TermSurfColorChangeKey = termsurfColorDidChange.rawValue

    /// Goto tab. Has tab index in the userinfo.
    static let termsurfMoveTab = Notification.Name("com.termsurf.moveTab")
    static let TermSurfMoveTabKey = termsurfMoveTab.rawValue

    /// Close tab
    static let termsurfCloseTab = Notification.Name("com.termsurf.closeTab")

    /// Close other tabs
    static let termsurfCloseOtherTabs = Notification.Name("com.termsurf.closeOtherTabs")

    /// Close tabs to the right of the focused tab
    static let termsurfCloseTabsOnTheRight = Notification.Name("com.termsurf.closeTabsOnTheRight")

    /// Close window
    static let termsurfCloseWindow = Notification.Name("com.termsurf.closeWindow")

    /// Resize the window to a default size.
    static let termsurfResetWindowSize = Notification.Name("com.termsurf.resetWindowSize")

    /// Ring the bell
    static let termsurfBellDidRing = Notification.Name("com.termsurf.termsurfBellDidRing")

    /// Readonly mode changed
    static let termsurfDidChangeReadonly = Notification.Name("com.termsurf.didChangeReadonly")
    static let ReadonlyKey = termsurfDidChangeReadonly.rawValue + ".readonly"
    static let termsurfCommandPaletteDidToggle = Notification.Name("com.termsurf.commandPaletteDidToggle")

    /// Toggle maximize of current window
    static let termsurfMaximizeDidToggle = Notification.Name("com.termsurf.maximizeDidToggle")

    /// Notification sent when scrollbar updates
    static let termsurfDidUpdateScrollbar = Notification.Name("com.termsurf.didUpdateScrollbar")
    static let ScrollbarKey = termsurfDidUpdateScrollbar.rawValue + ".scrollbar"

    /// Focus the search field
    static let termsurfSearchFocus = Notification.Name("com.termsurf.searchFocus")
}

// NOTE: I am moving all of these to Notification.Name extensions over time. This
// namespace was the old namespace.
extension TermSurf.Notification {
    /// Used to pass a configuration along when creating a new tab/window/split.
    static let NewSurfaceConfigKey = "com.termsurf.newSurfaceConfig"

    /// Posted when a new split is requested. The sending object will be the surface that had focus. The
    /// userdata has one key "direction" with the direction to split to.
    static let termsurfNewSplit = Notification.Name("com.termsurf.newSplit")

    /// Close the calling surface.
    static let termsurfCloseSurface = Notification.Name("com.termsurf.closeSurface")

    /// Focus previous/next split. Has a SplitFocusDirection in the userinfo.
    static let termsurfFocusSplit = Notification.Name("com.termsurf.focusSplit")
    static let SplitDirectionKey = termsurfFocusSplit.rawValue

    /// Goto tab. Has tab index in the userinfo.
    static let termsurfGotoTab = Notification.Name("com.termsurf.gotoTab")
    static let GotoTabKey = termsurfGotoTab.rawValue

    /// New tab. Has base surface config requested in userinfo.
    static let termsurfNewTab = Notification.Name("com.termsurf.newTab")

    /// New window. Has base surface config requested in userinfo.
    static let termsurfNewWindow = Notification.Name("com.termsurf.newWindow")

    /// Present terminal. Bring the surface's window to focus without activating the app.
    static let termsurfPresentTerminal = Notification.Name("com.termsurf.presentTerminal")

    /// Toggle fullscreen of current window
    static let termsurfToggleFullscreen = Notification.Name("com.termsurf.toggleFullscreen")
    static let FullscreenModeKey = termsurfToggleFullscreen.rawValue

    /// Notification sent to toggle split maximize/unmaximize.
    static let didToggleSplitZoom = Notification.Name("com.termsurf.didToggleSplitZoom")

    /// Notification
    static let didReceiveInitialWindowFrame = Notification.Name("com.termsurf.didReceiveInitialWindowFrame")
    static let FrameKey = "com.termsurf.frame"

    /// Notification to render the inspector for a surface
    static let inspectorNeedsDisplay = Notification.Name("com.termsurf.inspectorNeedsDisplay")

    /// Notification to show/hide the inspector
    static let didControlInspector = Notification.Name("com.termsurf.didControlInspector")

    static let confirmClipboard = Notification.Name("com.termsurf.confirmClipboard")
    static let ConfirmClipboardStrKey = confirmClipboard.rawValue + ".str"
    static let ConfirmClipboardStateKey = confirmClipboard.rawValue + ".state"
    static let ConfirmClipboardRequestKey = confirmClipboard.rawValue + ".request"

    /// Notification sent to the active split view to resize the split.
    static let didResizeSplit = Notification.Name("com.termsurf.didResizeSplit")
    static let ResizeSplitDirectionKey = didResizeSplit.rawValue + ".direction"
    static let ResizeSplitAmountKey = didResizeSplit.rawValue + ".amount"

    /// Notification sent to the split root to equalize split sizes
    static let didEqualizeSplits = Notification.Name("com.termsurf.didEqualizeSplits")

    /// Notification that renderer health changed
    static let didUpdateRendererHealth = Notification.Name("com.termsurf.didUpdateRendererHealth")

    /// Notifications related to key sequences
    static let didContinueKeySequence = Notification.Name("com.termsurf.didContinueKeySequence")
    static let didEndKeySequence = Notification.Name("com.termsurf.didEndKeySequence")
    static let KeySequenceKey = didContinueKeySequence.rawValue + ".key"

    /// Notifications related to key tables
    static let didChangeKeyTable = Notification.Name("com.termsurf.didChangeKeyTable")
    static let KeyTableKey = didChangeKeyTable.rawValue + ".action"
}

// Make the input enum hashable.
extension termsurf_input_key_e : @retroactive Hashable {}
