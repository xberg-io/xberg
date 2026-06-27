```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(1000L)
        .withOverlap(200L)
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Chunks: ${result.chunks()?.size ?: 0}")
}
```

```kotlin title="Kotlin - Semantic"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val chunking = ChunkingConfig.builder()
        .withChunkerType(ChunkerType.Semantic)
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Chunks: ${result.chunks()?.size ?: 0}")
}
```

```kotlin title="Kotlin - Prepend Heading Context"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(500L)
        .withOverlap(50L)
        .withChunkerType(ChunkerType.Markdown)
        .withPrependHeadingContext(true)
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val result = Xberg.extractSync(Paths.get("document.md"), null, config)
    println("Chunks: ${result.chunks()?.size ?: 0}")
}
```
