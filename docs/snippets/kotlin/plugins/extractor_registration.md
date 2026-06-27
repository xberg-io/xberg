```kotlin title="Kotlin"
import io.xberg.*
import io.xberg.kt.Xberg

// List registered document extractors and route work through the unified
// ExtractInput facade.
fun useRegisteredExtractors() {
    val extractors: List<String> = Xberg.listDocumentExtractors()
    println("Available extractors: $extractors")

    val config = ExtractionConfig.builder().build()
    val output: ExtractionResult = Xberg.extract(
        ExtractInput(
            kind = ExtractInputKind.URI,
            uri = "document.pdf",
        ),
        config,
    )
    val result: ExtractedDocument = output.results().first()
    println("Extracted ${result.content().length} characters via ${result.mimeType()}")
}
```
