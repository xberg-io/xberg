```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Files
import java.nio.file.Paths

fun main() {
    val content = Files.readAllBytes(Paths.get("document.pdf"))
    val config = ExtractionConfig.builder().build()
    val result = Kreuzberg.extractBytesSync(content, "application/pdf", config)

    println(result.content())
    println("Tables: ${result.tables()?.size ?: 0}")
}
```
