```swift title="Swift"
import Xberg

final class PdfOnlyProcessor: PostProcessor {
    func name() -> String {
        "pdf-only-processor"
    }

    func version() -> String {
        "1.0.0"
    }

    func processingStage() -> String {
        "middle"  // ProcessingStage enum name
    }

    func priority() -> Int32 {
        50  // Default priority
    }

    func process(result: ExtractedDocument, config: ExtractionConfig) -> String {
        // Returns JSON-encoded Result<(), String>
        // No-op post-processor for PDF-only processing
        "{\"ok\": null}"
    }

    func shouldProcess(result: ExtractedDocument, config: ExtractionConfig) -> Bool {
        result.mimeType == "application/pdf"
    }

    func estimatedDurationMs(result: ExtractedDocument) -> UInt64 {
        0  // No processing overhead
    }

    func initialize() -> String {
        "{\"ok\": null}"
    }

    func shutdown() -> String {
        "{\"ok\": null}"
    }
}

let processor = PdfOnlyProcessor()
try Xberg.registerPostProcessor(processor)
```
