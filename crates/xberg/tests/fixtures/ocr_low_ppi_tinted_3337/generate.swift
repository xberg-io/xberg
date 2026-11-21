// Generates the synthetic tinted table rasters used by the #3337 low-ppi fixtures.
//
//   xcrun swift generate.swift 100 9 tinted tinted-100-9px.png
//   xcrun swift generate.swift 100 7 tinted tinted-100-7px.png
//   xcrun swift generate.swift 300 27 tinted tinted-300-9px.png
//
// A fifth argument wraps the PNG it just wrote into a one-page PDF scan whose page geometry
// matches the raster's density, so the scan route (PDF raster -> OCR) sees the same pixels:
//
//   xcrun swift generate.swift 100 9 tinted tinted-100-9px.png tinted-100-9px.pdf
//   xcrun swift generate.swift 100 7 tinted tinted-100-7px.png tinted-100-7px.pdf
//
// Byte-identical output was verified for all three committed PNGs. The PDFs carry no creation
// date or producer string, so a regenerated PDF differs only in the trailer /ID. Arial is only
// needed at generation time; the committed rasters are the fixtures.
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
guard (args.count == 5 || args.count == 6), let dpi = Int(args[1]), let digitPx = Int(args[2]) else {
    FileHandle.standardError.write("usage: gen_tinted_table.swift <dpi> <digit-px> <tinted|plain> <out.png> [out.pdf]\n".data(using: .utf8)!)
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
let pngURL = URL(fileURLWithPath: outPath) as CFURL
let destination = CGImageDestinationCreateWithURL(pngURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(destination, image, [
    kCGImagePropertyDPIWidth: dpi,
    kCGImagePropertyDPIHeight: dpi,
] as CFDictionary)
guard CGImageDestinationFinalize(destination) else {
    FileHandle.standardError.write("png write failed\n".data(using: .utf8)!)
    exit(4)
}
print("wrote \(outPath) \(widthPx)x\(heightPx) dpi=\(dpi) digit=\(digitPx)px tinted=\(tinted)")

if args.count == 6 {
    // Wrap the just-written PNG, not a re-render, so the scan route receives the fixture raster
    // pixel for pixel. The page is the raster's own size in points at its own density.
    let pdfPath = args[5]
    let source = CGImageSourceCreateWithURL(pngURL, nil)!
    let raster = CGImageSourceCreateImageAtIndex(source, 0, nil)!
    var mediaBox = CGRect(
        x: 0,
        y: 0,
        width: Double(widthPx) * 72.0 / Double(dpi),
        height: Double(heightPx) * 72.0 / Double(dpi)
    )
    let pdfContext = CGContext(
        consumer: CGDataConsumer(url: URL(fileURLWithPath: pdfPath) as CFURL)!,
        mediaBox: &mediaBox,
        [kCGPDFContextCreator: "generate.swift"] as CFDictionary
    )!
    pdfContext.beginPDFPage(nil)
    pdfContext.draw(raster, in: mediaBox)
    pdfContext.endPDFPage()
    pdfContext.closePDF()
    print("wrote \(pdfPath) \(widthPx)x\(heightPx) dpi=\(dpi) scan page")
}
