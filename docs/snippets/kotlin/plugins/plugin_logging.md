```kotlin title="Kotlin"
import io.xberg.*
import java.util.logging.Logger

class LoggingPostProcessor : IPostProcessor {
    private val log: Logger = Logger.getLogger(LoggingPostProcessor::class.java.name)

    override fun name(): String = "logging-post-processor"
    override fun version(): String = "1.0.0"

    override fun initialize() {
        log.info("Initializing plugin: ${name()}")
    }

    override fun shutdown() {
        log.info("Shutting down plugin: ${name()}")
    }

    override fun process(result: ExtractedDocument, config: ExtractionConfig) {
        log.info("Processing ${result.mimeType()} (${result.content().length} chars)")
        if (result.content().isEmpty()) {
            log.warning("Extraction resulted in empty content")
        }
    }

    override fun processing_stage(): ProcessingStage = ProcessingStage.Late

    override fun should_process(
        _result: ExtractedDocument,
        _config: ExtractionConfig,
    ): Boolean = true

    override fun estimated_duration_ms(_result: ExtractedDocument): Long = 1L

    override fun priority(): Int = 10
}

fun registerLoggingPostProcessor() {
    PostProcessorBridge.registerPostProcessor(LoggingPostProcessor())
}
```
