```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)

    val tables = result.tables() ?: emptyList()
    for (table in tables) {
        println("Table on page ${table.pageNumber()} with ${table.cells().size} rows")
        println(table.markdown())

        for (row in table.cells()) {
            println(row)
        }
    }
}
```
