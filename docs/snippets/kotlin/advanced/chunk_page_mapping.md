```kotlin title="Kotlin"
import io.xberg.*
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

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
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
