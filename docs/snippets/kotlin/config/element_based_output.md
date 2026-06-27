```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    // Configure element-based output (resultFormat controls Unified vs ElementBased)
    val config = ExtractionConfig.builder()
        .withResultFormat(ResultFormat.ElementBased)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    val elements = result.elements().orEmpty()
    for (element in elements) {
        println("Type: ${element.elementType()}")
        val text = element.text()
        println("Text: ${text.take(100)}")

        element.metadata().pageNumber()?.let { page ->
            println("Page: $page")
        }
        println("---")
    }

    // Filter by element type
    val titles = elements.filter { it.elementType() == ElementType.Title }
    for (title in titles) {
        println("Title: ${title.text()}")
    }
}
```
