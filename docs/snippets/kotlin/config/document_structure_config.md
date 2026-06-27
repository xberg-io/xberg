```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val config = ExtractionConfig.builder()
        .withIncludeDocumentStructure(true)
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    val document = result.document()
    if (document != null) {
        for (node in document.nodes()) {
            println(node)
        }
    }
}
```
