```swift title="Swift"
import Xberg

final class MinLengthValidator: Validator {
    func name() -> String {
        "min_length"
    }

    func version() -> String {
        "1.0.0"
    }

    func priority() -> Int32 {
        50
    }

    func validate(result: ExtractionResult, config: ExtractionConfig) -> String {
        let contentLength = result.content().count
        if contentLength < 50 {
            let message = "Content too short: \(contentLength)"
            return "{\"err\": \"\(message)\"}"
        }
        return "{\"ok\": null}"
    }

    func shouldValidate(result: ExtractionResult, config: ExtractionConfig) -> Bool {
        true
    }

    func initialize() -> String {
        "{\"ok\": null}"
    }

    func shutdown() -> String {
        "{\"ok\": null}"
    }
}

let validator = MinLengthValidator()
try Xberg.registerValidator(validator)

// Extract a file; the validator runs in-pipeline during extraction
let config = ExtractionConfig(
    useCache: false,
    enableQualityProcessing: false,
    resultFormat: .unified,
    outputFormat: .markdown
)
let result = try extractSync(
    path: "document.pdf",
    mimeType: nil,
    config: config
)
print("Content length: \(result.content().count)")
```
