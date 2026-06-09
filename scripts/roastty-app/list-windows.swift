// List on-screen windows for a PID: swift list-windows.swift <pid>
// (winid.swift can resolve the wrong window; the real Roastty window is name="👻")
import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
for w in list {
  guard let owner = w[kCGWindowOwnerPID as String] as? Int32, owner == pid else { continue }
  let id = w[kCGWindowNumber as String] as? Int ?? -1
  let name = w[kCGWindowName as String] as? String ?? ""
  let layer = w[kCGWindowLayer as String] as? Int ?? -1
  if let b = w[kCGWindowBounds as String] as? [String: CGFloat] {
    print("id=\(id) layer=\(layer) bounds=(\(Int(b["X"]!)),\(Int(b["Y"]!)) \(Int(b["Width"]!))x\(Int(b["Height"]!))) name=\"\(name)\"")
  }
}
