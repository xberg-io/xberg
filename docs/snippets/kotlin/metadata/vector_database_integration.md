```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

data class VectorRecord(
    val id: String,
    val content: String,
    val embedding: List<Float>,
    val metadata: Map<String, String>,
)

fun extractAndVectorize(documentPath: String, documentId: String): List<VectorRecord> {
    val embedding = EmbeddingConfig.builder()
        .withModel(EmbeddingModelType.Preset("balanced"))
        .withNormalize(true)
        .withBatchSize(32L)
        .build()

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(512L)
        .withOverlap(50L)
        .withEmbedding(Optional.of(embedding))
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val result = Xberg.extractSync(Paths.get(documentPath), null, config)

    val records = mutableListOf<VectorRecord>()
    val chunks = result.chunks() ?: return records
    for ((index, chunk) in chunks.withIndex()) {
        val vector = chunk.embedding() ?: continue
        val metadata = mapOf(
            "document_id" to documentId,
            "chunk_index" to index.toString(),
            "content_length" to chunk.content().length.toString(),
        )
        records.add(
            VectorRecord(
                id = "${documentId}_chunk_$index",
                content = chunk.content(),
                embedding = vector,
                metadata = metadata,
            )
        )
    }
    return records
}

fun main() {
    val records = extractAndVectorize("document.pdf", "doc-001")
    println("Generated ${records.size} vector records")
}
```
