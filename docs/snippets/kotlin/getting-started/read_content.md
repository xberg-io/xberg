```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val config = ExtractionConfig.builder().build()
    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    result.tables()?.forEachIndexed { index, table ->
        println("Table ${index + 1}: ${table}")
    }

    result.chunks()?.forEachIndexed { index, chunk ->
        println("Chunk ${index + 1}: ${chunk}")
    }
}
```
