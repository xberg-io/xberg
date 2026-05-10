```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg
import kotlinx.coroutines.runBlocking
import java.nio.file.Files
import java.nio.file.Paths

fun main() = runBlocking {
    val content = Files.readAllBytes(Paths.get("document.pdf"))
    val config = ExtractionConfig.builder().build()
    val result = Kreuzberg.extractBytes(content, "application/pdf", config)

    println(result.content())
    println("Tables: ${result.tables()?.size ?: 0}")
}
```
