```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.util.concurrent.atomic.AtomicInteger

class PdfMetadataExtractor : IPostProcessor {
    private val processed = AtomicInteger(0)

    override fun name(): String = "pdf-metadata-extractor"
    override fun version(): String = "1.0.0"

    override fun process(result: ExtractionResult, config: ExtractionConfig) {
        if (result.mimeType() != "application/pdf") return

        val count = processed.incrementAndGet()
        val metadata: Metadata = result.metadata()
        // Metadata is an immutable record — read PDF metadata fields rather
        // than mutate. Reporting via stdout/log keeps the snippet honest.
        println(
            "[pdf-metadata] #$count title=${metadata.title()} authors=${metadata.authors()}",
        )
    }

    override fun processing_stage(): ProcessingStage = ProcessingStage.Late

    override fun should_process(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = _result.mimeType() == "application/pdf"

    override fun estimated_duration_ms(_result: ExtractionResult): Long = 2L

    override fun priority(): Int = 25
}

fun registerPdfMetadataExtractor() {
    PostProcessorBridge.registerPostProcessor(PdfMetadataExtractor())
}
```
