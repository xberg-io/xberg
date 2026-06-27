```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val config = ExtractionConfig.builder().build()
    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content())
    println("MIME type: ${result.mimeType()}")
}
```
