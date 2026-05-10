```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg

// The Kotlin/Java plugin bridge does not expose an IDocumentExtractor interface
// — extractor registration lives in the Rust core. From Kotlin you can list
// the extractors that are already registered and route extraction through the
// existing facade.
fun useRegisteredExtractors() {
    val extractors: List<String> = Kreuzberg.listDocumentExtractors()
    println("Available extractors: $extractors")

    val config = ExtractionConfig.builder().build()
    val result: ExtractionResult = Kreuzberg.extractFileSync(
        java.nio.file.Path.of("document.pdf"),
        null,
        config,
    )
    println("Extracted ${result.content().length} characters via ${result.mimeType()}")
}
```
