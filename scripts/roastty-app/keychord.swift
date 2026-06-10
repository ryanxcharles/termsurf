// Post a ⌘-modified keystroke via CGEvent: swift keychord.swift <keycode>
// e.g. 8 = 'c'. Goes to the frontmost/key app at .cghidEventTap. (Issue 802 / Exp 26.)
import CoreGraphics
import Foundation
guard CommandLine.arguments.count >= 2, let kc = Int(CommandLine.arguments[1]) else {
    print("usage: keychord.swift <keycode>"); exit(1)
}
let src = CGEventSource(stateID: .combinedSessionState)
let down = CGEvent(keyboardEventSource: src, virtualKey: CGKeyCode(kc), keyDown: true)
down?.flags = .maskCommand
let up = CGEvent(keyboardEventSource: src, virtualKey: CGKeyCode(kc), keyDown: false)
up?.flags = .maskCommand
down?.post(tap: .cghidEventTap)
usleep(40_000)
up?.post(tap: .cghidEventTap)
print("posted cmd+keycode \(kc)")
