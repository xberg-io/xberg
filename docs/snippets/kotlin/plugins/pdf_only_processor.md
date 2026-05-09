```kotlin title="Kotlin"
import dev.kreuzberg.*

class PdfOnlyProcessor : IPostProcessor {
    override fun name(): String = "pdf-only-processor"
    override fun version(): String = "1.0.0"

    override fun process(result: ExtractionResult, config: ExtractionConfig) {
        // Guard inside process() in addition to should_process() — the gate
        // saves the JSON roundtrip when this returns false.
        if (result.mimeType() != "application/pdf") return
        println("[pdf-only] processing PDF (${result.content().length} chars)")
    }

    override fun processing_stage(): ProcessingStage = ProcessingStage.Middle

    override fun should_process(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = _result.mimeType() == "application/pdf"

    override fun estimated_duration_ms(_result: ExtractionResult): Long = 5L

    override fun priority(): Int = 50
}

fun registerPdfOnlyProcessor() {
    PostProcessorBridge.registerPostProcessor(PdfOnlyProcessor())
}
```
