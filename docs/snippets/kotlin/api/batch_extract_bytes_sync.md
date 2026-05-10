```kotlin title="Kotlin"
import dev.kreuzberg.*

fun main() {
    val config = ExtractionConfig.builder().build()
    val items = listOf(
        BatchBytesItem("Hello, world!".toByteArray(), "text/plain", null),
        BatchBytesItem("# Heading\n\nParagraph text.".toByteArray(), "text/markdown", null),
    )
    val results = Kreuzberg.batchExtractBytesSync(items, config)

    results.forEachIndexed { index, result ->
        println("Item $index: ${result.content().length} chars")
    }
}
```
