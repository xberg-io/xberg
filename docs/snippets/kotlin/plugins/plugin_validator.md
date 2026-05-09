```kotlin title="Kotlin"
import dev.kreuzberg.*

// Generic validator pattern: every IValidator has the same shape.
// `name()` keys the registry, `priority()` orders execution (higher = earlier),
// `should_validate()` is a fast skip-check, and `validate()` throws on failure.
class GenericValidator(
    private val pluginName: String,
    private val pluginPriority: Int,
    private val check: (ExtractionResult, ExtractionConfig) -> Unit,
) : IValidator {
    override fun name(): String = pluginName
    override fun version(): String = "1.0.0"

    override fun initialize() {
        // Optional: open resources, load config files, etc.
    }

    override fun shutdown() {
        // Optional: release resources held in initialize().
    }

    override fun validate(result: ExtractionResult, config: ExtractionConfig) {
        check(result, config)
    }

    override fun should_validate(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = true

    override fun priority(): Int = pluginPriority
}

fun registerGenericValidator() {
    val validator = GenericValidator(
        pluginName = "non-empty-content",
        pluginPriority = 200,
    ) { result, _ ->
        require(result.content().isNotBlank()) { "Extracted content is blank" }
    }
    ValidatorBridge.registerValidator(validator)
}
```
