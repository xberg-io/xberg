```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = io.xberg.Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println(result.content())
    println("MIME type: ${result.mimeType()}")
}
```
