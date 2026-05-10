```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths

fun main() {
    println("Hello from Kreuzberg!")
    val config = ExtractionConfig.builder().build()
    val result = dev.kreuzberg.Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)
    println(result.content())
}
```
