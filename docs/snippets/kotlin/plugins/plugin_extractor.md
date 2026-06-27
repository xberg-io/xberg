```kotlin title="Kotlin"
import io.xberg.*
import io.xberg.kt.Xberg
import java.nio.file.Files
import java.nio.file.Path

// Wrap Xberg.extract behind a small client when you want app-level dispatch.
class GenericExtractorClient {
    suspend fun extract(
        content: ByteArray,
        mimeType: String,
        config: ExtractionConfig = ExtractionConfig.builder().build(),
    ): ExtractedDocument = Xberg.extract(
        ExtractInput(
            kind = ExtractInputKind.BYTES,
            bytes = content,
            mimeType = mimeType,
        ),
        config,
    ).results().first()

    suspend fun extract(
        path: Path,
        mimeType: String? = null,
        config: ExtractionConfig = ExtractionConfig.builder().build(),
    ): ExtractedDocument = Xberg.extract(
        ExtractInput(
            kind = ExtractInputKind.URI,
            uri = path.toString(),
            mimeType = mimeType,
        ),
        config,
    ).results().first()
}

suspend fun extractCustomPayload() {
    val client = GenericExtractorClient()
    val bytes = Files.readAllBytes(Path.of("payload.json"))
    val result = client.extract(bytes, mimeType = "application/json")
    println("Extracted ${result.content().length} chars")
}
```
