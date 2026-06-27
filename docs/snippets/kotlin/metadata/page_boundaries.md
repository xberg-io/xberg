```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val config = ExtractionConfig.builder().build()
    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    val pages = result.metadata().pages() ?: return
    val boundaries = pages.boundaries() ?: return

    val content = result.content()
    for (boundary in boundaries.take(3)) {
        val start = boundary.byteStart().toInt()
        val end = boundary.byteEnd().toInt()
        val pageText = content.substring(start, end)
        val previewEnd = minOf(100, pageText.length)

        println("Page ${boundary.pageNumber()}:")
        println("  Byte range: $start-$end")
        println("  Preview: ${pageText.substring(0, previewEnd)}...")
    }
}
```
