```swift title="Swift"
import Foundation
import Xberg
import RustBridge

// Build a fully-featured `ExtractionConfig` via JSON. ExtractionConfig has
// 30+ fields, so JSON is the ergonomic path for non-trivial configs.
let configJson = """
{
    "use_cache": true,
    "enable_quality_processing": true,
    "ocr": {
        "backend": "tesseract",
        "language": "eng"
    },
    "chunking": {
        "max_characters": 1000,
        "overlap": 200,
        "embedding": {
            "model": {"preset": {"name": "balanced"}},
            "batch_size": 32,
            "normalize": true,
            "show_download_progress": false
        }
    },
    "language_detection": {
        "enabled": true,
        "min_confidence": 0.8,
        "detect_multiple": false
    },
    "keywords": {
        "algorithm": "yake",
        "max_keywords": 10,
        "min_score": 0.1,
        "ngram_range": [1, 3],
        "language": "en"
    },
    "token_reduction": {
        "mode": "moderate",
        "preserve_important_words": true
    },
    "postprocessor": {
        "enabled": true
    }
}
"""

let config = try extractionConfigFromJson(configJson)
let input = try extractInputFromJson(#"{"kind":"uri","uri":"document.pdf"}"#)
let resultOutput = try await extract(input: input, config: config)
let result = resultOutput.results().get(index: 0)!

print("Content: \(result.content.toString())")
if let languages = result.detectedLanguages {
    let langs = languages.map { $0.toString() }
    print("Languages: \(langs)")
}
if let chunks = result.chunks {
    print("Chunks: \(chunks.count)")
}
```
