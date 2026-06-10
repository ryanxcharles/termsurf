// Shift + drag from (x0,y0) to (x1,y1): swift shiftdrag.swift <x0> <y0> <x1> <y1> (Issue 802 / Exp 33 live).
import CoreGraphics
import Foundation
let a = CommandLine.arguments
guard a.count >= 5, let x0 = Double(a[1]), let y0 = Double(a[2]), let x1 = Double(a[3]), let y1 = Double(a[4]) else {
    print("usage: shiftdrag.swift <x0> <y0> <x1> <y1>"); exit(1)
}
let prior = CGEvent(source: nil)?.location ?? CGPoint(x: x0, y: y0)
func post(_ t: CGEventType, _ p: CGPoint) {
    if let e = CGEvent(mouseEventSource: nil, mouseType: t, mouseCursorPosition: p, mouseButton: .left) {
        e.flags = .maskShift; e.post(tap: .cghidEventTap)
    }
}
let a0 = CGPoint(x: x0, y: y0), a1 = CGPoint(x: x1, y: y1)
CGWarpMouseCursorPosition(a0); usleep(120_000)
post(.leftMouseDown, a0); usleep(60_000)
for i in 1...8 {
    let t = Double(i) / 8.0
    post(.leftMouseDragged, CGPoint(x: x0 + (x1 - x0) * t, y: y0 + (y1 - y0) * t)); usleep(30_000)
}
post(.leftMouseUp, a1); usleep(80_000); CGWarpMouseCursorPosition(prior)
print("shift-dragged (\(Int(x0)),\(Int(y0)))->(\(Int(x1)),\(Int(y1)))")
