```swift title="Swift"
import Xberg
import os.lock

final class StatefulPlugin: PostProcessor {
    private var lock = NSLock()
    private var callCount: Int = 0
    private var cache: [String: String] = [:]

    func name() -> String {
        "stateful-plugin"
    }

    func version() -> String {
        "1.0.0"
    }

    func processingStage() -> String {
        "middle"
    }

    func priority() -> Int32 {
        50
    }

    func process(result: ExtractedDocument, config: ExtractionConfig) -> String {
        lock.lock()
        defer { lock.unlock() }

        callCount += 1
        cache["last_mime"] = result.mimeType
        cache["call_count"] = String(callCount)

        return "{\"ok\": null}"
    }

    func shouldProcess(result: ExtractedDocument, config: ExtractionConfig) -> Bool {
        true
    }

    func estimatedDurationMs(result: ExtractedDocument) -> UInt64 {
        1  // Minimal overhead
    }

    func initialize() -> String {
        lock.lock()
        defer { lock.unlock() }
        callCount = 0
        cache.removeAll()
        return "{\"ok\": null}"
    }

    func shutdown() -> String {
        lock.lock()
        defer { lock.unlock() }
        let finalCount = callCount
        cache.removeAll()
        let message = "Processed \(finalCount) extractions"
        print(message)
        return "{\"ok\": null}"
    }
}

let plugin = StatefulPlugin()
try Xberg.registerPostProcessor(plugin)
```
