```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val embedding = EmbeddingConfig.builder()
        .withModel(EmbeddingModelType.Preset("balanced"))
        .withBatchSize(16L)
        .withNormalize(true)
        .withShowDownloadProgress(true)
        .build()

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(1000L)
        .withOverlap(200L)
        .withEmbedding(Optional.of(embedding))
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Chunks with embeddings: ${result.chunks()?.size ?: 0}")
}
```
