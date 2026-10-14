// Draws the FretStudy app icon (1024×1024 PNG): a chord box on a warm tile, with dots in
// the app's finger colors. Run: swift scripts/make-icon.swift assets/icon-1024.png
import AppKit

let size = 1024.0
let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon-1024.png"
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size),
                           bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                           colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let ctx = NSGraphicsContext.current!.cgContext

func rgb(_ r: Int, _ g: Int, _ b: Int) -> CGColor {
    CGColor(red: CGFloat(r) / 255, green: CGFloat(g) / 255, blue: CGFloat(b) / 255, alpha: 1)
}

// Tile (macOS icon grid: 824 px rounded square centered in 1024).
let tile = CGRect(x: 100, y: 100, width: 824, height: 824)
let tilePath = CGPath(roundedRect: tile, cornerWidth: 185, cornerHeight: 185, transform: nil)
ctx.addPath(tilePath)
ctx.clip()
let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                          colors: [rgb(0xF2, 0x8C, 0x28), rgb(0xD5, 0x5E, 0x00)] as CFArray,
                          locations: [0, 1])!
ctx.drawLinearGradient(gradient, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 100), options: [])
ctx.resetClip()

// Fretboard panel.
let board = CGRect(x: 262, y: 200, width: 500, height: 560)
ctx.setFillColor(rgb(0xFB, 0xF3, 0xE4))
ctx.addPath(CGPath(roundedRect: board, cornerWidth: 36, cornerHeight: 36, transform: nil))
ctx.fillPath()

// Nut, frets, and strings.
let left = 312.0, right = 712.0, top = 700.0, bottom = 250.0
ctx.setFillColor(rgb(0x3A, 0x34, 0x2E))
ctx.fill(CGRect(x: left - 6, y: top, width: right - left + 12, height: 26))
ctx.setStrokeColor(rgb(0x9A, 0x96, 0x90))
ctx.setLineWidth(8)
for i in 1...4 {
    let y = top - Double(i) * (top - bottom) / 4
    ctx.move(to: CGPoint(x: left, y: y)); ctx.addLine(to: CGPoint(x: right, y: y))
}
ctx.strokePath()
ctx.setStrokeColor(rgb(0x5C, 0x55, 0x4C))
for s in 0..<6 {
    let x = left + Double(s) * (right - left) / 5
    ctx.setLineWidth(CGFloat(14 - s * 2))
    ctx.move(to: CGPoint(x: x, y: top)); ctx.addLine(to: CGPoint(x: x, y: bottom))
    ctx.strokePath()
}

// A C chord in finger colors: 3 (orange) on string 5, 2 (green) on string 4, 1 (blue) on string 2.
func dot(_ string: Int, _ fret: Int, _ color: CGColor) {
    let x = left + Double(6 - string) * (right - left) / 5
    let y = top - (Double(fret) - 0.5) * (top - bottom) / 4
    ctx.setFillColor(color)
    ctx.fillEllipse(in: CGRect(x: x - 46, y: y - 46, width: 92, height: 92))
    ctx.setStrokeColor(rgb(0x2A, 0x26, 0x22))
    ctx.setLineWidth(6)
    ctx.strokeEllipse(in: CGRect(x: x - 46, y: y - 46, width: 92, height: 92))
}
dot(5, 3, rgb(0xE6, 0x9F, 0x00))
dot(4, 2, rgb(0x00, 0x9E, 0x73))
dot(2, 1, rgb(0x00, 0x72, 0xB2))

NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
print("Wrote \(out)")
