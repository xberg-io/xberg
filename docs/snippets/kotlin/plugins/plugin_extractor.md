```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg
import java.nio.file.Files
import java.nio.file.Path

// The Kotlin/Java bindings expose plugin bridges for IPostProcessor,
// IValidator, IOcrBackend, and IEmbeddingBackend. There is no
// IDocumentExtractor bridge — extractor selection happens entirely in the
// Rust core based on MIME type. From Kotlin, the "extractor plugin" pattern
// is to wrap Kreuzberg.extractBytes / extractFile and dispatch to the right
// extractor by MIME.
class GenericExtractorClient {
    suspend fun extractBytes(
        content: ByteArray,
        mimeType: String,
        config: ExtractionConfig = ExtractionConfig.builder().build(),
    ): ExtractionResult = Kreuzberg.extractBytes(content, mimeType, config)

    suspend fun extractFile(
        path: Path,
        mimeType: String? = null,
        config: ExtractionConfig = ExtractionConfig.builder().build(),
    ): ExtractionResult = Kreuzberg.extractFile(path, mimeType, config)
}

suspend fun extractCustomPayload() {
    val client = GenericExtractorClient()
    val bytes = Files.readAllBytes(Path.of("payload.json"))
    val result = client.extractBytes(bytes, mimeType = "application/json")
    println("Extracted ${result.content().length} chars")
}
```
