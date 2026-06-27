```swift title="Swift"
import Foundation
import Xberg
import RustBridge

// Build a fully-featured `ExtractionConfig` via JSON. The opaque swift-bridge
// initializer takes 30+ positional parameters, so JSON is the ergonomic path
// for non-trivial configs.
let configJson = """
{
    "use_cache": true,
    "enable_quality_processing": true,
    "ocr": {
        "backend": "tesseract",
        "language": "eng"
    },
    "force_ocr": false,
    "chunking": {
        "max_characters": 800,
        "overlap": 100,
        "chunker_type": "markdown",
        "prepend_heading_context": true
    },
    "images": {
        "extract_images": true
    },
    "output_format": "markdown",
    "include_document_structure": true
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"report.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

let content = result.content.toString()
print("Content (\(content.count) chars):")
let preview = String(content.prefix(200))
print(preview)

if let chunks = result.chunks {
    print("\nChunks: \(chunks.count)")
}
print("Tables: \(result.tables.count)")

if let languages = result.detectedLanguages {
    let langs = languages.map { $0.toString() }
    print("Languages: \(langs)")
}

if let method = result.extractionMethod {
    print("Extraction method: \(method)")
}
```
