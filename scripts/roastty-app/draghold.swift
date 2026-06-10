// Drag from (x0,y0) to (x1,y1), HOLD the button down at the end for <holdMs>, then release.
// swift draghold.swift <x0> <y0> <x1> <y1> <holdMs>. For autoscroll: drag to the edge + hold so
// the present loop ticks. (Issue 802 / Exp 28.) Restores the prior cursor.
import CoreGraphics
import Foundation
let a = CommandLine.arguments
guard a.count >= 6, let x0 = Double(a[1]), let y0 = Double(a[2]),
      let x1 = Double(a[3]), let y1 = Double(a[4]), let holdMs = Int(a[5]) else {
    print("usage: draghold.swift <x0> <y0> <x1> <y1> <holdMs>"); exit(1)
}
let prior = CGEvent(source: nil)?.location ?? CGPoint(x: x0, y: y0)
func post(_ t: CGEventType, _ p: CGPoint) {
    if let e = CGEvent(mouseEventSource: nil, mouseType: t, mouseCursorPosition: p, mouseButton: .left) {
        e.post(tap: .cghidEventTap)
    }
}
CGWarpMouseCursorPosition(CGPoint(x: x0, y: y0)); usleep(120_000)
post(.leftMouseDown, CGPoint(x: x0, y: y0)); usleep(40_000)
let steps = 10
for i in 1...steps {
    let f = Double(i) / Double(steps)
    post(.leftMouseDragged, CGPoint(x: x0 + (x1 - x0) * f, y: y0 + (y1 - y0) * f)); usleep(25_000)
}
// hold at the edge so the present loop autoscrolls
usleep(UInt32(holdMs) * 1000)
post(.leftMouseUp, CGPoint(x: x1, y: y1)); usleep(80_000)
CGWarpMouseCursorPosition(prior)
print("drag+hold \(holdMs)ms (\(Int(x0)),\(Int(y0)))->(\(Int(x1)),\(Int(y1)))")
