```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = dev.kreuzberg.Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)

    result.tables()?.forEachIndexed { index, table ->
        println("Table ${index + 1}: ${table}")
    }

    result.chunks()?.forEachIndexed { index, chunk ->
        println("Chunk ${index + 1}: ${chunk}")
    }
}
```
