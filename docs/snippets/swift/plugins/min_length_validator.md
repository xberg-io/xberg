```swift title="Swift"
import Xberg

final class MinLengthValidator: Validator {
    let minLength: Int

    init(minLength: Int = 100) {
        self.minLength = minLength
    }

    func name() -> String {
        "min_length_validator"
    }

    func version() -> String {
        "1.0.0"
    }

    func priority() -> Int32 {
        100
    }

    func validate(result: ExtractedDocument, config: ExtractionConfig) -> String {
        // Returns JSON-encoded Result<(), String>
        let contentLength = result.content.count
        if contentLength < minLength {
            let message = "Content too short: \(contentLength) < \(minLength)"
            return "{\"err\": \"\(message)\"}"
        }
        return "{\"ok\": null}"
    }

    func shouldValidate(result: ExtractedDocument, config: ExtractionConfig) -> Bool {
        true
    }

    func initialize() -> String {
        "{\"ok\": null}"
    }

    func shutdown() -> String {
        "{\"ok\": null}"
    }
}

let validator = MinLengthValidator(minLength: 100)
try Xberg.registerValidator(validator)
```
