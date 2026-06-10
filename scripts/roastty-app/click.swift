// N rapid left clicks at a point: swift click.swift <x> <y> <count>
// Sets mouseEventClickState 1..N for proper double/triple-click semantics. (Issue 802 / Exp 27.)
import CoreGraphics
import Foundation
let a = CommandLine.arguments
guard a.count >= 4, let x = Double(a[1]), let y = Double(a[2]), let n = Int(a[3]) else {
    print("usage: click.swift <x> <y> <count>"); exit(1)
}
let p = CGPoint(x: x, y: y)
let prior = CGEvent(source: nil)?.location ?? p
CGWarpMouseCursorPosition(p); usleep(120_000)
func post(_ t: CGEventType, _ click: Int64) {
    if let e = CGEvent(mouseEventSource: nil, mouseType: t, mouseCursorPosition: p, mouseButton: .left) {
        e.setIntegerValueField(.mouseEventClickState, value: click)
        e.post(tap: .cghidEventTap)
    }
}
for i in 1...n {
    post(.leftMouseDown, Int64(i)); usleep(25_000)
    post(.leftMouseUp, Int64(i)); usleep(35_000)
}
usleep(120_000); CGWarpMouseCursorPosition(prior)
print("clicked \(n)x at (\(Int(x)),\(Int(y)))")
