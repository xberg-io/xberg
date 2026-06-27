```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths

fun main() {
    println("Hello from Xberg!")
    val config = ExtractionConfig.builder().build()
    val result = io.xberg.Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println(result.content())
}
```
