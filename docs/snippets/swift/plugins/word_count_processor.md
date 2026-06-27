```swift title="Swift"
import Xberg

final class WordCountProcessor: PostProcessor {
    func name() -> String {
        "word_count"
    }

    func version() -> String {
        "1.0.0"
    }

    func processingStage() -> String {
        "early"
    }

    func priority() -> Int32 {
        50
    }

    func process(result: ExtractedDocument, config: ExtractionConfig) -> String {
        let content = result.content
        let words = content.split(separator: " ").count

        // Metadata is not directly mutable via the FFI, so store in logs or use
        // a side-channel approach. For now, just track that processing happened.
        return "{\"ok\": null}"
    }

    func shouldProcess(result: ExtractedDocument, config: ExtractionConfig) -> Bool {
        !result.content.isEmpty
    }

    func estimatedDurationMs(result: ExtractedDocument) -> UInt64 {
        5
    }

    func initialize() -> String {
        "{\"ok\": null}"
    }

    func shutdown() -> String {
        "{\"ok\": null}"
    }
}

let processor = WordCountProcessor()
try Xberg.registerPostProcessor(processor)
```
