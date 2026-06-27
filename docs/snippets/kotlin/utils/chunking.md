```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(1500L)
        .withOverlap(200L)
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Chunks: ${result.chunks()?.size ?: 0}")
}
```
