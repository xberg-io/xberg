```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val pageConfig = PageConfig.builder()
        .withExtractPages(true)
        .build()

    val config = ExtractionConfig.builder()
        .withPages(Optional.of(pageConfig))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    val pages = result.pages() ?: return
    for (page in pages) {
        println("Page ${page.pageNumber()}:")
        println("  Content: ${page.content().length} chars")
        println("  Tables: ${page.tables().size}")
        println("  Images: ${page.images().size}")
    }
}
```
