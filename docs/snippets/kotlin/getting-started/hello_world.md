```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    println("Hello from Xberg!")
    val config = ExtractionConfig.builder().build()
    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content())
}
```
