```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val config = ExtractionConfig.builder().build()
    try {
        val resultOutput = Xberg.extract(
            ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
            config,
        )
        val result = resultOutput.results().first()
        println(result.content())
    } catch (e: XbergRsException) {
        System.err.println("Extraction failed: ${e.message}")
        System.err.println("Error code: ${e.code}")
    } catch (e: Exception) {
        System.err.println("Unexpected error: ${e.message}")
    }
}
```
