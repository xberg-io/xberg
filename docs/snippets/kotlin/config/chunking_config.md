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
    val chunks = result.chunks().orEmpty()
    println("Chunks: ${chunks.size}")
    for (chunk in chunks) {
        println("Length: ${chunk.content().length}")
    }
}
```

```kotlin title="Kotlin - Markdown with Heading Context"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val sizing = ChunkSizing.Tokenizer("Xenova/gpt-4o", Optional.empty())

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(500L)
        .withOverlap(50L)
        .withChunkerType(ChunkerType.Markdown)
        .withSizing(sizing)
        .build()

    val config = ExtractionConfig.builder()
        .withChunking(Optional.of(chunking))
        .build()

    val result = Xberg.extractSync(Paths.get("document.md"), null, config)
    for (chunk in result.chunks().orEmpty()) {
        chunk.metadata()?.headingContext()?.let { ctx ->
            for (heading in ctx.headings()) {
                println("Heading L${heading.level()}: ${heading.text()}")
            }
        }
        val text = chunk.content()
        println("Content: ${text.take(100)}...")
    }
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
    for (chunk in result.chunks().orEmpty()) {
        // Each chunk's content is prefixed with its heading breadcrumb
        val text = chunk.content()
        println("Content: ${text.take(100)}...")
    }
}
```
