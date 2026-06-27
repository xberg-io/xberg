```swift title="Swift"
import Xberg
import os.log

let logger = Logger(subsystem: "com.example.plugins", category: "MyPlugin")

final class MyPlugin: PostProcessor {
    func name() -> String {
        "my-plugin"
    }

    func version() -> String {
        "1.0.0"
    }

    func initialize() -> String {
        logger.info("Initializing plugin: my-plugin")
        return "{\"ok\": null}"
    }

    func shutdown() -> String {
        logger.info("Shutting down plugin: my-plugin")
        return "{\"ok\": null}"
    }

    func process(result: ExtractedDocument, config: ExtractionConfig) -> String {
        let contentLen = result.content.count
        logger.info("Processing \(result.mimeType) (\(contentLen) bytes)")

        if contentLen == 0 {
            logger.warning("Processing resulted in empty content")
        }

        return "{\"ok\": null}"
    }

    func shouldProcess(result: ExtractedDocument, config: ExtractionConfig) -> Bool {
        true
    }

    func processingStage() -> String {
        "early"
    }

    func priority() -> Int32 {
        50
    }

    func estimatedDurationMs(result: ExtractedDocument) -> UInt64 {
        10
    }
}

let plugin = MyPlugin()
try Xberg.registerPostProcessor(plugin)
```
