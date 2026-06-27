```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)

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
