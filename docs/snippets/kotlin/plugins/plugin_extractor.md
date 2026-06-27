```kotlin title="Kotlin"
import io.xberg.*
import io.xberg.kt.Xberg
import java.nio.file.Files
import java.nio.file.Path

// The Kotlin/Java bindings expose plugin bridges for IPostProcessor,
// IValidator, IOcrBackend, and IEmbeddingBackend. There is no
// IDocumentExtractor bridge — extractor selection happens entirely in the
// Rust core based on MIME type. From Kotlin, the "extractor plugin" pattern
// is to wrap Xberg.extract / extract and dispatch to the right
// extractor by MIME.
class GenericExtractorClient {
    suspend fun extract(
        content: ByteArray,
        mimeType: String,
        config: ExtractionConfig = ExtractionConfig.builder().build(),
    ): ExtractionResult = Xberg.extract(content, mimeType, config)

    suspend fun extract(
        path: Path,
        mimeType: String? = null,
        config: ExtractionConfig = ExtractionConfig.builder().build(),
    ): ExtractionResult = Xberg.extract(path, mimeType, config)
}

suspend fun extractCustomPayload() {
    val client = GenericExtractorClient()
    val bytes = Files.readAllBytes(Path.of("payload.json"))
    val result = client.extract(bytes, mimeType = "application/json")
    println("Extracted ${result.content().length} chars")
}
```
