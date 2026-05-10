```kotlin title="Kotlin"
import dev.kreuzberg.*

class MinLengthValidator(private val minLength: Int) : IValidator {
    override fun name(): String = "min-length-validator"
    override fun version(): String = "1.0.0"

    override fun validate(result: ExtractionResult, config: ExtractionConfig) {
        val length = result.content().length
        if (length < minLength) {
            throw IllegalStateException(
                "Content too short: $length < $minLength characters",
            )
        }
    }

    override fun should_validate(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = true

    override fun priority(): Int = 100
}

fun registerMinLengthValidator() {
    ValidatorBridge.registerValidator(MinLengthValidator(minLength = 100))
}
```
