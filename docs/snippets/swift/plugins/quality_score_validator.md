```swift title="Swift"
import Xberg

final class QualityValidator: Validator {
    let threshold: Double = 0.5

    func name() -> String {
        "quality-validator"
    }

    func version() -> String {
        "1.0.0"
    }

    func priority() -> Int32 {
        75
    }

    func validate(result: ExtractedDocument, config: ExtractionConfig) -> String {
        // Parse metadata to extract quality score
        let metadata = result.metadata
        let qualityScore: Double

        if let scoreStr = metadata["quality_score"] as? String,
           let score = Double(scoreStr) {
            qualityScore = score
        } else {
            qualityScore = 0.0
        }

        if qualityScore < threshold {
            let message = "Quality score too low: \(String(format: "%.2f", qualityScore))"
            return "{\"err\": \"\(message)\"}"
        }
        return "{\"ok\": null}"
    }

    func shouldValidate(result: ExtractedDocument, config: ExtractionConfig) -> Bool {
        // Only validate if quality processing was enabled
        config.enableQualityProcessing()
    }

    func initialize() -> String {
        "{\"ok\": null}"
    }

    func shutdown() -> String {
        "{\"ok\": null}"
    }
}

let validator = QualityValidator()
try Xberg.registerValidator(validator)
```
