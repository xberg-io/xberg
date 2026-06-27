```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = io.xberg.Xberg.extractSync(Paths.get("document.pdf"), null, config)

    result.tables()?.forEachIndexed { index, table ->
        println("Table ${index + 1}: ${table}")
    }

    result.chunks()?.forEachIndexed { index, chunk ->
        println("Chunk ${index + 1}: ${chunk}")
    }
}
```
