```kotlin title="Kotlin"
import dev.kreuzberg.*

class QualityScoreValidator(private val threshold: Double = 0.5) : IValidator {
    override fun name(): String = "quality-score-validator"
    override fun version(): String = "1.0.0"

    override fun validate(result: ExtractionResult, config: ExtractionConfig) {
        val score = result.qualityScore() ?: 0.0
        if (score < threshold) {
            throw IllegalStateException(
                "Quality score too low: %.2f < %.2f".format(score, threshold),
            )
        }
    }

    override fun should_validate(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = _result.qualityScore() != null

    override fun priority(): Int = 50
}

fun registerQualityScoreValidator() {
    ValidatorBridge.registerValidator(QualityScoreValidator(threshold = 0.5))
}
```
