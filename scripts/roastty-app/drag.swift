// Synthesize a left-button mouse drag: swift drag.swift <x0> <y0> <x1> <y1> [steps]
// Screen coords. leftMouseDown at (x0,y0) → leftMouseDragged steps → leftMouseUp at (x1,y1),
// at .cghidEventTap (routes to the window under the cursor). Restores the prior cursor.
// (Issue 802 / Exp 25.)
import CoreGraphics
import Foundation
let a = CommandLine.arguments
guard a.count >= 5, let x0 = Double(a[1]), let y0 = Double(a[2]),
      let x1 = Double(a[3]), let y1 = Double(a[4]) else {
    print("usage: drag.swift <x0> <y0> <x1> <y1> [steps]"); exit(1)
}
let steps = a.count >= 6 ? (Int(a[5]) ?? 10) : 10
let prior = CGEvent(source: nil)?.location ?? CGPoint(x: x0, y: y0)
func post(_ type: CGEventType, _ p: CGPoint) {
    if let e = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: p, mouseButton: .left) {
        e.post(tap: .cghidEventTap)
    }
}
CGWarpMouseCursorPosition(CGPoint(x: x0, y: y0)); usleep(120_000)
post(.leftMouseDown, CGPoint(x: x0, y: y0)); usleep(40_000)
for i in 1...steps {
    let f = Double(i) / Double(steps)
    let p = CGPoint(x: x0 + (x1 - x0) * f, y: y0 + (y1 - y0) * f)
    post(.leftMouseDragged, p); usleep(25_000)
}
post(.leftMouseUp, CGPoint(x: x1, y: y1)); usleep(80_000)
CGWarpMouseCursorPosition(prior)
print("dragged (\(Int(x0)),\(Int(y0))) -> (\(Int(x1)),\(Int(y1)))")
