```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val config = ExtractionConfig.builder()
        .withIncludeDocumentStructure(true)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    val document = result.document()
    if (document != null) {
        for (node in document.nodes()) {
            println(node)
        }
    }
}
```
