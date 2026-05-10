```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = dev.kreuzberg.Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)

    println(result.content())
    println("MIME type: ${result.mimeType()}")
    println("Tables: ${result.tables()?.size ?: 0}")
}
```
