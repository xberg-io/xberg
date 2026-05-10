```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg
import kotlinx.coroutines.runBlocking
import java.nio.file.Paths

fun main() = runBlocking {
    val config = ExtractionConfig.builder().build()
    val result = Kreuzberg.extractFile(Paths.get("document.pdf"), null, config)

    println(result.content())
    println("MIME type: ${result.mimeType()}")
    println("Tables: ${result.tables()?.size ?: 0}")
}
```
