```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    try {
        val result = Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)
        println(result.content())
    } catch (e: KreuzbergRsException) {
        System.err.println("Extraction failed: ${e.message}")
        System.err.println("Error code: ${e.code}")
    } catch (e: Exception) {
        System.err.println("Unexpected error: ${e.message}")
    }
}
```
