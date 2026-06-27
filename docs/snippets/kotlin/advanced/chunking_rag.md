```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val embedding = EmbeddingConfig.builder()
        .withModel(EmbeddingModelType.Preset("balanced"))
        .withNormalize(true)
        .build()

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(500L)
        .withOverlap(50L)
        .withEmbedding(Optional.of(embedding))
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val result = Xberg.extractSync(Paths.get("research_paper.pdf"), null, config)
    for (chunk in result.chunks().orEmpty()) {
        val metadata = chunk.metadata()
        println("Chunk ${metadata.chunkIndex() + 1}/${metadata.totalChunks()}")
        println("Position: ${metadata.byteStart()}-${metadata.byteEnd()}")
        val text = chunk.content()
        val preview = text.take(100)
        println("Content: $preview...")
        chunk.embedding()?.let { embedding ->
            println("Embedding: ${embedding.size} dimensions")
        }
    }
}
```
