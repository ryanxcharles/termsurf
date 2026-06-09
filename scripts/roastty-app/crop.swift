// Crop a PNG to a pixel rect: swift crop.swift <in.png> <out.png> <x> <y> <w> <h>
import AppKit
let a = CommandLine.arguments
guard a.count == 7, let x = Int(a[3]), let y = Int(a[4]), let w = Int(a[5]), let h = Int(a[6]),
      let img = NSImage(contentsOfFile: a[1]),
      let tiff = img.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
      let cg = rep.cgImage else { print("load/arg fail (count=\(a.count))"); exit(1) }
guard let sub = cg.cropping(to: CGRect(x: x, y: y, width: w, height: h)) else { print("crop fail"); exit(1) }
let out = NSBitmapImageRep(cgImage: sub)
guard let data = out.representation(using: .png, properties: [:]) else { print("enc fail"); exit(1) }
try! data.write(to: URL(fileURLWithPath: a[2]))
print("cropped \(w)x\(h)@(\(x),\(y)) -> \(a[2])")
