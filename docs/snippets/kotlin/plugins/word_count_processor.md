```kotlin title="Kotlin"
import dev.kreuzberg.*

class WordCountProcessor : IPostProcessor {
    override fun name(): String = "word-count"
    override fun version(): String = "1.0.0"

    override fun process(result: ExtractionResult, config: ExtractionConfig) {
        val wordCount = result.content().split(Regex("\\s+")).count { it.isNotEmpty() }
        // ExtractionResult is an immutable record on the Java side; observe
        // and report rather than mutate.
        println("[word-count] ${result.mimeType()} -> $wordCount words")
    }

    override fun processing_stage(): ProcessingStage = ProcessingStage.Early

    override fun should_process(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = _result.content().isNotEmpty()

    override fun estimated_duration_ms(_result: ExtractionResult): Long = 1L

    override fun priority(): Int = 50
}

fun registerWordCountProcessor() {
    PostProcessorBridge.registerPostProcessor(WordCountProcessor())
}
```
