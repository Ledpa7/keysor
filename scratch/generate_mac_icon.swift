import AppKit

let projectDir = "/Volumes/scratch/14-Keysor"
let pngPath = "\(projectDir)/public/logo.png"
let svgPath = "\(projectDir)/public/logo.svg"
let iconsetDir = "\(projectDir)/keysor.iconset"
let icnsPath = "\(projectDir)/keysor.icns"

let fileManager = FileManager.default

try? fileManager.removeItem(atPath: iconsetDir)
try? fileManager.createDirectory(atPath: iconsetDir, withIntermediateDirectories: true)

// Load existing logo image (try PNG first, fall back to SVG)
var logoImage = NSImage(contentsOfFile: pngPath)
if logoImage == nil {
    logoImage = NSImage(contentsOfFile: svgPath)
}

guard let image = logoImage else {
    print("Error: Could not load logo image")
    exit(1)
}

print("Loaded logo image with size: \(image.size)")

func generateIcon(totalSize: CGFloat, tileSize: CGFloat, filename: String) {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: Int(totalSize),
        pixelsHigh: Int(totalSize),
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )!

    NSGraphicsContext.saveGraphicsState()
    let context = NSGraphicsContext(bitmapImageRep: rep)
    NSGraphicsContext.current = context

    // Clear context with transparency
    NSColor.clear.set()
    NSRect(x: 0, y: 0, width: totalSize, height: totalSize).fill()

    let margin = (totalSize - tileSize) / 2.0
    let tileRect = NSRect(x: margin, y: margin, width: tileSize, height: tileSize)

    // Standard macOS squircle corner radius (22.37% of tile width)
    let cornerRadius = tileSize * 0.2237
    let clipPath = NSBezierPath(roundedRect: tileRect, xRadius: cornerRadius, yRadius: cornerRadius)

    // Draw subtle drop shadow for macOS tile depth
    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.35)
    shadow.shadowOffset = NSSize(width: 0, height: -tileSize * 0.025)
    shadow.shadowBlurRadius = tileSize * 0.05
    shadow.set()

    // Background tile carbon fill
    let bgPath = NSBezierPath(roundedRect: tileRect, xRadius: cornerRadius, yRadius: cornerRadius)
    NSColor(red: 0.07, green: 0.07, blue: 0.06, alpha: 1.0).setFill()
    bgPath.fill()

    // Reset shadow before drawing image content
    NSShadow().set()

    // Clip to squircle path
    clipPath.addClip()

    // Draw source logo image with explicit source rect!
    let srcRect = NSRect(origin: .zero, size: image.size)
    image.draw(in: tileRect, from: srcRect, operation: .sourceOver, fraction: 1.0)

    NSGraphicsContext.restoreGraphicsState()

    if let pngData = rep.representation(using: .png, properties: [:]) {
        let savePath = "\(iconsetDir)/\(filename)"
        try? pngData.write(to: URL(fileURLWithPath: savePath))
        print("Generated \(filename) (\(Int(totalSize))x\(Int(totalSize)))")
    }
}

let specs: [(CGFloat, CGFloat, String)] = [
    (16, 16, "icon_16x16.png"),
    (32, 32, "icon_16x16@2x.png"),
    (32, 32, "icon_32x32.png"),
    (64, 64, "icon_32x32@2x.png"),
    (128, 128, "icon_128x128.png"),
    (256, 256, "icon_128x128@2x.png"),
    (256, 256, "icon_256x256.png"),
    (512, 512, "icon_256x256@2x.png"),
    (512, 512, "icon_512x512.png"),
    (1024, 1024, "icon_512x512@2x.png")
]

for spec in specs {
    let tileRatio: CGFloat = 0.82
    let tileSize = spec.0 * tileRatio
    generateIcon(totalSize: spec.0, tileSize: tileSize, filename: spec.2)
}

print("Running iconutil to generate keysor.icns...")
let process = Process()
process.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
process.arguments = ["-c", "icns", iconsetDir, "-o", icnsPath]
try? process.run()
process.waitUntilExit()

if fileManager.fileExists(atPath: icnsPath) {
    print("SUCCESS! Created macOS app icon: \(icnsPath)")
} else {
    print("Error: iconutil failed")
}
