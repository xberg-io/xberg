```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val config = ExtractionConfig.builder().build()
    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

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
