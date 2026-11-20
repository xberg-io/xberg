// Generates the synthetic tinted table rasters used by the #3337 low-ppi resample fixtures.
//
//   xcrun swift generate.swift 100 9 tinted tinted-100-9px.png
//   xcrun swift generate.swift 100 7 tinted tinted-100-7px.png
//   xcrun swift generate.swift 300 27 tinted tinted-300-9px.png
//
// Byte-identical output was verified for all three committed PNGs. Arial is only needed at
// generation time; the committed PNG is the fixture.
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
guard args.count == 5, let dpi = Int(args[1]), let digitPx = Int(args[2]) else {
    FileHandle.standardError.write("usage: gen_tinted_table.swift <dpi> <digit-px> <tinted|plain> <out.png>\n".data(using: .utf8)!)
    exit(2)
}
let tinted = args[3] == "tinted"
let outPath = args[4]

// A4 page.
let widthPx = Int((210.0 / 25.4 * Double(dpi)).rounded())
let heightPx = Int((297.0 / 25.4 * Double(dpi)).rounded())
let scale = Double(dpi) / 100.0

let colorSpace = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CGContext(
    data: nil,
    width: widthPx,
    height: heightPx,
    bitsPerComponent: 8,
    bytesPerRow: 0,
    space: colorSpace,
    bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue
)!

func colour(_ r: Double, _ g: Double, _ b: Double) -> CGColor {
    CGColor(colorSpace: colorSpace, components: [r / 255.0, g / 255.0, b / 255.0, 1.0])!
}

let paper = tinted ? colour(214, 224, 238) : colour(255, 255, 255)
let ink = tinted ? colour(40, 52, 70) : colour(0, 0, 0)
context.setFillColor(paper)
context.fill(CGRect(x: 0, y: 0, width: widthPx, height: heightPx))
context.setShouldAntialias(true)
context.setAllowsAntialiasing(true)
context.setShouldSmoothFonts(true)

// Digit height of Arial caps is ~0.716 em; ask for the requested pixel height directly.
let emPx = Double(digitPx) / 0.716
let fontPt = emPx
let font = CTFontCreateWithName("Arial" as CFString, fontPt, nil)
let attributes: [NSAttributedString.Key: Any] = [
    NSAttributedString.Key(kCTFontAttributeName as String): font,
    NSAttributedString.Key(kCTForegroundColorAttributeName as String): ink,
]

let leftMargin = emPx * 2.0
let topMargin = emPx * 3.0
let columnPitch = emPx * 3.1
let rowPitch = emPx * 1.5

for row in 0..<12 {
    for column in 0..<6 {
        let value = 101 + row * 6 + column
        let line = CTLineCreateWithAttributedString(
            NSAttributedString(string: "\(value)", attributes: attributes)
        )
        let x = leftMargin + Double(column) * columnPitch
        let y = Double(heightPx) - topMargin - Double(row) * rowPitch - emPx
        context.textPosition = CGPoint(x: x, y: y)
        CTLineDraw(line, context)
    }
}

let image = context.makeImage()!
let destination = CGImageDestinationCreateWithURL(
    URL(fileURLWithPath: outPath) as CFURL, UTType.png.identifier as CFString, 1, nil
)!
CGImageDestinationAddImage(destination, image, [
    kCGImagePropertyDPIWidth: dpi,
    kCGImagePropertyDPIHeight: dpi,
] as CFDictionary)
guard CGImageDestinationFinalize(destination) else {
    FileHandle.standardError.write("png write failed\n".data(using: .utf8)!)
    exit(4)
}
print("wrote \(outPath) \(widthPx)x\(heightPx) dpi=\(dpi) digit=\(digitPx)px tinted=\(tinted)")
