```swift title="Swift"
import Xberg
import Testing

// Unit test a Swift Validator implementation
final class MinLengthValidator: Validator {
    let minLength: Int

    init(minLength: Int = 100) {
        self.minLength = minLength
    }

    func name() -> String { "test-validator" }
    func version() -> String { "1.0.0" }
    func priority() -> Int32 { 50 }
    func initialize() -> String { "{\"ok\": null}" }
    func shutdown() -> String { "{\"ok\": null}" }

    func validate(result: ExtractionResult, config: ExtractionConfig) -> String {
        let contentLength = result.content().count
        if contentLength < minLength {
            return "{\"err\": \"Content too short: \(contentLength) < \(minLength)\"}"
        }
        return "{\"ok\": null}"
    }

    func shouldValidate(result: ExtractionResult, config: ExtractionConfig) -> Bool {
        true
    }
}

// Unit test the validator by directly testing its logic.
// Integration tests exercise validators in-pipeline during extraction.

let validator = MinLengthValidator(minLength: 100)

// Create extraction config and result via the binding
let configJson = "{\"use_cache\": false}"
let config = try extractionConfigFromJson(configJson)

// Extract a document; the validator runs automatically during extraction
let result = try extract(path: "test.txt", mimeType: "text/plain", config: config)

// The validator's validate() method is invoked in-pipeline.
// If it rejects, the extraction throws an error.
```
