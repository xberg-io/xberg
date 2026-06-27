```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(500L)
        .withOverlap(50L)
        .build()

    val pages = PageConfig.builder()
        .withExtractPages(true)
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .withPages(Optional.of(pages))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    for (chunk in result.chunks().orEmpty()) {
        val first = chunk.metadata().firstPage()
        val last = chunk.metadata().lastPage()
        if (first != null && last != null) {
            val pageRange = if (first == last) "Page $first" else "Pages $first-$last"
            val preview = chunk.content().take(50)
            println("Chunk: $preview... ($pageRange)")
        }
    }
}
```
